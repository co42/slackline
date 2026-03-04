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

    /// Resolve a channel name or ID to a SlackChannelId.
    /// Accepts: `C1RCG46LS`, `#general`, `general`
    pub async fn resolve_channel(&self, channel: &str) -> Result<SlackChannelId> {
        // If it looks like a channel ID, use it directly
        if (channel.starts_with('C') || channel.starts_with('D') || channel.starts_with('G'))
            && !channel.contains(|c: char| c.is_lowercase())
        {
            return Ok(SlackChannelId::new(channel.to_string()));
        }

        let name = channel.strip_prefix('#').unwrap_or(channel);
        let session = self.session();
        let mut cursor = None;
        loop {
            let mut req = SlackApiConversationsListRequest::new()
                .with_limit(200)
                .with_exclude_archived(true);
            if let Some(c) = cursor {
                req = req.with_cursor(c);
            }
            let resp = session.conversations_list(&req).await?;
            for ch in &resp.channels {
                if ch.name.as_deref() == Some(name) {
                    return Ok(ch.id.clone());
                }
            }
            match resp.response_metadata.and_then(|m| m.next_cursor) {
                Some(c) if !c.0.is_empty() => cursor = Some(c),
                _ => break,
            }
        }

        Err(crate::error::SlackCliError::Api(format!(
            "channel not found: {channel}"
        )))
    }
}
