use crate::config::Config;
use crate::error::Result;
use slack_morphism::prelude::*;
use std::sync::Arc;

pub type HyperConnector = SlackClientHyperHttpsConnector;

pub struct Client {
    inner: Arc<SlackHyperClient>,
    token: SlackApiToken,
    token_str: String,
}

impl Client {
    pub fn new(config: &Config) -> Result<Self> {
        let connector = SlackClientHyperHttpsConnector::new()?;
        let inner = Arc::new(slack_morphism::SlackClient::new(connector));
        let token = SlackApiToken::new(config.token.clone().into());
        let token_str = config.token.clone();

        Ok(Self {
            inner,
            token,
            token_str,
        })
    }

    /// Get raw token string (for APIs not in slack-morphism)
    pub fn token(&self) -> &str {
        &self.token_str
    }

    pub fn session(&self) -> SlackClientSession<'_, HyperConnector> {
        self.inner.open_session(&self.token)
    }

    /// Test authentication and return user info
    pub async fn auth_test(&self) -> Result<SlackApiAuthTestResponse> {
        let session = self.session();
        let response = session.auth_test().await?;
        Ok(response)
    }
}
