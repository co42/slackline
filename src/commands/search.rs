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
            format!("{}...", &self.text[..self.text.floor_char_boundary(200)])
        } else {
            self.text.clone()
        };
        println!("  {}", text);
        println!("  {}", self.permalink.dimmed());
        println!();
    }
}

/// Highest `count` that `search.messages` accepts. Above this Slack ignores the
/// value and returns its own default of 20, so the caller gets fewer results.
const MAX_COUNT: u16 = 100;

const DEFAULT_COUNT: u16 = 20;

/// Results per page to request. Values above `MAX_COUNT` come back as `MAX_COUNT`.
fn page_count(limit: Option<u16>) -> u16 {
    limit.unwrap_or(DEFAULT_COUNT).min(MAX_COUNT)
}

/// Search messages using Slack search API
pub async fn messages(
    client: &Client,
    output: &Output,
    query: &str,
    limit: Option<u16>,
    page: Option<u32>,
) -> Result<()> {
    let token = client.token();
    let count = page_count(limit);

    if limit.is_some_and(|l| l > MAX_COUNT) {
        output.status(&format!(
            "Slack returns at most {MAX_COUNT} results per page. Use --page to read further."
        ));
    }

    // Build search URL
    let mut url = format!(
        "https://slack.com/api/search.messages?query={}&count={}&sort=timestamp&sort_dir=desc",
        urlencoding::encode(query),
        count
    );

    if let Some(p) = page {
        url.push_str(&format!("&page={}", p));
    }

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

    let total = messages.total;

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

    let title = format!("Search results for '{}' ({} total)", query, total);

    let wrapper = serde_json::json!({
        "total": total,
        "results": results,
    });

    output.print_list_wrapped(&results, &title, &wrapper);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_count_defaults_to_20_when_no_limit_given() {
        assert_eq!(page_count(None), 20);
    }

    #[test]
    fn page_count_passes_through_values_slack_accepts() {
        assert_eq!(page_count(Some(1)), 1);
        assert_eq!(page_count(Some(50)), 50);
        assert_eq!(page_count(Some(MAX_COUNT)), MAX_COUNT);
    }

    /// Slack answers a `count` above 100 with its own default of 20, so an unclamped
    /// request returns fewer results than a smaller one. Clamping keeps the page full.
    #[test]
    fn page_count_clamps_above_the_slack_maximum() {
        assert_eq!(page_count(Some(101)), MAX_COUNT);
        assert_eq!(page_count(Some(200)), MAX_COUNT);
        assert_eq!(page_count(Some(u16::MAX)), MAX_COUNT);
    }
}
