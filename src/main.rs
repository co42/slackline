use clap::{Parser, Subcommand};
use slackline::{Config, Output, SlackClient, commands};

const ABOUT: &str = "Read-only Slack CLI for AI agents.

WORKFLOW:
  1. me channels              # List channels you're in
  2. channels history <ID>    # Read messages, note 'ts' for threads
  3. messages replies <ID> <TS>  # Read thread if reply_count > 0
  4. search messages '<query>'   # Search (e.g., 'from:@user' or 'to:me')

FIND MENTIONS:
  search messages 'to:me'           # Messages sent to you
  search messages 'from:@someone'   # Messages from someone
  search messages 'in:#channel keyword'  # Keyword in channel

Use --json for machine-readable output.";

#[derive(Parser)]
#[command(name = "slack-cli")]
#[command(about = "Read-only Slack CLI for AI agents", long_about = ABOUT)]
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
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify token and show workspace info
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// List channels, read messages, get members
    Channels {
        #[command(subcommand)]
        command: ChannelCommands,
    },
    /// List users, get info and presence
    Users {
        #[command(subcommand)]
        command: UserCommands,
    },
    /// Read thread replies, get permalinks
    Messages {
        #[command(subcommand)]
        command: MessageCommands,
    },
    /// Direct messages (IMs and group DMs)
    Dms {
        #[command(subcommand)]
        command: DmCommands,
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
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Test token and show workspace/user info
    Test,
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
    },
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let output = Output::new(cli.json, cli.quiet);

    let config = match cli.token {
        Some(token) => Config::with_token(token),
        None => Config::from_env()?,
    };

    let client = SlackClient::new(&config)?;

    let result = match cli.command {
        Commands::Auth { command } => match command {
            AuthCommands::Test => commands::auth::test(&client, &output).await,
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
        },
        Commands::Users { command } => match command {
            UserCommands::List { limit } => commands::users::list(&client, &output, limit).await,
            UserCommands::Info { user } => commands::users::info(&client, &output, &user).await,
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
        },
        Commands::Dms { command } => match command {
            DmCommands::List { limit } => commands::dms::list(&client, &output, limit).await,
            DmCommands::History { dm_channel, limit } => {
                commands::dms::history(&client, &output, &dm_channel, limit).await
            }
        },
        Commands::Me { command } => match command {
            MeCommands::Channels { limit, dms } => {
                commands::me::channels(&client, &output, limit, dms).await
            }
        },
        Commands::Search { command } => match command {
            SearchCommands::Messages { query, limit } => {
                commands::search::messages(&client, &output, &query, limit).await
            }
        },
    };

    if let Err(e) = result {
        output.error(&e.to_string());
        std::process::exit(1);
    }

    Ok(())
}
