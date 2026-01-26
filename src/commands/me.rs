use crate::client::Client;
use crate::error::Result;
use crate::output::{HumanReadable, Output};
use colored::Colorize;
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

/// List channels the current user is a member of
pub async fn channels(
    client: &Client,
    output: &Output,
    limit: Option<u16>,
    include_dms: bool,
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

    let channels: Vec<MyChannel> = response
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
        })
        .collect();

    output.print_list(&channels, "My Channels");

    Ok(())
}
