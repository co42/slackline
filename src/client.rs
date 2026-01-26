use crate::config::Config;
use crate::error::Result;
use slack_morphism::prelude::*;
use std::sync::Arc;

pub type HyperConnector = SlackClientHyperHttpsConnector;

pub struct Client {
    inner: Arc<SlackHyperClient>,
    token: SlackApiToken,
}

impl Client {
    pub fn new(config: &Config) -> Result<Self> {
        let connector = SlackClientHyperHttpsConnector::new()?;
        let inner = Arc::new(slack_morphism::SlackClient::new(connector));
        let token = SlackApiToken::new(config.token.clone().into());

        Ok(Self { inner, token })
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
