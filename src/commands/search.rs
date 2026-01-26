use crate::client::Client;
use crate::error::{Result, SlackCliError};
use crate::output::{HumanReadable, Output};
use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct SearchResponse {
    ok: bool,
    error: Option<String>,
    messages: Option<SearchMessages>,
}

#[derive(Debug, Deserialize)]
struct SearchMessages {
    matches: Vec<SearchMatch>,
    total: u64,
}

#[derive(Debug, Deserialize)]
struct SearchMatch {
    ts: String,
    text: String,
    user: Option<String>,
    username: Option<String>,
    channel: SearchChannel,
    permalink: String,
}

#[derive(Debug, Deserialize)]
struct SearchChannel {
    id: String,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub ts: String,
    pub text: String,
    pub user: Option<String>,
    pub username: Option<String>,
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub permalink: String,
    pub timestamp: Option<DateTime<Utc>>,
}

impl HumanReadable for SearchResult {
    fn print_human(&self) {
        let time = self
            .timestamp
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| self.ts.clone());

        let user = self
            .username
            .as_deref()
            .or(self.user.as_deref())
            .unwrap_or("unknown");

        let channel = self.channel_name.as_deref().unwrap_or(&self.channel_id);

        println!("{} {} in #{}:", time.dimmed(), user.green(), channel.cyan());
        // Truncate long messages
        let text = if self.text.len() > 200 {
            format!("{}...", &self.text[..200])
        } else {
            self.text.clone()
        };
        println!("  {}", text);
        println!("  {}", self.permalink.dimmed());
        println!();
    }
}

/// Search messages using Slack search API
pub async fn messages(
    client: &Client,
    output: &Output,
    query: &str,
    limit: Option<u16>,
) -> Result<()> {
    let token = client.token();
    let count = limit.unwrap_or(20);

    // Build search URL
    let url = format!(
        "https://slack.com/api/search.messages?query={}&count={}&sort=timestamp&sort_dir=desc",
        urlencoding::encode(query),
        count
    );

    // Make HTTP request
    let http_client = reqwest::Client::new();
    let response = http_client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| SlackCliError::Api(e.to_string()))?;

    let search_response: SearchResponse = response
        .json()
        .await
        .map_err(|e| SlackCliError::Api(e.to_string()))?;

    if !search_response.ok {
        return Err(SlackCliError::Api(
            search_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string()),
        ));
    }

    let messages = search_response.messages.unwrap_or(SearchMessages {
        matches: vec![],
        total: 0,
    });

    let results: Vec<SearchResult> = messages
        .matches
        .into_iter()
        .map(|m| {
            let ts_float: f64 = m.ts.parse().unwrap_or(0.0);
            let timestamp = DateTime::from_timestamp(ts_float as i64, 0);

            SearchResult {
                ts: m.ts,
                text: m.text,
                user: m.user,
                username: m.username,
                channel_id: m.channel.id,
                channel_name: m.channel.name,
                permalink: m.permalink,
                timestamp,
            }
        })
        .collect();

    output.print_list(
        &results,
        &format!("Search results for '{}' ({} total)", query, messages.total),
    );

    Ok(())
}
