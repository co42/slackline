use crate::client::HyperConnector;
use crate::error::{Result, SlackCliError};
use chrono::{DateTime, Utc};
use serde::Serialize;
use slack_morphism::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Event filter types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventFilter {
    Message,
    Mention,
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
            "mention" => Ok(Self::Mention),
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
        EventFilter::Mention,
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
        EventFilter::Mention => event_type == "mention",
        EventFilter::Reaction => {
            event_type == "reaction_added" || event_type == "reaction_removed"
        }
        EventFilter::Dm => event_type == "dm",
        EventFilter::Channel => event_type.starts_with("channel_"),
        EventFilter::File => event_type.starts_with("file_") || event_type == "file_shared",
        EventFilter::Member => event_type == "member_joined" || event_type == "member_left",
        EventFilter::Status => event_type == "status_changed",
        EventFilter::All => true,
    })
}

// ---------------------------------------------------------------------------
// Normalized output events
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// User state passed through SlackClientEventsUserState
// ---------------------------------------------------------------------------

struct WatchState {
    filters: Vec<EventFilter>,
    channel_filter: Vec<String>,
    raw: bool,
    user_token: SlackApiToken,
    name_cache: Arc<RwLock<NameCache>>,
}

// TODO: cap cache size (LRU or periodic eviction) for long-running sessions
#[derive(Debug, Default)]
struct NameCache {
    users: HashMap<String, String>,
    channels: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Name resolution with cache
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Event → WatchEvent conversion
// ---------------------------------------------------------------------------

fn ts_to_rfc3339(event_time: &SlackDateTime) -> String {
    let dt: DateTime<Utc> = event_time.0;
    dt.to_rfc3339()
}

fn slack_ts_to_string(ts: &SlackTs) -> String {
    ts.0.clone()
}

fn is_dm_channel(channel_type: Option<&SlackChannelType>) -> bool {
    channel_type
        .map(|ct| ct.0 == "im" || ct.0 == "mpim")
        .unwrap_or(false)
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

            Some(WatchEvent {
                ts,
                event_type: event_type.to_string(),
                channel: channel_id,
                channel_name,
                user: user_id,
                user_name,
                text: msg.content.as_ref().and_then(|c| c.text.clone()),
                thread_ts: msg.origin.thread_ts.as_ref().map(slack_ts_to_string),
                subtype: msg.subtype.as_ref().and_then(|s| {
                    serde_json::to_value(s)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                }),
                ..Default::default()
            })
        }

        SlackEventCallbackBody::AppMention(mention) => {
            let channel_id = mention.channel.0.clone();
            let user_id = mention.user.0.clone();

            Some(WatchEvent {
                ts,
                event_type: "mention".to_string(),
                channel: Some(channel_id.clone()),
                channel_name: resolve_channel_name(cache, &session, &channel_id).await,
                user: Some(user_id.clone()),
                user_name: resolve_user_name(cache, &session, &user_id).await,
                text: mention.content.text.clone(),
                thread_ts: mention.origin.thread_ts.as_ref().map(slack_ts_to_string),
                ..Default::default()
            })
        }

        SlackEventCallbackBody::ReactionAdded(reaction) => {
            let user_id = reaction.user.0.clone();
            let (channel, item_ts) = extract_reaction_item(&reaction.item);
            let channel_name = if let Some(ref cid) = channel {
                resolve_channel_name(cache, &session, cid).await
            } else {
                None
            };

            Some(WatchEvent {
                ts,
                event_type: "reaction_added".to_string(),
                channel,
                channel_name,
                user: Some(user_id.clone()),
                user_name: resolve_user_name(cache, &session, &user_id).await,
                emoji: Some(reaction.reaction.0.clone()),
                item_ts,
                ..Default::default()
            })
        }

        SlackEventCallbackBody::ReactionRemoved(reaction) => {
            let user_id = reaction.user.0.clone();
            let (channel, item_ts) = extract_reaction_item(&reaction.item);
            let channel_name = if let Some(ref cid) = channel {
                resolve_channel_name(cache, &session, cid).await
            } else {
                None
            };

            Some(WatchEvent {
                ts,
                event_type: "reaction_removed".to_string(),
                channel,
                channel_name,
                user: Some(user_id.clone()),
                user_name: resolve_user_name(cache, &session, &user_id).await,
                emoji: Some(reaction.reaction.0.clone()),
                item_ts,
                ..Default::default()
            })
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

// ---------------------------------------------------------------------------
// Push events callback (bare fn — state accessed via SlackClientEventsUserState)
// ---------------------------------------------------------------------------

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
    let channel_filter = watch_state.channel_filter.clone();
    let token = watch_state.user_token.clone();
    let cache = watch_state.name_cache.clone();
    drop(state_guard);

    if let Some(watch_event) = normalize_event(&event, &cache, &client, &token).await {
        if !matches_filter(&watch_event.event_type, &filters) {
            return Ok(());
        }

        if !channel_filter.is_empty() {
            match watch_event.channel {
                Some(ref ch) if channel_filter.contains(ch) => {}
                Some(_) | None => return Ok(()),
            }
        }

        let json = serde_json::to_string(&watch_event)?;
        println!("{json}");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Main listener entry point
// ---------------------------------------------------------------------------

pub async fn listen(
    config: &crate::Config,
    events: &[EventFilter],
    channels: &[String],
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

    let watch_state = WatchState {
        filters,
        channel_filter: channels.to_vec(),
        raw,
        user_token,
        name_cache: Arc::new(RwLock::new(NameCache::default())),
    };

    let client = Arc::new(slack_morphism::SlackClient::new(
        SlackClientHyperHttpsConnector::new()?,
    ));

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
