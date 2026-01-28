use crate::client::Client;
use crate::error::{Result, SlackCliError};
use crate::output::{HumanReadable, Output};
use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct FileInfoResponse {
    ok: bool,
    error: Option<String>,
    file: Option<SlackFile>,
}

#[derive(Debug, Deserialize)]
struct SlackFile {
    id: String,
    name: String,
    title: Option<String>,
    mimetype: String,
    filetype: String,
    size: u64,
    user: Option<String>,
    timestamp: Option<i64>,
    url_private: Option<String>,
    url_private_download: Option<String>,
    permalink: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub mimetype: String,
    pub filetype: String,
    pub size: u64,
    pub user: Option<String>,
    pub url_private: Option<String>,
    pub url_private_download: Option<String>,
    pub permalink: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
}

impl HumanReadable for FileInfo {
    fn print_human(&self) {
        let time = self
            .timestamp
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let user = self.user.as_deref().unwrap_or("unknown");
        let title = self.title.as_deref().unwrap_or(&self.name);

        println!("{} by {}", title.green().bold(), user.cyan());
        println!("  {} | {} | {} bytes", self.filetype.yellow(), self.mimetype.dimmed(), self.size);
        println!("  Uploaded: {}", time.dimmed());

        if let Some(url) = &self.url_private_download {
            println!("  Download: {}", url.blue());
        } else if let Some(url) = &self.url_private {
            println!("  URL: {}", url.blue());
        }

        if let Some(permalink) = &self.permalink {
            println!("  Permalink: {}", permalink.dimmed());
        }
        println!();
    }
}

/// Get file info by ID
pub async fn info(client: &Client, output: &Output, file_id: &str) -> Result<()> {
    let token = client.token();

    let url = format!(
        "https://slack.com/api/files.info?file={}",
        urlencoding::encode(file_id)
    );

    let http_client = reqwest::Client::new();
    let response = http_client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| SlackCliError::Api(e.to_string()))?;

    let file_response: FileInfoResponse = response
        .json()
        .await
        .map_err(|e| SlackCliError::Api(e.to_string()))?;

    if !file_response.ok {
        return Err(SlackCliError::Api(
            file_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string()),
        ));
    }

    let file = file_response
        .file
        .ok_or_else(|| SlackCliError::Api("No file in response".to_string()))?;

    let timestamp = file.timestamp.and_then(|ts| DateTime::from_timestamp(ts, 0));

    let info = FileInfo {
        id: file.id,
        name: file.name,
        title: file.title,
        mimetype: file.mimetype,
        filetype: file.filetype,
        size: file.size,
        user: file.user,
        url_private: file.url_private,
        url_private_download: file.url_private_download,
        permalink: file.permalink,
        timestamp,
    };

    output.print(&info);

    Ok(())
}

/// Download a file to stdout or a path
pub async fn download(client: &Client, file_id: &str, output_path: Option<&str>) -> Result<()> {
    let token = client.token();

    // First get file info to get download URL
    let url = format!(
        "https://slack.com/api/files.info?file={}",
        urlencoding::encode(file_id)
    );

    let http_client = reqwest::Client::new();
    let response = http_client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| SlackCliError::Api(e.to_string()))?;

    let file_response: FileInfoResponse = response
        .json()
        .await
        .map_err(|e| SlackCliError::Api(e.to_string()))?;

    if !file_response.ok {
        return Err(SlackCliError::Api(
            file_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string()),
        ));
    }

    let file = file_response
        .file
        .ok_or_else(|| SlackCliError::Api("No file in response".to_string()))?;

    let download_url = file
        .url_private_download
        .or(file.url_private)
        .ok_or_else(|| SlackCliError::Api("No download URL available".to_string()))?;

    // Download the file
    let file_response = http_client
        .get(&download_url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| SlackCliError::Api(e.to_string()))?;

    let bytes = file_response
        .bytes()
        .await
        .map_err(|e| SlackCliError::Api(e.to_string()))?;

    match output_path {
        Some(path) => {
            std::fs::write(path, &bytes)?;
            eprintln!("Downloaded {} to {}", file.name, path);
        }
        None => {
            // Write to stdout
            use std::io::Write;
            std::io::stdout().write_all(&bytes)?;
        }
    }

    Ok(())
}
