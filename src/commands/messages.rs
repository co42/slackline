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

/// Reaction on a message
#[derive(Debug, Serialize)]
pub struct ReactionInfo {
    pub name: String,
    pub count: u64,
    pub users: Vec<String>,
}

impl HumanReadable for ReactionInfo {
    fn print_human(&self) {
        let users = self.users.join(", ");
        println!("  :{}: ({}) — {}", self.name, self.count, users.dimmed());
    }
}

/// Sent message echo
#[derive(Debug, Serialize)]
pub struct SentMessage {
    pub channel: String,
    pub ts: String,
    pub text: String,
}

impl HumanReadable for SentMessage {
    fn print_human(&self) {
        println!("{} in {}", "Message sent".green(), self.channel);
        println!("  ts: {}", self.ts.dimmed());
    }
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

/// Get reactions on a message
pub async fn reactions(
    client: &Client,
    output: &Output,
    channel: &str,
    ts: &str,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());
    let timestamp = SlackTs::new(ts.to_string());

    let request = SlackApiReactionsGetRequest::new()
        .with_channel(channel_id)
        .with_timestamp(timestamp);

    let response = session.reactions_get(&request).await?;

    let reactions = match response {
        SlackApiReactionsGetResponse::Message(msg) => msg
            .message
            .content
            .reactions
            .unwrap_or_default()
            .into_iter()
            .map(|r| ReactionInfo {
                name: r.name.0,
                count: r.count as u64,
                users: r.users.into_iter().map(|u| u.0).collect(),
            })
            .collect::<Vec<_>>(),
        SlackApiReactionsGetResponse::File(_) => vec![],
    };

    output.print_list(&reactions, &format!("Reactions on message {} in {}", ts, channel));

    Ok(())
}

/// Send a message to a channel
pub async fn send(
    client: &Client,
    output: &Output,
    channel: &str,
    text: &str,
    thread_ts: Option<&str>,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());
    let content = SlackMessageContent::new().with_text(text.to_string());

    let mut request = SlackApiChatPostMessageRequest::new(channel_id, content);
    if let Some(ts) = thread_ts {
        request = request.with_thread_ts(SlackTs::new(ts.to_string()));
    }

    let response = session.chat_post_message(&request).await?;

    let sent = SentMessage {
        channel: response.channel.0,
        ts: response.ts.0,
        text: text.to_string(),
    };

    output.print(&sent);
    output.success("Message sent");

    Ok(())
}

/// Add a reaction to a message
pub async fn react(
    client: &Client,
    output: &Output,
    channel: &str,
    ts: &str,
    emoji: &str,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());
    let timestamp = SlackTs::new(ts.to_string());
    let name = SlackReactionName::new(emoji.to_string());

    let request = SlackApiReactionsAddRequest::new(channel_id, name, timestamp);
    session.reactions_add(&request).await?;

    output.success(&format!("Added :{}:", emoji));

    Ok(())
}

/// Remove a reaction from a message
pub async fn unreact(
    client: &Client,
    output: &Output,
    channel: &str,
    ts: &str,
    emoji: &str,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());
    let timestamp = SlackTs::new(ts.to_string());
    let name = SlackReactionName::new(emoji.to_string());

    let request = SlackApiReactionsRemoveRequest::new(name)
        .with_channel(channel_id)
        .with_timestamp(timestamp);
    session.reactions_remove(&request).await?;

    output.success(&format!("Removed :{}:", emoji));

    Ok(())
}

/// Pin a message
pub async fn pin(
    client: &Client,
    output: &Output,
    channel: &str,
    ts: &str,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());
    let timestamp = SlackTs::new(ts.to_string());

    let request = SlackApiPinsAddRequest::new(channel_id, timestamp);
    session.pins_add(&request).await?;

    output.success("Message pinned");

    Ok(())
}

/// Unpin a message
pub async fn unpin(
    client: &Client,
    output: &Output,
    channel: &str,
    ts: &str,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());
    let timestamp = SlackTs::new(ts.to_string());

    let request = SlackApiPinsRemoveRequest::new(channel_id, timestamp);
    session.pins_remove(&request).await?;

    output.success("Message unpinned");

    Ok(())
}
