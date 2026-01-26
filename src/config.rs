use crate::error::{Result, SlackCliError};

#[derive(Debug, Clone)]
pub struct Config {
    pub token: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Try to load .env file if it exists
        let _ = dotenvy::dotenv();

        let token = std::env::var("SLACK_TOKEN")
            .or_else(|_| std::env::var("SLACK_BOT_TOKEN"))
            .or_else(|_| std::env::var("SLACK_USER_TOKEN"))
            .map_err(|_| {
                SlackCliError::Config(
                    "No Slack token found. Set SLACK_TOKEN, SLACK_BOT_TOKEN, or SLACK_USER_TOKEN environment variable, or use --token flag".to_string()
                )
            })?;

        Ok(Self { token })
    }

    pub fn with_token(token: String) -> Self {
        Self { token }
    }
}
