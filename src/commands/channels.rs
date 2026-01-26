use crate::client::Client;
use crate::error::Result;
use crate::output::{HumanReadable, Output};
use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::Serialize;
use slack_morphism::prelude::*;

#[derive(Debug, Serialize)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
    pub topic: Option<String>,
    pub purpose: Option<String>,
    pub num_members: Option<u64>,
    pub is_private: bool,
    pub is_archived: bool,
}

impl HumanReadable for ChannelInfo {
    fn print_human(&self) {
        let prefix = if self.is_private { "🔒" } else { "#" };
        let archived = if self.is_archived {
            " (archived)".dimmed().to_string()
        } else {
            String::new()
        };
        let members = self
            .num_members
            .map(|n| format!(" ({} members)", n))
            .unwrap_or_default();

        println!(
            "{}{}{}{}",
            prefix,
            self.name.bold(),
            members.dimmed(),
            archived
        );
        if let Some(topic) = &self.topic {
            if !topic.is_empty() {
                println!("  {}", topic.dimmed());
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MessageInfo {
    pub ts: String,
    pub user: Option<String>,
    pub text: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub thread_ts: Option<String>,
    pub reply_count: Option<u64>,
}

impl HumanReadable for MessageInfo {
    fn print_human(&self) {
        let time = self
            .timestamp
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| self.ts.clone());

        let user = self.user.as_deref().unwrap_or("unknown");
        let thread_info = self
            .reply_count
            .filter(|&c| c > 0)
            .map(|c| format!(" [{} replies]", c).cyan().to_string())
            .unwrap_or_default();

        println!("{} {}{}:", time.dimmed(), user.green(), thread_info);
        println!("  {}", self.text);
        println!();
    }
}

#[derive(Debug, Serialize)]
pub struct MemberInfo {
    pub id: String,
    pub name: Option<String>,
}

impl HumanReadable for MemberInfo {
    fn print_human(&self) {
        let display = self.name.as_deref().unwrap_or(&self.id);
        println!("  @{}", display);
    }
}

pub async fn list(client: &Client, output: &Output, limit: Option<u16>) -> Result<()> {
    let session = client.session();

    let request = SlackApiConversationsListRequest::new()
        .with_exclude_archived(true)
        .with_limit(limit.unwrap_or(100));

    let response = session.conversations_list(&request).await?;

    let channels: Vec<ChannelInfo> = response
        .channels
        .into_iter()
        .map(|c| ChannelInfo {
            id: c.id.0,
            name: c.name.unwrap_or_default(),
            topic: c.topic.map(|t| t.value),
            purpose: c.purpose.map(|p| p.value),
            num_members: c.num_members,
            is_private: c.flags.is_private.unwrap_or(false),
            is_archived: c.flags.is_archived.unwrap_or(false),
        })
        .collect();

    output.print_list(&channels, "Channels");

    Ok(())
}

pub async fn info(client: &Client, output: &Output, channel: &str) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());

    let request = SlackApiConversationsInfoRequest::new(channel_id);
    let response = session.conversations_info(&request).await?;

    let c = response.channel;
    let info = ChannelInfo {
        id: c.id.0,
        name: c.name.unwrap_or_default(),
        topic: c.topic.map(|t| t.value),
        purpose: c.purpose.map(|p| p.value),
        num_members: c.num_members,
        is_private: c.flags.is_private.unwrap_or(false),
        is_archived: c.flags.is_archived.unwrap_or(false),
    };

    output.print(&info);

    Ok(())
}

pub async fn history(
    client: &Client,
    output: &Output,
    channel: &str,
    limit: Option<u16>,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());

    let request = SlackApiConversationsHistoryRequest::new()
        .with_channel(channel_id)
        .with_limit(limit.unwrap_or(20));

    let response = session.conversations_history(&request).await?;

    let messages: Vec<MessageInfo> = response
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
            }
        })
        .collect();

    output.print_list(&messages, &format!("Messages in {}", channel));

    Ok(())
}

pub async fn members(
    client: &Client,
    output: &Output,
    channel: &str,
    limit: Option<u16>,
) -> Result<()> {
    let session = client.session();
    let channel_id = SlackChannelId::new(channel.to_string());

    let request = SlackApiConversationsMembersRequest::new()
        .with_channel(channel_id)
        .with_limit(limit.unwrap_or(100));

    let response = session.conversations_members(&request).await?;

    let members: Vec<MemberInfo> = response
        .members
        .into_iter()
        .map(|id| MemberInfo {
            id: id.0,
            name: None,
        })
        .collect();

    output.print_list(&members, &format!("Members of {}", channel));

    Ok(())
}
