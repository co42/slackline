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

fn manifest_url(manifest: &serde_json::Value) -> String {
    let compact = serde_json::to_string(manifest).unwrap();
    let encoded = urlencoding::encode(&compact);
    format!("https://api.slack.com/apps?new_app=1&manifest_json={encoded}")
}

fn ro_manifest() -> serde_json::Value {
    build_manifest(
        "Slackline CLI (read-only)",
        "Slack CLI for AI agents (read-only)",
        READ_SCOPES,
    )
}

fn rw_manifest() -> serde_json::Value {
    let all_scopes: Vec<&str> = READ_SCOPES
        .iter()
        .chain(WRITE_SCOPES.iter())
        .copied()
        .collect();
    build_manifest("Slackline CLI", "Slack CLI for AI agents", &all_scopes)
}

pub fn create(output: &Output, readonly: bool) -> Result<()> {
    let manifest = if readonly {
        ro_manifest()
    } else {
        rw_manifest()
    };
    let mode = if readonly { "read-only" } else { "read-write" };
    let url = manifest_url(&manifest);

    if output.is_json() {
        let info = serde_json::json!({
            "mode": mode,
            "steps": [
                "Open the Slack app creation URL",
                "Select your workspace",
                "Click 'Create' to create the app from manifest",
                "Go to 'Install App' in the sidebar",
                "Click 'Install to Workspace' and authorize",
                "Copy the 'User OAuth Token' (starts with xoxp-)",
                "Store the token securely"
            ],
            "create_url": url,
            "manifest": manifest,
        });
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
    } else {
        println!();
        println!("{}", "═".repeat(60));
        println!(
            "  CREATE A SLACK USER TOKEN FOR SLACKLINE ({})",
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
        println!("  Read scopes: channels, groups, im, mpim (read + history),");
        println!("  files:read, search:read, users:read, users:read.email,");
        println!("  pins:read, reactions:read");
        if !readonly {
            println!("  Write scopes: chat:write, files:write, im:write,");
            println!("  pins:write, reactions:write, users.profile:write");
        }
        println!("{}", "─".repeat(60));
        println!();
    }
    Ok(())
}

const WATCH_BOT_SCOPES: &[&str] = &[
    "app_mentions:read",
    "channels:history",
    "channels:read",
    "groups:history",
    "groups:read",
    "im:history",
    "im:read",
    "mpim:history",
    "mpim:read",
    "reactions:read",
    "users:read",
    "users:read.email",
    "files:read",
];

const WATCH_EVENT_SUBSCRIPTIONS: &[&str] = &[
    "app_mention",
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

fn watch_manifest() -> serde_json::Value {
    serde_json::json!({
        "display_information": {
            "name": "Slackline Watch",
            "description": "Socket Mode event streaming for slackline CLI",
            "background_color": "#4a154b"
        },
        "features": {
            "bot_user": {
                "display_name": "Slackline Watch",
                "always_online": false
            }
        },
        "oauth_config": {
            "scopes": {
                "bot": WATCH_BOT_SCOPES
            }
        },
        "settings": {
            "event_subscriptions": {
                "bot_events": WATCH_EVENT_SUBSCRIPTIONS
            },
            "org_deploy_enabled": false,
            "socket_mode_enabled": true,
            "token_rotation_enabled": false
        }
    })
}

pub fn create_watch(output: &Output) -> Result<()> {
    let manifest = watch_manifest();
    let url = manifest_url(&manifest);

    if output.is_json() {
        let info = serde_json::json!({
            "mode": "watch",
            "steps": [
                "Open the Slack app creation URL",
                "Select your workspace",
                "Click 'Create' to create the app from manifest",
                "Go to 'Basic Information' → 'App-Level Tokens'",
                "Click 'Generate Token and Scopes'",
                "Name it (e.g. 'socket') and add the 'connections:write' scope",
                "Copy the token (starts with xapp-)",
                "Go to 'Install App' and install to your workspace",
                "Copy the 'Bot User OAuth Token' (starts with xoxb-) as your SLACK_TOKEN",
                "Store both tokens securely"
            ],
            "create_url": url,
            "manifest": manifest,
        });
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
    } else {
        println!();
        println!("{}", "═".repeat(60));
        println!("  CREATE A SOCKET MODE APP FOR SLACKLINE WATCH");
        println!("{}", "═".repeat(60));
        println!();
        println!("1. Open this URL to create a Slack app with Socket Mode:");
        println!();
        println!("   {}", url);
        println!();
        println!("2. Select your workspace and click 'Create'");
        println!();
        println!("3. Go to 'Basic Information' → 'App-Level Tokens'");
        println!("   Click 'Generate Token and Scopes'");
        println!("   Name: socket  |  Scope: connections:write");
        println!("   Copy the token (starts with xapp-)");
        println!();
        println!("4. Go to 'Install App' → 'Install to Workspace'");
        println!("   Copy the 'Bot User OAuth Token' (starts with xoxb-)");
        println!();
        println!("5. Store both tokens:");
        println!();
        println!("   export SLACK_TOKEN='xoxb-...'        # Bot token (API lookups)");
        println!("   export SLACK_APP_TOKEN='xapp-...'     # App token (Socket Mode)");
        println!();
        println!("6. Invite the bot to channels you want to watch:");
        println!("   /invite @Slackline Watch");
        println!();
        println!("7. Start streaming:");
        println!("   slackline watch");
        println!();
        println!("{}", "─".repeat(60));
        println!("  Bot scopes: channels/groups/im/mpim (read + history),");
        println!("  app_mentions:read, reactions:read, users:read, files:read");
        println!("  Events: messages, reactions, mentions, members, files,");
        println!("  channels, user status, team join");
        println!("{}", "─".repeat(60));
        println!();
    }
    Ok(())
}

pub fn manifest_watch(output: &Output) -> Result<()> {
    print_manifest(output, &watch_manifest())
}

pub fn manifest(output: &Output, readonly: bool) -> Result<()> {
    let manifest = if readonly {
        ro_manifest()
    } else {
        rw_manifest()
    };
    print_manifest(output, &manifest)
}

fn print_manifest(_output: &Output, manifest: &serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(manifest).unwrap());
    Ok(())
}
