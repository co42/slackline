use crate::client::Client;
use crate::error::Result;
use crate::output::{HumanReadable, Output};
use crate::timeparse::parse_time_expr;
use chrono::{DateTime, Utc};
use colored::Colorize;
use futures::TryStreamExt;
use serde::Serialize;
use slack_morphism::prelude::*;
use std::collections::HashMap;

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
        println!("  {}: {}", "ID".dimmed(), self.id);
        if let Some(topic) = &self.topic
            && !topic.is_empty()
        {
            println!("  {}: {}", "Topic".dimmed(), topic);
        }
        if let Some(purpose) = &self.purpose
            && !purpose.is_empty()
        {
            println!("  {}: {}", "Purpose".dimmed(), purpose);
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
    pub latest_reply: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_name: Option<String>,
}

impl HumanReadable for MessageInfo {
    fn print_human(&self) {
        let time = self
            .timestamp
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| self.ts.clone());

        let user_display = match (&self.username, &self.real_name) {
            (Some(uname), Some(rname)) => format!("{} ({})", uname, rname),
            (Some(uname), None) => uname.clone(),
            _ => self.user.as_deref().unwrap_or("unknown").to_string(),
        };

        let thread_info = self
            .reply_count
            .filter(|&c| c > 0)
            .map(|c| format!(" [{} replies]", c).cyan().to_string())
            .unwrap_or_default();

        let reply_info = self
            .latest_reply
            .as_ref()
            .map(|ts| format!(" (latest reply: {})", ts).dimmed().to_string())
            .unwrap_or_default();

        println!(
            "{} {}{}{}:",
            time.dimmed(),
            user_display.green(),
            thread_info,
            reply_info
        );
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

#[derive(Debug, Serialize)]
pub struct PinnedMessage {
    pub channel: String,
    pub ts: String,
    pub text: String,
    pub pinned_by: Option<String>,
    pub pinned_at: Option<DateTime<Utc>>,
}

impl HumanReadable for PinnedMessage {
    fn print_human(&self) {
        let pinned_by = self.pinned_by.as_deref().unwrap_or("unknown");
        let pinned_at = self
            .pinned_at
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();

        println!("pinned by {} {}", pinned_by.green(), pinned_at.dimmed());
        println!("  ts: {}", self.ts.dimmed());
        if !self.text.is_empty() {
            println!("  {}", self.text);
        }
        println!();
    }
}

pub async fn list(client: &Client, output: &Output, limit: Option<u16>) -> Result<()> {
    let session = client.session();

    let request = SlackApiConversationsListRequest::new()
        .with_exclude_archived(true)
        .with_limit(limit.unwrap_or(200));

    let mut all_channels: Vec<SlackChannelInfo> = Vec::new();
    let scroller = request.scroller();
    let mut stream = scroller.to_items_stream(&session);

    while let Some(batch) = stream.try_next().await? {
        all_channels.extend(batch);
    }

    let channels: Vec<ChannelInfo> = all_channels
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
    let channel_id = client.resolve_channel(channel).await?;

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
    after: Option<&str>,
    before: Option<&str>,
    enrich: bool,
) -> Result<()> {
    let session = client.session();
    let channel_id = client.resolve_channel(channel).await?;

    let mut request = SlackApiConversationsHistoryRequest::new()
        .with_channel(channel_id)
        .with_limit(limit.unwrap_or(20));

    if let Some(after) = after {
        let ts = parse_time_expr(after)
            .map_err(crate::error::SlackCliError::Api)?;
        request = request.with_oldest(SlackTs::new(ts));
    }
    if let Some(before) = before {
        let ts = parse_time_expr(before)
            .map_err(crate::error::SlackCliError::Api)?;
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

    output.print_list(&messages, &format!("Messages in {}", channel));

    Ok(())
}

/// Batch-fetch user info for all unique user IDs in messages and populate username/real_name.
pub async fn enrich_messages(client: &Client, messages: &mut [MessageInfo]) -> Result<()> {
    let user_ids: Vec<String> = messages
        .iter()
        .filter_map(|m| m.user.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if user_ids.is_empty() {
        return Ok(());
    }

    let session = client.session();
    let mut user_map: HashMap<String, (String, Option<String>)> = HashMap::new();

    for uid in &user_ids {
        let user_id = SlackUserId::new(uid.clone());
        let request = SlackApiUsersInfoRequest::new(user_id);
        match session.users_info(&request).await {
            Ok(resp) => {
                let name = resp.user.name.unwrap_or_default();
                let real_name = resp
                    .user
                    .profile
                    .and_then(|p| p.real_name);
                user_map.insert(uid.clone(), (name, real_name));
            }
            Err(_) => continue, // skip users we can't resolve
        }
    }

    for msg in messages.iter_mut() {
        if let Some(uid) = &msg.user
            && let Some((uname, rname)) = user_map.get(uid)
        {
            msg.username = Some(uname.clone());
            msg.real_name = rname.clone();
        }
    }

    Ok(())
}

pub async fn members(
    client: &Client,
    output: &Output,
    channel: &str,
    limit: Option<u16>,
) -> Result<()> {
    let session = client.session();
    let channel_id = client.resolve_channel(channel).await?;

    let request = SlackApiConversationsMembersRequest::new()
        .with_channel(channel_id)
        .with_limit(limit.unwrap_or(200));

    let mut all_members: Vec<SlackUserId> = Vec::new();
    let scroller = request.scroller();
    let mut stream = scroller.to_items_stream(&session);

    while let Some(batch) = stream.try_next().await? {
        all_members.extend(batch);
    }

    let members: Vec<MemberInfo> = all_members
        .into_iter()
        .map(|id| MemberInfo {
            id: id.0,
            name: None,
        })
        .collect();

    output.print_list(&members, &format!("Members of {}", channel));

    Ok(())
}

/// List pinned messages in a channel
pub async fn pins(client: &Client, output: &Output, channel: &str) -> Result<()> {
    let session = client.session();
    let channel_id = client.resolve_channel(channel).await?;

    let request = SlackApiPinsListRequest::new(channel_id);
    let response = session.pins_list(&request).await?;

    let pinned: Vec<PinnedMessage> = response
        .items
        .into_iter()
        .filter_map(|item| {
            let msg = item.message?;
            let pinned_at = DateTime::from_timestamp(item.created.0.timestamp(), 0);

            Some(PinnedMessage {
                channel: channel.to_string(),
                ts: msg.origin.ts.0,
                text: msg.content.text.unwrap_or_default(),
                pinned_by: Some(item.created_by.0),
                pinned_at,
            })
        })
        .collect();

    output.print_list(&pinned, &format!("Pinned messages in {}", channel));

    Ok(())
}

/// Join a channel
pub async fn join(client: &Client, output: &Output, channel: &str) -> Result<()> {
    let session = client.session();
    let channel_id = client.resolve_channel(channel).await?;

    let request = SlackApiConversationsJoinRequest::new(channel_id);
    let response = session.conversations_join(&request).await?;

    let name = response.channel.name.unwrap_or_else(|| channel.to_string());
    output.success(&format!("Joined #{}", name));

    Ok(())
}

/// Leave a channel
pub async fn leave(client: &Client, output: &Output, channel: &str) -> Result<()> {
    let session = client.session();
    let channel_id = client.resolve_channel(channel).await?;

    let request = SlackApiConversationsLeaveRequest::new(channel_id);
    session.conversations_leave(&request).await?;

    output.success(&format!("Left #{}", channel));

    Ok(())
}
