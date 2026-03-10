use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlackCliError {
    #[error("Slack API error: {0}")]
    Api(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Rate limited: {0}")]
    RateLimit(String),

    #[error(transparent)]
    Http(#[from] slack_morphism::errors::SlackClientError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl SlackCliError {
    pub fn code(&self) -> &str {
        match self {
            Self::Auth(_) => "auth",
            Self::ChannelNotFound(_) | Self::UserNotFound(_) => "not_found",
            Self::Api(_) | Self::Http(_) => "api",
            Self::Config(_) => "config",
            Self::RateLimit(_) => "rate_limit",
            Self::Io(_) | Self::Other(_) => "generic",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Auth(_) => 2,
            Self::ChannelNotFound(_) | Self::UserNotFound(_) => 3,
            Self::RateLimit(_) => 4,
            _ => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, SlackCliError>;
