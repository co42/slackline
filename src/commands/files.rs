use crate::client::Client;
use crate::error::{Result, SlackCliError};
use crate::output::{HumanReadable, Output};
use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::Serialize;
use slack_morphism::prelude::*;

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub mimetype: Option<String>,
    pub filetype: Option<String>,
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
        println!("  {}: {}", "ID".dimmed(), self.id);
        if let (Some(filetype), Some(mimetype)) = (&self.filetype, &self.mimetype) {
            println!("  {} | {}", filetype.yellow(), mimetype.dimmed());
        }
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

#[derive(Debug, Serialize)]
pub struct UploadedFile {
    pub id: String,
    pub name: String,
    pub size: u64,
}

impl HumanReadable for UploadedFile {
    fn print_human(&self) {
        println!("{} {}", "Uploaded".green(), self.name.bold());
        println!("  id: {}", self.id.dimmed());
        println!("  size: {} bytes", self.size);
    }
}

fn slack_file_to_info(file: SlackFile) -> FileInfo {
    let timestamp = file.timestamp.map(|t| t.0);

    FileInfo {
        id: file.id.0,
        name: file.name.unwrap_or_default(),
        title: file.title,
        mimetype: file.mimetype.map(|m| m.0),
        filetype: file.filetype.map(|f| f.0),
        user: file.user.map(|u| u.0),
        url_private: file.url_private.map(|u| u.to_string()),
        url_private_download: file.url_private_download.map(|u| u.to_string()),
        permalink: file.permalink.map(|u| u.to_string()),
        timestamp,
    }
}

/// Get file info by ID
pub async fn info(client: &Client, output: &Output, file_id: &str) -> Result<()> {
    let session = client.session();
    let request = SlackApiFilesInfoRequest::new(SlackFileId(file_id.to_string()));

    let response = session.files_info(&request).await?;
    let info = slack_file_to_info(response.file);

    output.print(&info);

    Ok(())
}

/// List files
pub async fn list(
    client: &Client,
    output: &Output,
    channel: Option<&str>,
    user: Option<&str>,
    limit: Option<u32>,
) -> Result<()> {
    let session = client.session();

    let mut request = SlackApiFilesListRequest::new();
    if let Some(ch) = channel {
        let channel_id = client.resolve_channel(ch).await?;
        request = request.with_channel(channel_id);
    }
    if let Some(u) = user {
        request = request.with_user(SlackUserId(u.to_string()));
    }
    if let Some(count) = limit {
        request = request.with_count(count);
    }

    let response = session.files_list(&request).await?;

    let files: Vec<FileInfo> = response.files.into_iter().map(slack_file_to_info).collect();

    let title = match (channel, user) {
        (Some(ch), Some(u)) => format!("Files in #{} by {}", ch, u),
        (Some(ch), None) => format!("Files in #{}", ch),
        (None, Some(u)) => format!("Files by {}", u),
        (None, None) => "Files".to_string(),
    };

    output.print_list(&files, &title);

    Ok(())
}

/// Download a file to stdout or a path
pub async fn download(client: &Client, file_id: &str, output_path: Option<&str>) -> Result<()> {
    let session = client.session();
    let request = SlackApiFilesInfoRequest::new(SlackFileId(file_id.to_string()));

    let response = session.files_info(&request).await?;
    let file = response.file;

    let download_url = file
        .url_private_download
        .or(file.url_private)
        .ok_or_else(|| SlackCliError::Api("No download URL available".to_string()))?;

    // Download the file using reqwest (slack-morphism doesn't have file download)
    let token = client.token();
    let http_client = reqwest::Client::new();
    let file_response = http_client
        .get(download_url.as_str())
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| SlackCliError::Api(e.to_string()))?;

    let bytes = file_response
        .bytes()
        .await
        .map_err(|e| SlackCliError::Api(e.to_string()))?;

    let filename = file.name.unwrap_or_else(|| "file".to_string());

    match output_path {
        Some(path) => {
            std::fs::write(path, &bytes)?;
            eprintln!("Downloaded {} to {}", filename, path);
        }
        None => {
            use std::io::Write;
            std::io::stdout().write_all(&bytes)?;
        }
    }

    Ok(())
}

/// Upload a file to Slack (3-step upload flow)
pub async fn upload(
    client: &Client,
    output: &Output,
    path: &str,
    channel: Option<&str>,
    thread_ts: Option<&str>,
    comment: Option<&str>,
) -> Result<()> {
    let session = client.session();

    // Read the file
    let file_bytes = std::fs::read(path)
        .map_err(|e| SlackCliError::Api(format!("Failed to read {}: {}", path, e)))?;
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    let file_size = file_bytes.len() as u64;

    // Detect content type
    let content_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    // Step 1: Get upload URL
    let url_request =
        SlackApiFilesGetUploadUrlExternalRequest::new(filename.clone(), file_size as usize);
    let url_response = session.get_upload_url_external(&url_request).await?;

    // Step 2: Upload file bytes to the URL
    let upload_request =
        SlackApiFilesUploadViaUrlRequest::new(url_response.upload_url, file_bytes, content_type);
    session.files_upload_via_url(&upload_request).await?;

    // Step 3: Complete the upload
    let file_complete = SlackApiFilesComplete::new(url_response.file_id.clone());
    let mut complete_request = SlackApiFilesCompleteUploadExternalRequest::new(vec![file_complete]);
    if let Some(ch) = channel {
        let channel_id = client.resolve_channel(ch).await?;
        complete_request = complete_request.with_channel_id(channel_id);
    }
    if let Some(ts) = thread_ts {
        complete_request = complete_request.with_thread_ts(SlackTs::new(ts.to_string()));
    }
    if let Some(c) = comment {
        complete_request = complete_request.with_initial_comment(c.to_string());
    }
    session
        .files_complete_upload_external(&complete_request)
        .await?;

    let uploaded = UploadedFile {
        id: url_response.file_id.0,
        name: filename,
        size: file_size,
    };

    output.print(&uploaded);
    output.success("File uploaded");

    Ok(())
}
