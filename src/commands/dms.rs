use crate::client::Client;
use crate::commands::channels::{enrich_messages, MessageInfo};
use crate::error::Result;
use crate::output::{HumanReadable, Output};
use crate::timeparse::parse_time_expr;
use chrono::DateTime;
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
pub struct SentDm {
    pub channel: String,
    pub ts: String,
    pub user: String,
    pub text: String,
}

impl HumanReadable for SentDm {
    fn print_human(&self) {
        println!("{} to {}", "DM sent".green(), self.user);
        println!("  channel: {}", self.channel.dimmed());
        println!("  ts: {}", self.ts.dimmed());
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
    after: Option<&str>,
    before: Option<&str>,
    enrich: bool,
) -> Result<()> {
    let session = client.session();
    let channel_id = client.resolve_channel(dm_channel).await?;

    let mut request = SlackApiConversationsHistoryRequest::new()
        .with_channel(channel_id)
        .with_limit(limit.unwrap_or(20));

    if let Some(after) = after {
        let ts =
            parse_time_expr(after).map_err(crate::error::SlackCliError::Api)?;
        request = request.with_oldest(SlackTs::new(ts));
    }
    if let Some(before) = before {
        let ts =
            parse_time_expr(before).map_err(crate::error::SlackCliError::Api)?;
        request = request.with_latest(SlackTs::new(ts));
    }

    let response = session.conversations_history(&request).await?;

    let mut messages: Vec<MessageInfo> = response
        .messages
        .into_iter()
        .map(|m| {
            let ts_float: f64 = m.origin.ts.0.parse().unwrap_or(0.0);
            let timestamp = DateTime::from_timestamp(ts_float as i64, 0);

            MessageInfo {
                ts: m.origin.ts.0,
                user: m.sender.user.map(|u| u.0),
                text: m.content.text.unwrap_or_default(),
                timestamp,
                thread_ts: m.origin.thread_ts.map(|t| t.0),
                reply_count: m.parent.reply_count.map(|c| c as u64),
                latest_reply: m.parent.latest_reply.map(|t| t.0),
                username: None,
                real_name: None,
            }
        })
        .collect();

    if enrich {
        enrich_messages(client, &mut messages).await?;
    }

    output.print_list(&messages, &format!("DM history in {}", dm_channel));

    Ok(())
}

/// Send a DM to a user (opens conversation first)
pub async fn send(client: &Client, output: &Output, user: &str, text: &str) -> Result<()> {
    let session = client.session();
    let user_id = SlackUserId::new(user.to_string());

    // Open a DM conversation with the user
    let open_request = SlackApiConversationsOpenRequest::new().with_users(vec![user_id]);
    let open_response = session.conversations_open(&open_request).await?;
    let channel_id = open_response.channel.id;

    // Send the message
    let content = SlackMessageContent::new().with_text(text.to_string());
    let msg_request = SlackApiChatPostMessageRequest::new(channel_id.clone(), content);
    let msg_response = session.chat_post_message(&msg_request).await?;

    let sent = SentDm {
        channel: channel_id.0,
        ts: msg_response.ts.0,
        user: user.to_string(),
        text: text.to_string(),
    };

    output.print(&sent);
    output.success("DM sent");

    Ok(())
}
