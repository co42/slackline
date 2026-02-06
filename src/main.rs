use clap::{Parser, Subcommand};
use slackline::{Config, Output, SlackClient, commands};

const ABOUT: &str = "Read-only Slack CLI for AI agents.";

#[derive(Parser)]
#[command(name = "slackline")]
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
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List channels, read messages, get members
    Channels {
        #[command(subcommand)]
        command: ChannelCommands,
    },
    /// List users, search by name/email, get info and presence
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
    /// File operations (info, download)
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
    /// Show instructions and URL to create a new Slack token
    Create,
    /// Print the app manifest JSON
    Manifest,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let output = Output::new(cli.json, cli.quiet);

    // Print help if no command provided
    let Some(cmd) = cli.command else {
        use clap::CommandFactory;
        Cli::command().print_long_help()?;
        return Ok(());
    };

    // Handle token create/manifest commands (no auth required)
    if let Commands::Token { command } = &cmd {
        let result = match command {
            TokenCommands::Create => Some(commands::token::create(&output)),
            TokenCommands::Manifest => Some(commands::token::manifest(&output)),
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

    let config = match cli.token {
        Some(token) => Config::with_token(token),
        None => Config::from_env()?,
    };

    let client = SlackClient::new(&config)?;

    let result = match cmd {
        Commands::Token { command } => match command {
            TokenCommands::Test => commands::token::test(&client, &output).await,
            TokenCommands::Create | TokenCommands::Manifest => unreachable!(), // handled above
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
        },
        Commands::Dms { command } => match command {
            DmCommands::List { limit } => commands::dms::list(&client, &output, limit).await,
            DmCommands::History { dm_channel, limit } => {
                commands::dms::history(&client, &output, &dm_channel, limit).await
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
        },
        Commands::Me { command } => match command {
            MeCommands::Channels { limit, dms, unread } => {
                commands::me::channels(&client, &output, limit, dms, unread).await
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
