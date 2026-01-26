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

    #[error(transparent)]
    Http(#[from] slack_morphism::errors::SlackClientError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, SlackCliError>;
