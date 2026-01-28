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

const APP_MANIFEST: &str = r##"{
  "display_information": {
    "name": "Slackline CLI",
    "description": "Read-only Slack CLI for AI agents",
    "background_color": "#4a154b"
  },
  "oauth_config": {
    "scopes": {
      "user": [
        "channels:history",
        "channels:read",
        "files:read",
        "groups:history",
        "groups:read",
        "im:history",
        "im:read",
        "mpim:history",
        "mpim:read",
        "search:read",
        "users:read",
        "users:read.email"
      ]
    }
  },
  "settings": {
    "org_deploy_enabled": false,
    "socket_mode_enabled": false,
    "token_rotation_enabled": false
  }
}"##;

pub fn create(output: &Output) -> Result<()> {
    if output.is_json() {
        let info = serde_json::json!({
            "steps": [
                "Open the Slack app creation URL",
                "Select your workspace",
                "Click 'Create' to create the app from manifest",
                "Go to 'OAuth & Permissions' in the sidebar",
                "Click 'Install to Workspace' and authorize",
                "Copy the 'User OAuth Token' (starts with xoxp-)",
                "Store the token securely"
            ],
            "create_url": format!("https://api.slack.com/apps?new_app=1&manifest_json={}", urlencoded_manifest()),
            "manifest": serde_json::from_str::<serde_json::Value>(APP_MANIFEST).unwrap(),
            "scopes": [
                "channels:history",
                "channels:read",
                "files:read",
                "groups:history",
                "groups:read",
                "im:history",
                "im:read",
                "mpim:history",
                "mpim:read",
                "search:read",
                "users:read",
                "users:read.email"
            ]
        });
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
    } else {
        println!();
        println!("{}", "═".repeat(60));
        println!("  CREATE A SLACK USER TOKEN FOR SLACKLINE");
        println!("{}", "═".repeat(60));
        println!();
        println!("1. Open this URL to create a Slack app with the right permissions:");
        println!();
        println!("   {}", create_url());
        println!();
        println!("2. Select your workspace and click 'Create'");
        println!();
        println!("3. In the app settings, go to 'OAuth & Permissions'");
        println!();
        println!("4. Click 'Install to Workspace' and authorize");
        println!();
        println!("5. Copy the 'User OAuth Token' (starts with xoxp-)");
        println!();
        println!("6. Store it securely:");
        println!();
        println!("   # macOS Keychain (recommended):");
        println!("   security add-generic-password -s slack-token -a $USER -w 'xoxp-...'");
        println!();
        println!("   # Then use with:");
        println!("   export SLACK_TOKEN=$(security find-generic-password -s slack-token -w)");
        println!();
        println!("   # Or add to ~/.zshrc:");
        println!("   export SLACK_TOKEN='xoxp-...'");
        println!();
        println!("{}", "─".repeat(60));
        println!("  Scopes included: channels:read, channels:history,");
        println!("  files:read, groups:read/history, im:read/history,");
        println!("  mpim:read/history, search:read, users:read, users:read.email");
        println!("{}", "─".repeat(60));
        println!();
    }
    Ok(())
}

pub fn manifest(output: &Output) -> Result<()> {
    if output.is_json() {
        let manifest: serde_json::Value = serde_json::from_str(APP_MANIFEST).unwrap();
        println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
    } else {
        println!("{}", APP_MANIFEST);
    }
    Ok(())
}

fn urlencoded_manifest() -> String {
    urlencoding::encode(APP_MANIFEST).to_string()
}

fn create_url() -> String {
    format!(
        "https://api.slack.com/apps?new_app=1&manifest_json={}",
        urlencoded_manifest()
    )
}
