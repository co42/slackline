use crate::client::Client;
use crate::error::Result;
use crate::output::{HumanReadable, Output};
use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::Serialize;
use slack_morphism::prelude::*;

#[derive(Debug, Serialize)]
pub struct ReplyInfo {
    pub ts: String,
    pub user: Option<String>,
    pub text: String,
    pub timestamp: Option<DateTime<Utc>>,
}

impl HumanReadable for ReplyInfo {
    fn print_human(&self) {
        let time = self
            .timestamp
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| self.ts.clone());

        let user = self.user.as_deref().unwrap_or("unknown");

        println!("{} {}:", time.dimmed(), user.green());
        println!("  {}", self.text);
        println!();
    }
}

#[derive(Debug, Serialize)]
pub struct PermalinkInfo {
    pub channel: String,
    pub message_ts: String,
    pub permalink: String,
}

impl HumanReadable for PermalinkInfo {
    fn print_human(&self) {
        println!("{}", self.permalink.cyan());
    }
}

pub async fn replies(
    client: &Client,
    output: &Output,
    channel: &str,
    thread_ts: &str,
    limit: Option<u16>,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());
    let ts = SlackTs::new(thread_ts.to_string());

    let request =
        SlackApiConversationsRepliesRequest::new(channel_id, ts).with_limit(limit.unwrap_or(100));

    let response = session.conversations_replies(&request).await?;

    let replies: Vec<ReplyInfo> = response
        .messages
        .into_iter()
        .map(|m| {
            let ts_float: f64 = m.origin.ts.0.parse().unwrap_or(0.0);
            let timestamp = DateTime::from_timestamp(ts_float as i64, 0);

            ReplyInfo {
                ts: m.origin.ts.0,
                user: m.sender.user.map(|u| u.0),
                text: m.content.text.unwrap_or_default(),
                timestamp,
            }
        })
        .collect();

    output.print_list(&replies, &format!("Thread replies in {}", channel));

    Ok(())
}

pub async fn permalink(
    client: &Client,
    output: &Output,
    channel: &str,
    message_ts: &str,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());
    let ts = SlackTs::new(message_ts.to_string());

    let request = SlackApiChatGetPermalinkRequest::new(channel_id.clone(), ts.clone());
    let response = session.chat_get_permalink(&request).await?;

    let info = PermalinkInfo {
        channel: channel_id.0,
        message_ts: ts.0,
        permalink: response.permalink.to_string(),
    };

    output.print(&info);

    Ok(())
}
