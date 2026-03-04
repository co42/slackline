use crate::config::Config;
use crate::error::Result;
use futures::TryStreamExt;
use slack_morphism::prelude::*;
use std::collections::HashMap;
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
        if Self::looks_like_id(channel) {
            return Ok(SlackChannelId::new(channel.to_string()));
        }

        let name = channel.strip_prefix('#').unwrap_or(channel);
        let session = self.session();
        let mut cursor = None;
        loop {
            let mut req = SlackApiConversationsListRequest::new()
                .with_limit(200)
                .with_exclude_archived(true)
                .with_types(vec![
                    SlackConversationType::Public,
                    SlackConversationType::Private,
                    SlackConversationType::Mpim,
                    SlackConversationType::Im,
                ]);
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

        Err(crate::error::SlackCliError::ChannelNotFound(
            channel.to_string(),
        ))
    }

    /// Resolve multiple channel names/IDs in a single paginated scan.
    pub async fn resolve_channels(&self, channels: &[String]) -> Result<Vec<SlackChannelId>> {
        let mut result: Vec<Option<SlackChannelId>> = vec![None; channels.len()];
        let mut names_to_find: HashMap<&str, Vec<usize>> = HashMap::new();

        for (i, channel) in channels.iter().enumerate() {
            if Self::looks_like_id(channel) {
                result[i] = Some(SlackChannelId::new(channel.clone()));
            } else {
                let name = channel.strip_prefix('#').unwrap_or(channel);
                names_to_find.entry(name).or_default().push(i);
            }
        }

        if names_to_find.is_empty() {
            return Ok(result.into_iter().map(|o| o.unwrap()).collect());
        }

        let session = self.session();
        let mut cursor = None;
        loop {
            let mut req = SlackApiConversationsListRequest::new()
                .with_limit(200)
                .with_exclude_archived(true)
                .with_types(vec![
                    SlackConversationType::Public,
                    SlackConversationType::Private,
                    SlackConversationType::Mpim,
                    SlackConversationType::Im,
                ]);
            if let Some(c) = cursor {
                req = req.with_cursor(c);
            }
            let resp = session.conversations_list(&req).await?;
            for ch in &resp.channels {
                if let Some(name) = ch.name.as_deref()
                    && let Some(indices) = names_to_find.remove(name)
                {
                    for i in indices {
                        result[i] = Some(ch.id.clone());
                    }
                }
            }
            if names_to_find.is_empty() {
                break;
            }
            match resp.response_metadata.and_then(|m| m.next_cursor) {
                Some(c) if !c.0.is_empty() => cursor = Some(c),
                _ => break,
            }
        }

        if !names_to_find.is_empty() {
            let missing: Vec<&str> = names_to_find.into_keys().collect();
            return Err(crate::error::SlackCliError::ChannelNotFound(
                missing.join(", "),
            ));
        }

        Ok(result.into_iter().map(|o| o.unwrap()).collect())
    }

    /// Get the inner SlackHyperClient for socket mode reuse.
    pub fn inner(&self) -> &Arc<SlackHyperClient> {
        &self.inner
    }

    fn looks_like_id(s: &str) -> bool {
        (s.starts_with('C')
            || s.starts_with('D')
            || s.starts_with('G')
            || s.starts_with('W'))
            && !s.contains(|c: char| c.is_lowercase())
    }

    fn looks_like_user_id(s: &str) -> bool {
        (s.starts_with('U') || s.starts_with('W'))
            && !s.contains(|c: char| c.is_lowercase())
    }

    /// Resolve a user name, @name, or ID to a SlackUserId.
    /// Accepts: `U032LQBJTH8`, `@username`, `username`
    pub async fn resolve_user(&self, user: &str) -> Result<SlackUserId> {
        if Self::looks_like_user_id(user) {
            return Ok(SlackUserId::new(user.to_string()));
        }

        let name = user.strip_prefix('@').unwrap_or(user);
        let session = self.session();
        let request = SlackApiUsersListRequest::new().with_limit(200);
        let scroller = request.scroller();
        let mut stream = scroller.to_items_stream(&session);

        while let Some(batch) = stream.try_next().await? {
            for u in batch {
                let matches_name = u.name.as_deref() == Some(name);
                let matches_display = u
                    .profile
                    .as_ref()
                    .and_then(|p| p.display_name.as_deref())
                    .is_some_and(|d| d == name);
                if matches_name || matches_display {
                    return Ok(u.id);
                }
            }
        }

        Err(crate::error::SlackCliError::UserNotFound(
            user.to_string(),
        ))
    }
}
