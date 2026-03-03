use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use slackline::{Config, Output, SlackClient, commands};
use slackline::commands::watch::EventFilter;

const ABOUT: &str = "Slack CLI for AI agents.";

#[derive(Parser)]
#[command(name = "slackline")]
#[command(about = "Slack CLI for AI agents", long_about = ABOUT)]
#[command(version)]
struct Cli {
    /// Slack token (or set SLACK_TOKEN env var)
    #[arg(long, env = "SLACK_TOKEN")]
    token: Option<String>,

    /// Output JSON instead of human-readable format
    #[arg(long, global = true)]
    json: bool,

    /// Suppress status messages
    #[arg(long, short, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List channels, read messages, get members, list pins
    Channels {
        #[command(subcommand)]
        command: ChannelCommands,
    },
    /// List users, search by name/email, get info and presence
    Users {
        #[command(subcommand)]
        command: UserCommands,
    },
    /// Read threads, send messages, react, pin
    Messages {
        #[command(subcommand)]
        command: MessageCommands,
    },
    /// Direct messages (IMs and group DMs)
    Dms {
        #[command(subcommand)]
        command: DmCommands,
    },
    /// File operations (info, download, upload)
    Files {
        #[command(subcommand)]
        command: FileCommands,
    },
    /// Current user shortcuts
    Me {
        #[command(subcommand)]
        command: MeCommands,
    },
    /// Search messages across workspace
    Search {
        #[command(subcommand)]
        command: SearchCommands,
    },
    /// Create and manage Slack tokens
    Token {
        #[command(subcommand)]
        command: TokenCommands,
    },
    /// Stream real-time events via Socket Mode (JSONL to stdout)
    Watch {
        /// Event types to stream (comma-separated: message,mention,reaction,dm,channel,file,member,status,all)
        #[arg(long, value_delimiter = ',', value_parser = parse_event_filter)]
        events: Vec<EventFilter>,
        /// Filter to specific channel IDs (comma-separated, e.g. C1RCG46LS,C0AB2G3EY)
        #[arg(long, value_delimiter = ',')]
        channels: Vec<String>,
        /// Output raw slack-morphism event JSON instead of normalized format
        #[arg(long)]
        raw: bool,
    },
}

#[derive(Subcommand)]
enum ChannelCommands {
    /// List channels (returns id, name, topic, member count)
    List {
        /// Max channels to return [default: 100]
        #[arg(long, short)]
        limit: Option<u16>,
    },
    /// Get channel details by ID
    Info {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
    },
    /// Read recent messages from channel
    History {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
        /// Max messages to return [default: 20]
        #[arg(long, short, default_value = "20")]
        limit: Option<u16>,
    },
    /// List channel member user IDs
    Members {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
        /// Max members to return [default: 100]
        #[arg(long, short)]
        limit: Option<u16>,
    },
    /// List pinned messages in channel
    Pins {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
    },
}

#[derive(Subcommand)]
enum UserCommands {
    /// List users (returns id, name, real_name, title)
    List {
        /// Max users to return [default: 100]
        #[arg(long, short)]
        limit: Option<u16>,
    },
    /// Get user details by ID
    Info {
        /// User ID (e.g., U032LQBJTH8)
        user: String,
    },
    /// Search users by name, username, or email
    Search {
        /// Search query (matches against name, username, email)
        query: String,
    },
    /// Check if user is online or away
    Presence {
        /// User ID (e.g., U032LQBJTH8)
        user: String,
    },
}

#[derive(Subcommand)]
enum MessageCommands {
    /// Read thread replies (use ts from parent message)
    Replies {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
        /// Thread timestamp from parent message (e.g., 1769415774.159039)
        thread_ts: String,
        /// Max replies to return [default: 100]
        #[arg(long, short)]
        limit: Option<u16>,
    },
    /// Get shareable URL for a message
    Permalink {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
        /// Message timestamp (e.g., 1769415774.159039)
        message_ts: String,
    },
    /// Get reactions on a message
    Reactions {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
        /// Message timestamp (e.g., 1769415774.159039)
        ts: String,
    },
    /// Send a message to a channel
    Send {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
        /// Message text
        text: String,
        /// Reply in thread (parent message timestamp)
        #[arg(long)]
        thread_ts: Option<String>,
    },
    /// Add an emoji reaction to a message
    React {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
        /// Message timestamp (e.g., 1769415774.159039)
        ts: String,
        /// Emoji name without colons (e.g., thumbsup)
        emoji: String,
    },
    /// Remove an emoji reaction from a message
    Unreact {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
        /// Message timestamp (e.g., 1769415774.159039)
        ts: String,
        /// Emoji name without colons (e.g., thumbsup)
        emoji: String,
    },
    /// Pin a message to a channel
    Pin {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
        /// Message timestamp (e.g., 1769415774.159039)
        ts: String,
    },
    /// Unpin a message from a channel
    Unpin {
        /// Channel ID (e.g., C1RCG46LS)
        channel: String,
        /// Message timestamp (e.g., 1769415774.159039)
        ts: String,
    },
}

#[derive(Subcommand)]
enum DmCommands {
    /// List DM conversations (returns channel ID for each)
    List {
        /// Max DMs to return [default: 50]
        #[arg(long, short)]
        limit: Option<u16>,
    },
    /// Read DM history (use DM channel ID from 'dms list')
    History {
        /// DM channel ID (e.g., D01234567)
        dm_channel: String,
        /// Max messages to return [default: 20]
        #[arg(long, short)]
        limit: Option<u16>,
    },
    /// Send a direct message to a user
    Send {
        /// User ID (e.g., U032LQBJTH8)
        user: String,
        /// Message text
        text: String,
    },
}

#[derive(Subcommand)]
enum FileCommands {
    /// List files in workspace
    List {
        /// Filter by channel ID
        #[arg(long, short)]
        channel: Option<String>,
        /// Filter by user ID
        #[arg(long, short)]
        user: Option<String>,
        /// Max files to return [default: 100]
        #[arg(long, short)]
        limit: Option<u32>,
    },
    /// Get file details by ID
    Info {
        /// File ID (e.g., F0AB1G1EY5V)
        file: String,
    },
    /// Download a file
    Download {
        /// File ID (e.g., F0AB1G1EY5V)
        file: String,
        /// Output path (defaults to stdout)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Upload a file to Slack
    Upload {
        /// Path to file on disk
        path: String,
        /// Channel to share to (optional)
        #[arg(long, short)]
        channel: Option<String>,
        /// Thread timestamp to post in (optional)
        #[arg(long)]
        thread_ts: Option<String>,
        /// Initial comment when sharing (optional)
        #[arg(long)]
        comment: Option<String>,
    },
}

#[derive(Subcommand)]
enum MeCommands {
    /// List channels you're a member of
    Channels {
        /// Max channels to return [default: 100]
        #[arg(long, short)]
        limit: Option<u16>,
        /// Include DMs in the list
        #[arg(long)]
        dms: bool,
        /// Only show channels with unread messages
        #[arg(long, short)]
        unread: bool,
    },
    /// Set your Slack status
    SetStatus {
        /// Status text
        text: String,
        /// Status emoji (e.g., :coffee:) [default: :speech_balloon:]
        #[arg(long, short)]
        emoji: Option<String>,
    },
    /// Clear your Slack status
    ClearStatus,
}

#[derive(Subcommand)]
enum SearchCommands {
    /// Search messages (supports Slack search syntax)
    ///
    /// Examples:
    ///   'to:me'              - Messages sent to you
    ///   'from:@username'     - Messages from a user
    ///   'in:#channel word'   - Word in specific channel
    ///   'has:link'           - Messages with links
    ///   'before:today'       - Messages before today
    Messages {
        /// Search query (Slack search syntax)
        query: String,
        /// Max results to return [default: 20]
        #[arg(long, short)]
        limit: Option<u16>,
    },
}

#[derive(Subcommand)]
enum TokenCommands {
    /// Test token and show workspace/user info
    Test,
    /// Show instructions and URL to create a new Slack token (read-only by default)
    Create {
        /// Include write scopes (chat:write, files:write, etc.)
        #[arg(long, conflicts_with = "watch")]
        write: bool,
        /// Create a Socket Mode app for `slackline watch` (includes bot + event subscriptions)
        #[arg(long, conflicts_with = "write")]
        watch: bool,
    },
    /// Print the app manifest JSON (read-only by default)
    Manifest {
        /// Include write scopes (chat:write, files:write, etc.)
        #[arg(long, conflicts_with = "watch")]
        write: bool,
        /// Print Socket Mode manifest for `slackline watch`
        #[arg(long, conflicts_with = "write")]
        watch: bool,
    },
}

fn parse_event_filter(s: &str) -> std::result::Result<EventFilter, String> {
    EventFilter::parse(s)
}

fn resolve_config(token: Option<String>) -> anyhow::Result<Config> {
    Ok(match token {
        Some(token) => Config::with_token(token),
        None => Config::from_env()?,
    })
}

fn is_readonly() -> bool {
    std::env::var("SLACKLINE_READONLY")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

/// Names of write subcommands that should be hidden in readonly mode
const WRITE_MESSAGE_CMDS: &[&str] = &["send", "react", "unreact", "pin", "unpin"];
const WRITE_DM_CMDS: &[&str] = &["send"];
const WRITE_FILE_CMDS: &[&str] = &["upload"];
const WRITE_ME_CMDS: &[&str] = &["set-status", "clear-status"];

fn hide_write_subcommands(mut cmd: clap::Command) -> clap::Command {
    for name in WRITE_MESSAGE_CMDS {
        cmd = cmd.mut_subcommand("messages", |m| m.mut_subcommand(name, |s| s.hide(true)));
    }
    for name in WRITE_DM_CMDS {
        cmd = cmd.mut_subcommand("dms", |m| m.mut_subcommand(name, |s| s.hide(true)));
    }
    for name in WRITE_FILE_CMDS {
        cmd = cmd.mut_subcommand("files", |m| m.mut_subcommand(name, |s| s.hide(true)));
    }
    for name in WRITE_ME_CMDS {
        cmd = cmd.mut_subcommand("me", |m| m.mut_subcommand(name, |s| s.hide(true)));
    }
    cmd
}

fn is_write_command(cmd: &Commands) -> bool {
    matches!(
        cmd,
        Commands::Messages {
            command: MessageCommands::Send { .. }
                | MessageCommands::React { .. }
                | MessageCommands::Unreact { .. }
                | MessageCommands::Pin { .. }
                | MessageCommands::Unpin { .. }
        } | Commands::Dms {
            command: DmCommands::Send { .. }
        } | Commands::Files {
            command: FileCommands::Upload { .. }
        } | Commands::Me {
            command: MeCommands::SetStatus { .. } | MeCommands::ClearStatus
        }
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let readonly = is_readonly();

    // Build command, hiding write subcommands if readonly
    let mut cmd = Cli::command();
    if readonly {
        cmd = hide_write_subcommands(cmd);
    }

    let matches = cmd.get_matches();
    let cli = Cli::from_arg_matches(&matches)?;
    let output = Output::new(cli.json, cli.quiet);

    // Print help if no command provided
    let Some(cmd) = cli.command else {
        let mut help_cmd = Cli::command();
        if readonly {
            help_cmd = hide_write_subcommands(help_cmd);
        }
        help_cmd.print_long_help()?;
        return Ok(());
    };

    // Guard write commands in readonly mode
    if readonly && is_write_command(&cmd) {
        output.error("Write operations are disabled (SLACKLINE_READONLY is set)");
        std::process::exit(1);
    }

    // Handle token create/manifest commands (no auth required)
    if let Commands::Token { command } = &cmd {
        let result = match command {
            TokenCommands::Create { watch: true, .. } => {
                Some(commands::token::create_watch(&output))
            }
            TokenCommands::Create { write, .. } => Some(commands::token::create(&output, !write)),
            TokenCommands::Manifest { watch: true, .. } => {
                Some(commands::token::manifest_watch(&output))
            }
            TokenCommands::Manifest { write, .. } => {
                Some(commands::token::manifest(&output, !write))
            }
            TokenCommands::Test => None, // requires auth, handled below
        };
        if let Some(result) = result {
            if let Err(e) = result {
                output.error(&e.to_string());
                std::process::exit(1);
            }
            return Ok(());
        }
    }

    // Handle watch command (needs config but not the usual client)
    if let Commands::Watch {
        events,
        channels,
        raw,
    } = &cmd
    {
        let config = resolve_config(cli.token)?;
        if let Err(e) = commands::watch::listen(&config, events, channels, *raw).await {
            output.error(&e.to_string());
            std::process::exit(1);
        }
        return Ok(());
    }

    let config = resolve_config(cli.token)?;

    let client = SlackClient::new(&config)?;

    let result = match cmd {
        Commands::Token { command } => match command {
            TokenCommands::Test => commands::token::test(&client, &output).await,
            TokenCommands::Create { .. } | TokenCommands::Manifest { .. } => unreachable!(),
        },
        Commands::Channels { command } => match command {
            ChannelCommands::List { limit } => {
                commands::channels::list(&client, &output, limit).await
            }
            ChannelCommands::Info { channel } => {
                commands::channels::info(&client, &output, &channel).await
            }
            ChannelCommands::History { channel, limit } => {
                commands::channels::history(&client, &output, &channel, limit).await
            }
            ChannelCommands::Members { channel, limit } => {
                commands::channels::members(&client, &output, &channel, limit).await
            }
            ChannelCommands::Pins { channel } => {
                commands::channels::pins(&client, &output, &channel).await
            }
        },
        Commands::Users { command } => match command {
            UserCommands::List { limit } => commands::users::list(&client, &output, limit).await,
            UserCommands::Info { user } => commands::users::info(&client, &output, &user).await,
            UserCommands::Search { query } => {
                commands::users::search(&client, &output, &query).await
            }
            UserCommands::Presence { user } => {
                commands::users::presence(&client, &output, &user).await
            }
        },
        Commands::Messages { command } => match command {
            MessageCommands::Replies {
                channel,
                thread_ts,
                limit,
            } => commands::messages::replies(&client, &output, &channel, &thread_ts, limit).await,
            MessageCommands::Permalink {
                channel,
                message_ts,
            } => commands::messages::permalink(&client, &output, &channel, &message_ts).await,
            MessageCommands::Reactions { channel, ts } => {
                commands::messages::reactions(&client, &output, &channel, &ts).await
            }
            MessageCommands::Send {
                channel,
                text,
                thread_ts,
            } => {
                commands::messages::send(&client, &output, &channel, &text, thread_ts.as_deref())
                    .await
            }
            MessageCommands::React { channel, ts, emoji } => {
                commands::messages::react(&client, &output, &channel, &ts, &emoji).await
            }
            MessageCommands::Unreact { channel, ts, emoji } => {
                commands::messages::unreact(&client, &output, &channel, &ts, &emoji).await
            }
            MessageCommands::Pin { channel, ts } => {
                commands::messages::pin(&client, &output, &channel, &ts).await
            }
            MessageCommands::Unpin { channel, ts } => {
                commands::messages::unpin(&client, &output, &channel, &ts).await
            }
        },
        Commands::Dms { command } => match command {
            DmCommands::List { limit } => commands::dms::list(&client, &output, limit).await,
            DmCommands::History { dm_channel, limit } => {
                commands::dms::history(&client, &output, &dm_channel, limit).await
            }
            DmCommands::Send { user, text } => {
                commands::dms::send(&client, &output, &user, &text).await
            }
        },
        Commands::Files { command } => match command {
            FileCommands::List {
                channel,
                user,
                limit,
            } => {
                commands::files::list(&client, &output, channel.as_deref(), user.as_deref(), limit)
                    .await
            }
            FileCommands::Info { file } => commands::files::info(&client, &output, &file).await,
            FileCommands::Download { file, output: out } => {
                commands::files::download(&client, &file, out.as_deref()).await
            }
            FileCommands::Upload {
                path,
                channel,
                thread_ts,
                comment,
            } => {
                commands::files::upload(
                    &client,
                    &output,
                    &path,
                    channel.as_deref(),
                    thread_ts.as_deref(),
                    comment.as_deref(),
                )
                .await
            }
        },
        Commands::Me { command } => match command {
            MeCommands::Channels { limit, dms, unread } => {
                commands::me::channels(&client, &output, limit, dms, unread).await
            }
            MeCommands::SetStatus { text, emoji } => {
                commands::me::set_status(&client, &output, &text, emoji.as_deref()).await
            }
            MeCommands::ClearStatus => commands::me::clear_status(&client, &output).await,
        },
        Commands::Search { command } => match command {
            SearchCommands::Messages { query, limit } => {
                commands::search::messages(&client, &output, &query, limit).await
            }
        },
        Commands::Watch { .. } => unreachable!("handled above"),
    };

    if let Err(e) = result {
        output.error(&e.to_string());
        std::process::exit(1);
    }

    Ok(())
}
