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

const READ_SCOPES: &[&str] = &[
    "channels:history",
    "channels:read",
    "files:read",
    "groups:history",
    "groups:read",
    "im:history",
    "im:read",
    "mpim:history",
    "mpim:read",
    "pins:read",
    "reactions:read",
    "search:read",
    "users:read",
    "users:read.email",
];

const WRITE_SCOPES: &[&str] = &[
    "chat:write",
    "files:write",
    "im:write",
    "pins:write",
    "reactions:write",
    "users.profile:write",
];

const EVENT_SUBSCRIPTIONS: &[&str] = &[
    "message.channels",
    "message.groups",
    "message.im",
    "message.mpim",
    "reaction_added",
    "reaction_removed",
    "member_joined_channel",
    "member_left_channel",
    "file_shared",
    "user_status_changed",
    "channel_created",
    "channel_deleted",
    "channel_archive",
    "channel_unarchive",
    "channel_rename",
    "team_join",
];

fn build_manifest(name: &str, description: &str, scopes: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "display_information": {
            "name": name,
            "description": description,
            "background_color": "#4a154b"
        },
        "oauth_config": {
            "scopes": {
                "user": scopes
            }
        },
        "settings": {
            "org_deploy_enabled": false,
            "socket_mode_enabled": false,
            "token_rotation_enabled": false
        }
    })
}

fn build_manifest_with_events(name: &str, description: &str, scopes: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "display_information": {
            "name": name,
            "description": description,
            "background_color": "#4a154b"
        },
        "oauth_config": {
            "scopes": {
                "user": scopes
            }
        },
        "settings": {
            "event_subscriptions": {
                "user_events": EVENT_SUBSCRIPTIONS
            },
            "org_deploy_enabled": false,
            "socket_mode_enabled": true,
            "token_rotation_enabled": false
        }
    })
}

fn manifest_url(manifest: &serde_json::Value) -> String {
    let compact = serde_json::to_string(manifest).unwrap();
    let encoded = urlencoding::encode(&compact);
    format!("https://api.slack.com/apps?new_app=1&manifest_json={encoded}")
}

fn make_manifest(write: bool, watch: bool) -> serde_json::Value {
    let scopes: Vec<&str> = if write {
        READ_SCOPES
            .iter()
            .chain(WRITE_SCOPES.iter())
            .copied()
            .collect()
    } else {
        READ_SCOPES.to_vec()
    };

    let mode = if write { "rw" } else { "ro" };
    let suffix = if watch {
        format!(" {mode} watch")
    } else {
        format!(" {mode}")
    };
    let name = format!("Slackline{suffix}");
    let description = format!("Slack CLI{suffix}");

    if watch {
        build_manifest_with_events(&name, &description, &scopes)
    } else {
        build_manifest(&name, &description, &scopes)
    }
}

pub fn create(output: &Output, write: bool, watch: bool) -> Result<()> {
    let manifest = make_manifest(write, watch);
    let mode = match (write, watch) {
        (true, true) => "read-write + watch",
        (true, false) => "read-write",
        (false, true) => "read + watch",
        (false, false) => "read-only",
    };
    let url = manifest_url(&manifest);

    if output.is_json() {
        let mut steps = vec![
            "Open the Slack app creation URL",
            "Select your workspace",
            "Click 'Create' to create the app from manifest",
            "Go to 'Install App' in the sidebar",
            "Click 'Install to Workspace' and authorize",
            "Copy the 'User OAuth Token' (starts with xoxp-)",
            "Store the token securely",
        ];
        if watch {
            steps.push("Go to 'Basic Information' → 'App-Level Tokens'");
            steps.push("Click 'Generate Token and Scopes'");
            steps.push("Name it (e.g. 'socket') and add the 'connections:write' scope");
            steps.push("Copy the app token (starts with xapp-)");
        }
        let info = serde_json::json!({
            "mode": mode,
            "steps": steps,
            "create_url": url,
            "manifest": manifest,
        });
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
    } else {
        println!();
        println!("{}", "═".repeat(60));
        println!(
            "  CREATE A SLACK APP FOR SLACKLINE ({})",
            mode.to_uppercase()
        );
        println!("{}", "═".repeat(60));
        println!();
        println!("1. Open this URL to create a Slack app with the right permissions:");
        println!();
        println!("   {}", url);
        println!();
        println!("2. Select your workspace and click 'Create'");
        println!();
        println!("3. Go to 'Install App' in the sidebar");
        println!();
        println!("4. Click 'Install to Workspace' and authorize");
        println!();
        println!("5. Copy the 'User OAuth Token' (starts with xoxp-)");
        println!();

        let mut step = 6;
        if watch {
            println!("{}. Go to 'Basic Information' → 'App-Level Tokens'", step);
            println!("   Click 'Generate Token and Scopes'");
            println!("   Name: socket  |  Scope: connections:write");
            println!("   Copy the token (starts with xapp-)");
            println!();
            step += 1;
        }

        println!("{}. Store tokens securely:", step);
        println!();
        println!("   export SLACK_TOKEN='xoxp-...'");
        if watch {
            println!("   export SLACK_APP_TOKEN='xapp-...'");
        }
        println!();
        println!("{}", "─".repeat(60));
        println!("  Read scopes: channels, groups, im, mpim (read + history),");
        println!("  files:read, search:read, users:read, users:read.email,");
        println!("  pins:read, reactions:read");
        if write {
            println!("  Write scopes: chat:write, files:write, im:write,");
            println!("  pins:write, reactions:write, users.profile:write");
        }
        if watch {
            println!("  Events: messages, reactions, members, files, channels,");
            println!("  user status, team join");
        }
        println!("{}", "─".repeat(60));
        println!();
    }
    Ok(())
}

pub fn manifest(output: &Output, write: bool, watch: bool) -> Result<()> {
    let manifest = make_manifest(write, watch);
    if output.is_json() {
        println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
    } else {
        println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
        println!();
        let url = manifest_url(&manifest);
        println!("Create app: {}", url);
    }
    Ok(())
}
