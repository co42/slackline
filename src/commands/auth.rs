use crate::client::Client;
use crate::error::Result;
use crate::output::{HumanReadable, Output};
use colored::Colorize;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuthInfo {
    pub url: String,
    pub team: String,
    pub user: String,
    pub team_id: String,
    pub user_id: String,
}

impl HumanReadable for AuthInfo {
    fn print_human(&self) {
        println!("{}: {}", "Team".cyan(), self.team);
        println!("{}: {}", "User".cyan(), self.user);
        println!("{}: {}", "Team ID".dimmed(), self.team_id);
        println!("{}: {}", "User ID".dimmed(), self.user_id);
        println!("{}: {}", "URL".dimmed(), self.url);
    }
}

pub async fn test(client: &Client, output: &Output) -> Result<()> {
    let response = client.auth_test().await?;

    let info = AuthInfo {
        url: response.url.0.to_string(),
        team: response.team,
        user: response.user.unwrap_or_default(),
        team_id: response.team_id.0,
        user_id: response.user_id.0,
    };

    output.print(&info);
    output.success("Authentication successful");

    Ok(())
}
