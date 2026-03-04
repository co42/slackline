use crate::client::Client;
use crate::error::Result;
use crate::output::{HumanReadable, Output};
use colored::Colorize;
use futures::TryStreamExt;
use serde::Serialize;
use slack_morphism::prelude::*;

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub real_name: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub title: Option<String>,
    pub is_admin: bool,
    pub is_bot: bool,
    pub deleted: bool,
    pub tz: Option<String>,
}

impl HumanReadable for UserInfo {
    fn print_human(&self) {
        let status = if self.deleted {
            " (deleted)".red().to_string()
        } else if self.is_bot {
            " (bot)".cyan().to_string()
        } else if self.is_admin {
            " (admin)".yellow().to_string()
        } else {
            String::new()
        };

        let display = self
            .display_name
            .as_deref()
            .or(self.real_name.as_deref())
            .unwrap_or(&self.name);

        println!("@{} - {}{}", self.name.green(), display.bold(), status);
        println!("  {}: {}", "ID".dimmed(), self.id);
        if let Some(title) = &self.title
            && !title.is_empty()
        {
            println!("  {}: {}", "Title".dimmed(), title);
        }
        if let Some(email) = &self.email {
            println!("  {}: {}", "Email".dimmed(), email);
        }
        if let Some(tz) = &self.tz {
            println!("  {}: {}", "TZ".dimmed(), tz);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PresenceInfo {
    pub user_id: String,
    pub presence: String,
    pub online: bool,
}

impl HumanReadable for PresenceInfo {
    fn print_human(&self) {
        let status = if self.online {
            "online".green()
        } else {
            "away".dimmed()
        };
        println!("{}: {}", self.user_id, status);
    }
}

fn user_from_slack(u: SlackUser) -> UserInfo {
    let profile = u.profile;
    UserInfo {
        id: u.id.0,
        name: u.name.unwrap_or_default(),
        real_name: profile.as_ref().and_then(|p| p.real_name.clone()),
        display_name: profile.as_ref().and_then(|p| p.display_name.clone()),
        email: profile
            .as_ref()
            .and_then(|p| p.email.as_ref().map(|e| e.0.clone())),
        title: profile.as_ref().and_then(|p| p.title.clone()),
        is_admin: u.flags.is_admin.unwrap_or(false),
        is_bot: u.flags.is_bot.unwrap_or(false),
        deleted: u.deleted.unwrap_or(false),
        tz: u.tz,
    }
}

pub async fn list(client: &Client, output: &Output, limit: Option<u16>) -> Result<()> {
    let session = client.session();

    let request = SlackApiUsersListRequest::new().with_limit(limit.unwrap_or(200));
    let scroller = request.scroller();
    let mut stream = scroller.to_items_stream(&session);

    let mut users: Vec<UserInfo> = Vec::new();
    while let Some(batch) = stream.try_next().await? {
        users.extend(
            batch
                .into_iter()
                .filter(|u| !u.deleted.unwrap_or(false))
                .map(user_from_slack),
        );
    }

    output.print_list(&users, "Users");

    Ok(())
}

pub async fn search(client: &Client, output: &Output, query: &str) -> Result<()> {
    let session = client.session();

    let request = SlackApiUsersListRequest::new().with_limit(200);
    let scroller = request.scroller();
    let mut stream = scroller.to_items_stream(&session);

    let query_lower = query.to_lowercase();

    let mut users: Vec<UserInfo> = Vec::new();
    while let Some(batch) = stream.try_next().await? {
        users.extend(
            batch
                .into_iter()
                .filter(|u| !u.deleted.unwrap_or(false))
                .map(user_from_slack)
                .filter(|u| {
                    u.name.to_lowercase().contains(&query_lower)
                        || u.real_name
                            .as_ref()
                            .is_some_and(|n| n.to_lowercase().contains(&query_lower))
                        || u.display_name
                            .as_ref()
                            .is_some_and(|n| n.to_lowercase().contains(&query_lower))
                        || u.email
                            .as_ref()
                            .is_some_and(|e| e.to_lowercase().contains(&query_lower))
                }),
        );
    }

    output.print_list(&users, &format!("Users matching '{query}'"));

    Ok(())
}

pub async fn info(client: &Client, output: &Output, user: &str) -> Result<()> {
    let session = client.session();
    let user_id = client.resolve_user(user).await?;

    let request = SlackApiUsersInfoRequest::new(user_id);
    let response = session.users_info(&request).await?;

    let info = user_from_slack(response.user);

    output.print(&info);

    Ok(())
}

pub async fn presence(client: &Client, output: &Output, user: &str) -> Result<()> {
    let session = client.session();
    let user_id = client.resolve_user(user).await?;

    let request = SlackApiUsersGetPresenceRequest::new(user_id.clone());
    let response = session.users_get_presence(&request).await?;

    let info = PresenceInfo {
        user_id: user_id.0,
        presence: response.presence.clone(),
        online: response.presence == "active",
    };

    output.print(&info);

    Ok(())
}
