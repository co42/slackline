use crate::client::Client;
use crate::error::Result;
use crate::output::{HumanReadable, Output};
use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::Serialize;
use slack_morphism::prelude::*;

#[derive(Debug, Serialize)]
pub struct DmConversation {
    pub id: String,
    pub user_id: Option<String>,
    pub is_open: bool,
    pub priority: Option<f64>,
}

impl HumanReadable for DmConversation {
    fn print_human(&self) {
        let user = self.user_id.as_deref().unwrap_or("unknown");
        let status = if self.is_open { "" } else { " (closed)" };
        println!(
            "DM {} → user {}{}",
            self.id.dimmed(),
            user.green(),
            status.dimmed()
        );
    }
}

#[derive(Debug, Serialize)]
pub struct DmMessage {
    pub ts: String,
    pub user: Option<String>,
    pub text: String,
    pub timestamp: Option<DateTime<Utc>>,
}

impl HumanReadable for DmMessage {
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

/// List DM conversations
pub async fn list(client: &Client, output: &Output, limit: Option<u16>) -> Result<()> {
    let session = client.session();

    // Get DMs (im) and group DMs (mpim)
    let request = SlackApiUsersConversationsRequest::new()
        .with_types(vec![SlackConversationType::Im, SlackConversationType::Mpim])
        .with_exclude_archived(true)
        .with_limit(limit.unwrap_or(50));

    let response = session.users_conversations(&request).await?;

    let dms: Vec<DmConversation> = response
        .channels
        .into_iter()
        .map(|c| DmConversation {
            id: c.id.0,
            user_id: c.creator.map(|u| u.0),
            is_open: c.flags.is_im.unwrap_or(false) || c.flags.is_mpim.unwrap_or(false),
            priority: c.priority.map(|p| p.0),
        })
        .collect();

    output.print_list(&dms, "Direct Messages");

    Ok(())
}

/// Get DM history with a user (pass DM channel ID, not user ID)
pub async fn history(
    client: &Client,
    output: &Output,
    dm_channel: &str,
    limit: Option<u16>,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(dm_channel.to_string());

    let request = SlackApiConversationsHistoryRequest::new()
        .with_channel(channel_id)
        .with_limit(limit.unwrap_or(20));

    let response = session.conversations_history(&request).await?;

    let messages: Vec<DmMessage> = response
        .messages
        .into_iter()
        .map(|m| {
            let ts_float: f64 = m.origin.ts.0.parse().unwrap_or(0.0);
            let timestamp = DateTime::from_timestamp(ts_float as i64, 0);

            DmMessage {
                ts: m.origin.ts.0,
                user: m.sender.user.map(|u| u.0),
                text: m.content.text.unwrap_or_default(),
                timestamp,
            }
        })
        .collect();

    output.print_list(&messages, &format!("DM history in {}", dm_channel));

    Ok(())
}
