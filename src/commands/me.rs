use crate::client::Client;
use crate::error::Result;
use crate::output::{HumanReadable, Output};
use colored::Colorize;
use futures::future::join_all;
use serde::Serialize;
use slack_morphism::prelude::*;

#[derive(Debug, Serialize)]
pub struct MyChannel {
    pub id: String,
    pub name: String,
    pub is_private: bool,
    pub is_im: bool,
    pub is_mpim: bool,
    pub num_members: Option<u64>,
    pub unread_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_unread: Option<bool>,
}

impl HumanReadable for MyChannel {
    fn print_human(&self) {
        let prefix = if self.is_im || self.is_mpim {
            "DM"
        } else if self.is_private {
            "🔒"
        } else {
            "#"
        };

        let unread = self
            .unread_count
            .filter(|&c| c > 0)
            .map(|c| format!(" [{}]", c).red().to_string())
            .or_else(|| {
                self.has_unread
                    .filter(|&u| u)
                    .map(|_| " [unread]".red().to_string())
            })
            .unwrap_or_default();

        let members = self
            .num_members
            .filter(|_| !self.is_im && !self.is_mpim)
            .map(|n| format!(" ({} members)", n))
            .unwrap_or_default();

        println!(
            "{} {}{}{}",
            prefix,
            self.name.bold(),
            members.dimmed(),
            unread
        );
    }
}

/// Check if a channel has unread messages by comparing last_read with latest message
async fn check_channel_unread(
    session: &SlackClientSession<'_, crate::client::HyperConnector>,
    channel_id: &SlackChannelId,
) -> Option<bool> {
    // Get channel info to get last_read
    let info_request = SlackApiConversationsInfoRequest::new(channel_id.clone());
    let info_response = session.conversations_info(&info_request).await.ok()?;
    let last_read = info_response.channel.last_state.last_read?;

    // Get latest message
    let history_request = SlackApiConversationsHistoryRequest::new()
        .with_channel(channel_id.clone())
        .with_limit(1);
    let history_response = session.conversations_history(&history_request).await.ok()?;
    let latest_message = history_response.messages.first()?;
    let latest_ts = &latest_message.origin.ts;

    // Compare timestamps (they're strings like "1713203474.121819")
    Some(latest_ts.0 > last_read.0)
}

/// List channels the current user is a member of
pub async fn channels(
    client: &Client,
    output: &Output,
    limit: Option<u16>,
    include_dms: bool,
    unread_only: bool,
) -> Result<()> {
    let session = client.session();

    let mut types = vec![
        SlackConversationType::Public,
        SlackConversationType::Private,
    ];

    if include_dms {
        types.push(SlackConversationType::Im);
        types.push(SlackConversationType::Mpim);
    }

    let request = SlackApiUsersConversationsRequest::new()
        .with_types(types)
        .with_exclude_archived(true)
        .with_limit(limit.unwrap_or(100));

    let response = session.users_conversations(&request).await?;

    let mut channels: Vec<MyChannel> = response
        .channels
        .into_iter()
        .map(|c| MyChannel {
            id: c.id.0,
            name: c.name.unwrap_or_else(|| "DM".to_string()),
            is_private: c.flags.is_private.unwrap_or(false),
            is_im: c.flags.is_im.unwrap_or(false),
            is_mpim: c.flags.is_mpim.unwrap_or(false),
            num_members: c.num_members,
            unread_count: c.last_state.unread_count,
            has_unread: None,
        })
        .collect();

    // If filtering by unread, check each channel
    if unread_only {
        output.status("Checking for unread messages...");

        // Check unread status for all channels concurrently
        let futures: Vec<_> = channels
            .iter()
            .map(|c| {
                let channel_id = SlackChannelId::new(c.id.clone());
                let session = client.session();
                async move { check_channel_unread(&session, &channel_id).await }
            })
            .collect();

        let results: Vec<Option<bool>> = join_all(futures).await;

        // Update channels with unread status and filter
        for (channel, has_unread) in channels.iter_mut().zip(results.into_iter()) {
            channel.has_unread = has_unread;
        }

        // Filter to only channels with unread messages
        // For DMs, use unread_count if available
        channels.retain(|c| {
            c.unread_count.map(|count| count > 0).unwrap_or(false) || c.has_unread.unwrap_or(false)
        });
    }

    let title = if unread_only {
        "Unread Channels"
    } else {
        "My Channels"
    };
    output.print_list(&channels, title);

    Ok(())
}
