use crate::client::HyperConnector;
use crate::error::{Result, SlackCliError};
use chrono::{DateTime, Utc};
use serde::Serialize;
use slack_morphism::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventFilter {
    Message,
    Reaction,
    Dm,
    Channel,
    File,
    Member,
    Status,
    All,
}

impl EventFilter {
    pub fn parse(s: &str) -> std::result::Result<Self, String> {
        match s.to_lowercase().as_str() {
            "message" => Ok(Self::Message),
            "reaction" => Ok(Self::Reaction),
            "dm" => Ok(Self::Dm),
            "channel" => Ok(Self::Channel),
            "file" => Ok(Self::File),
            "member" => Ok(Self::Member),
            "status" => Ok(Self::Status),
            "all" => Ok(Self::All),
            other => Err(format!("unknown event type: {other}")),
        }
    }
}

fn default_filters() -> Vec<EventFilter> {
    vec![
        EventFilter::Message,
        EventFilter::Dm,
        EventFilter::Reaction,
    ]
}

fn matches_filter(event_type: &str, filters: &[EventFilter]) -> bool {
    if filters.contains(&EventFilter::All) {
        return true;
    }
    filters.iter().any(|f| match f {
        EventFilter::Message => event_type == "message",
        EventFilter::Reaction => {
            event_type == "reaction_added" || event_type == "reaction_removed"
        }
        EventFilter::Dm => event_type == "dm",
        EventFilter::Channel => event_type.starts_with("channel_"),
        EventFilter::File => event_type.starts_with("file_"),
        EventFilter::Member => event_type == "member_joined" || event_type == "member_left",
        EventFilter::Status => event_type == "status_changed",
        EventFilter::All => true,
    })
}

#[derive(Debug, Default, Serialize)]
pub struct WatchEvent {
    pub ts: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
}

struct WatchState {
    filters: Arc<[EventFilter]>,
    channels: Arc<[String]>,
    exclude_channels: Arc<[String]>,
    exclude_subtypes: Arc<[String]>,
    raw: bool,
    user_token: SlackApiToken,
    name_cache: Arc<RwLock<NameCache>>,
}

/// Grows unbounded for the session lifetime. Acceptable for a CLI — a busy
/// workspace might see a few thousand unique users/channels, well within limits.
#[derive(Debug, Default)]
struct NameCache {
    users: HashMap<String, String>,
    channels: HashMap<String, String>,
}

async fn resolve_user_name(
    cache: &RwLock<NameCache>,
    session: &SlackClientSession<'_, HyperConnector>,
    user_id: &str,
) -> Option<String> {
    {
        let c = cache.read().await;
        if let Some(name) = c.users.get(user_id) {
            return Some(name.clone());
        }
    }
    let req = SlackApiUsersInfoRequest::new(SlackUserId::new(user_id.to_string()));
    if let Ok(resp) = session.users_info(&req).await {
        let name = resp
            .user
            .name
            .or(resp.user.real_name)
            .unwrap_or_default();
        cache
            .write()
            .await
            .users
            .insert(user_id.to_string(), name.clone());
        Some(name)
    } else {
        None
    }
}

async fn resolve_channel_name(
    cache: &RwLock<NameCache>,
    session: &SlackClientSession<'_, HyperConnector>,
    channel_id: &str,
) -> Option<String> {
    {
        let c = cache.read().await;
        if let Some(name) = c.channels.get(channel_id) {
            return Some(name.clone());
        }
    }
    let req =
        SlackApiConversationsInfoRequest::new(SlackChannelId::new(channel_id.to_string()));
    if let Ok(resp) = session.conversations_info(&req).await {
        let name = resp.channel.name.unwrap_or_default();
        let display = format!("#{name}");
        cache
            .write()
            .await
            .channels
            .insert(channel_id.to_string(), display.clone());
        Some(display)
    } else {
        None
    }
}

fn ts_to_rfc3339(event_time: &SlackDateTime) -> String {
    let dt: DateTime<Utc> = event_time.0;
    dt.to_rfc3339()
}

fn slack_ts_to_string(ts: &SlackTs) -> String {
    ts.0.clone()
}

/// Cheaply extract event type(s) and channel ID from a raw event without API calls.
/// For Message events, returns both "message" and "dm" when channel_type is unknown.
fn extract_event_info(event: &SlackEventCallbackBody) -> (Vec<&'static str>, Option<&str>) {
    match event {
        SlackEventCallbackBody::Message(msg) => {
            let channel = msg.origin.channel.as_ref().map(|c| c.0.as_str());
            if is_dm_channel(msg.origin.channel_type.as_ref()) {
                (vec!["dm"], channel)
            } else if msg.origin.channel_type.is_some() {
                (vec!["message"], channel)
            } else {
                (vec!["message", "dm"], channel)
            }
        }
        SlackEventCallbackBody::ReactionAdded(r) => {
            let ch = match &r.item {
                SlackReactionsItem::Message(m) => m.origin.channel.as_ref().map(|c| c.0.as_str()),
                SlackReactionsItem::File(_) => None,
            };
            (vec!["reaction_added"], ch)
        }
        SlackEventCallbackBody::ReactionRemoved(r) => {
            let ch = match &r.item {
                SlackReactionsItem::Message(m) => m.origin.channel.as_ref().map(|c| c.0.as_str()),
                SlackReactionsItem::File(_) => None,
            };
            (vec!["reaction_removed"], ch)
        }
        SlackEventCallbackBody::MemberJoinedChannel(e) => {
            (vec!["member_joined"], Some(e.channel.0.as_str()))
        }
        SlackEventCallbackBody::MemberLeftChannel(e) => {
            (vec!["member_left"], Some(e.channel.0.as_str()))
        }
        SlackEventCallbackBody::FileShared(e) => {
            (vec!["file_shared"], Some(e.channel_id.0.as_str()))
        }
        SlackEventCallbackBody::UserStatusChanged(_) => (vec!["status_changed"], None),
        _ => (vec!["unknown"], None),
    }
}

fn could_match_filter(possible_types: &[&str], filters: &[EventFilter]) -> bool {
    if filters.contains(&EventFilter::All) {
        return true;
    }
    possible_types.iter().any(|t| matches_filter(t, filters))
}

fn is_dm_channel(channel_type: Option<&SlackChannelType>) -> bool {
    channel_type
        .map(|ct| ct.0 == "im" || ct.0 == "mpim")
        .unwrap_or(false)
}

async fn normalize_reaction(
    event_type: &str,
    ts: String,
    user: &SlackUserId,
    reaction: &SlackReactionName,
    item: &SlackReactionsItem,
    cache: &RwLock<NameCache>,
    session: &SlackClientSession<'_, HyperConnector>,
) -> WatchEvent {
    let user_id = user.0.clone();
    let (channel, item_ts) = extract_reaction_item(item);
    let channel_name = if let Some(ref cid) = channel {
        resolve_channel_name(cache, session, cid).await
    } else {
        None
    };
    let text = match (&channel, &item_ts) {
        (Some(ch), Some(its)) => fetch_message_text(session, ch, its).await,
        _ => None,
    };
    WatchEvent {
        ts,
        event_type: event_type.to_string(),
        channel,
        channel_name,
        user: Some(user_id.clone()),
        user_name: resolve_user_name(cache, session, &user_id).await,
        text,
        emoji: Some(reaction.0.clone()),
        item_ts,
        ..Default::default()
    }
}

async fn normalize_event(
    event: &SlackPushEventCallback,
    cache: &RwLock<NameCache>,
    client: &Arc<SlackHyperClient>,
    token: &SlackApiToken,
) -> Option<WatchEvent> {
    let session = client.open_session(token);
    let ts = ts_to_rfc3339(&event.event_time);

    match &event.event {
        SlackEventCallbackBody::Message(msg) => {
            let channel_id = msg.origin.channel.as_ref().map(|c| c.0.clone());
            let user_id = msg.sender.user.as_ref().map(|u| u.0.clone());
            let is_dm = is_dm_channel(msg.origin.channel_type.as_ref());
            let event_type = if is_dm { "dm" } else { "message" };

            let channel_name = if let Some(ref cid) = channel_id {
                if !is_dm {
                    resolve_channel_name(cache, &session, cid).await
                } else {
                    None
                }
            } else {
                None
            };

            let user_name = if let Some(ref uid) = user_id {
                resolve_user_name(cache, &session, uid).await
            } else {
                None
            };

            // For message_changed, text lives in the nested `message` field
            let text = msg
                .message
                .as_ref()
                .and_then(|m| m.content.as_ref())
                .and_then(|c| c.text.clone())
                .or_else(|| msg.content.as_ref().and_then(|c| c.text.clone()));

            Some(WatchEvent {
                ts,
                event_type: event_type.to_string(),
                channel: channel_id,
                channel_name,
                user: user_id,
                user_name,
                text,
                thread_ts: msg.origin.thread_ts.as_ref().map(slack_ts_to_string),
                subtype: msg.subtype.as_ref().and_then(|s| {
                    serde_json::to_value(s)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                }),
                ..Default::default()
            })
        }

        SlackEventCallbackBody::ReactionAdded(r) => {
            Some(
                normalize_reaction("reaction_added", ts, &r.user, &r.reaction, &r.item, cache, &session).await,
            )
        }

        SlackEventCallbackBody::ReactionRemoved(r) => {
            Some(
                normalize_reaction("reaction_removed", ts, &r.user, &r.reaction, &r.item, cache, &session).await,
            )
        }

        SlackEventCallbackBody::MemberJoinedChannel(e) => Some(WatchEvent {
            ts,
            event_type: "member_joined".to_string(),
            channel: Some(e.channel.0.clone()),
            channel_name: resolve_channel_name(cache, &session, &e.channel.0).await,
            user: Some(e.user.0.clone()),
            user_name: resolve_user_name(cache, &session, &e.user.0).await,
            ..Default::default()
        }),

        SlackEventCallbackBody::MemberLeftChannel(e) => Some(WatchEvent {
            ts,
            event_type: "member_left".to_string(),
            channel: Some(e.channel.0.clone()),
            channel_name: resolve_channel_name(cache, &session, &e.channel.0).await,
            user: Some(e.user.0.clone()),
            user_name: resolve_user_name(cache, &session, &e.user.0).await,
            ..Default::default()
        }),

        SlackEventCallbackBody::FileShared(e) => Some(WatchEvent {
            ts,
            event_type: "file_shared".to_string(),
            channel: Some(e.channel_id.0.clone()),
            channel_name: resolve_channel_name(cache, &session, &e.channel_id.0).await,
            user: Some(e.user_id.0.clone()),
            user_name: resolve_user_name(cache, &session, &e.user_id.0).await,
            file_id: Some(e.file_id.0.clone()),
            ..Default::default()
        }),

        SlackEventCallbackBody::UserStatusChanged(e) => Some(WatchEvent {
            ts,
            event_type: "status_changed".to_string(),
            user: Some(e.user.id.0.clone()),
            user_name: e.user.name.clone(),
            text: e
                .user
                .profile
                .as_ref()
                .and_then(|p| p.status_text.clone()),
            emoji: e
                .user
                .profile
                .as_ref()
                .and_then(|p| p.status_emoji.as_ref().map(|em| em.0.clone())),
            ..Default::default()
        }),

        _ => Some(WatchEvent {
            ts,
            event_type: "unknown".to_string(),
            ..Default::default()
        }),
    }
}

async fn fetch_message_text(
    session: &SlackClientSession<'_, HyperConnector>,
    channel: &str,
    ts: &str,
) -> Option<String> {
    let req = SlackApiConversationsHistoryRequest::new()
        .with_channel(SlackChannelId::new(channel.to_string()))
        .with_latest(SlackTs::new(ts.to_string()))
        .with_oldest(SlackTs::new(ts.to_string()))
        .with_inclusive(true)
        .with_limit(1);
    session
        .conversations_history(&req)
        .await
        .ok()
        .and_then(|resp| resp.messages.into_iter().next())
        .and_then(|msg| msg.content.text)
}

fn extract_reaction_item(item: &SlackReactionsItem) -> (Option<String>, Option<String>) {
    match item {
        SlackReactionsItem::Message(msg) => {
            let channel = msg.origin.channel.as_ref().map(|c| c.0.clone());
            let ts = Some(msg.origin.ts.0.clone());
            (channel, ts)
        }
        SlackReactionsItem::File(_) => (None, None),
    }
}

/// Fetch all channel IDs the current user is a member of.
async fn fetch_my_channels(client: &crate::client::Client) -> Result<Vec<String>> {
    let session = client.session();
    let mut channels = Vec::new();
    let mut cursor = None;

    loop {
        let mut req = SlackApiUsersConversationsRequest::new()
            .with_types(vec![
                SlackConversationType::Public,
                SlackConversationType::Private,
                SlackConversationType::Mpim,
                SlackConversationType::Im,
            ])
            .with_exclude_archived(true)
            .with_limit(200);
        if let Some(c) = cursor {
            req = req.with_cursor(c);
        }
        let resp = session.users_conversations(&req).await?;
        for ch in &resp.channels {
            channels.push(ch.id.0.clone());
        }
        match resp.response_metadata.and_then(|m| m.next_cursor) {
            Some(c) if !c.0.is_empty() => cursor = Some(c),
            _ => break,
        }
    }

    eprintln!("watching {} channels", channels.len());
    Ok(channels)
}

async fn push_events_handler(
    event: SlackPushEventCallback,
    client: Arc<SlackHyperClient>,
    states: SlackClientEventsUserState,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state_guard = states.read().await;
    let watch_state = state_guard
        .get_user_state::<WatchState>()
        .expect("WatchState not found in user state");

    if watch_state.raw {
        let json = serde_json::to_string(&event)?;
        println!("{json}");
        return Ok(());
    }

    let filters = watch_state.filters.clone();
    let channels = watch_state.channels.clone();
    let exclude_channels = watch_state.exclude_channels.clone();
    let exclude_subtypes = watch_state.exclude_subtypes.clone();
    let token = watch_state.user_token.clone();
    let cache = watch_state.name_cache.clone();
    drop(state_guard);

    // Cheap pre-filter: extract event type and channel from the raw event
    // without any API calls, and skip events that can't match.
    let (possible_types, channel_id) = extract_event_info(&event.event);

    if !could_match_filter(&possible_types, &filters) {
        return Ok(());
    }

    if let Some(ch) = channel_id {
        if !channels.is_empty() && !channels.iter().any(|c| c == ch) {
            return Ok(());
        }
        if exclude_channels.iter().any(|c| c == ch) {
            return Ok(());
        }
    }

    // Expensive path: resolve names via API calls.
    if let Some(watch_event) = normalize_event(&event, &cache, &client, &token).await {
        // Exact filter check (resolves the Message-vs-DM ambiguity).
        if !matches_filter(&watch_event.event_type, &filters) {
            return Ok(());
        }

        if let Some(ref subtype) = watch_event.subtype
            && exclude_subtypes.contains(subtype)
        {
            return Ok(());
        }

        let json = serde_json::to_string(&watch_event)?;
        println!("{json}");
    }

    Ok(())
}

/// Connect to Slack Socket Mode and stream events as JSONL to stdout.
pub async fn listen(
    config: &crate::Config,
    events: &[EventFilter],
    channels: &[String],
    exclude_channels: &[String],
    all_channels: bool,
    exclude_subtypes: &[String],
    raw: bool,
) -> Result<()> {
    let app_token_str = config.app_token.as_deref().ok_or_else(|| {
        SlackCliError::Config(
            "SLACK_APP_TOKEN required for watch mode. Create a Socket Mode app token (xapp-...) \
             in your Slack app settings under Basic Information → App-Level Tokens."
                .to_string(),
        )
    })?;

    if !app_token_str.starts_with("xapp-") {
        return Err(SlackCliError::Config(
            "SLACK_APP_TOKEN must start with 'xapp-'. App-level tokens are different from bot/user tokens."
                .to_string(),
        ));
    }

    let filters = if events.is_empty() {
        default_filters()
    } else {
        events.to_vec()
    };

    let user_token = SlackApiToken::new(config.token.clone().into());

    let api_client = crate::client::Client::new(config)?;

    let channels = if !channels.is_empty() {
        api_client
            .resolve_channels(channels)
            .await?
            .into_iter()
            .map(|id| id.0)
            .collect()
    } else if all_channels {
        Vec::new()
    } else {
        eprintln!("fetching your channel list...");
        fetch_my_channels(&api_client).await?
    };

    let exclude_channels: Vec<String> = if !exclude_channels.is_empty() {
        api_client
            .resolve_channels(exclude_channels)
            .await?
            .into_iter()
            .map(|id| id.0)
            .collect()
    } else {
        Vec::new()
    };

    let watch_state = WatchState {
        filters: filters.into(),
        channels: channels.into(),
        exclude_channels: exclude_channels.into(),
        exclude_subtypes: exclude_subtypes.to_vec().into(),
        raw,
        user_token,
        name_cache: Arc::new(RwLock::new(NameCache::default())),
    };

    let client = Arc::clone(api_client.inner());

    let socket_mode_callbacks =
        SlackSocketModeListenerCallbacks::new().with_push_events(push_events_handler);

    let listener_environment = Arc::new(
        SlackClientEventsListenerEnvironment::new(client.clone())
            .with_error_handler(|err, _client, _states| {
                eprintln!("socket mode error: {err}");
                http::StatusCode::OK
            })
            .with_user_state(watch_state),
    );

    let socket_mode_listener = SlackClientSocketModeListener::new(
        &SlackClientSocketModeConfig::new(),
        listener_environment,
        socket_mode_callbacks,
    );

    let app_token = SlackApiToken::new(app_token_str.to_string().into());
    socket_mode_listener
        .listen_for(&app_token)
        .await
        .map_err(|e| SlackCliError::Api(format!("failed to connect socket mode: {e}")))?;

    eprintln!("connected, streaming events... (ctrl-c to stop)");

    tokio::select! {
        _ = socket_mode_listener.serve() => {}
        _ = tokio::signal::ctrl_c() => {
            eprintln!("interrupted, shutting down");
        }
    }

    Ok(())
}
