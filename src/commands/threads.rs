use crate::client::Client;
use crate::error::{Result, SlackCliError};
use crate::output::{HumanReadable, Output};
use chrono::{DateTime, Utc};
use colored::Colorize;
use futures::StreamExt;
use futures::stream;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Slack has no public API for the sidebar "Threads" view (`subscriptions.thread.getView`
/// rejects user tokens). We rebuild it: search finds the threads the user took part in,
/// then `conversations.replies` on each parent returns the undocumented `subscribed` and
/// `last_read` fields that drive the unread state.
const SEARCH_QUERIES: [&str; 2] = ["from:me is:thread", "to:me is:thread"];
const SEARCH_PAGE_SIZE: u16 = 100;
const MAX_SEARCH_PAGES: u32 = 5;
/// `conversations.replies` is Tier 3 (about 50 calls a minute), so keep this low.
const CONCURRENCY: usize = 4;
const MAX_RETRIES: u32 = 10;
const PROGRESS_EVERY: usize = 25;

/// Earliest time the next request may start. A 429 pushes it forward for every
/// worker, so they wait together instead of each burning its own retries.
struct RateGate(Mutex<Instant>);

impl RateGate {
    fn new() -> Self {
        Self(Mutex::new(Instant::now()))
    }

    async fn wait(&self) {
        let not_before = *self.0.lock().unwrap();
        let now = Instant::now();
        if not_before > now {
            tokio::time::sleep(not_before - now).await;
        }
    }

    fn back_off(&self, wait: Duration) {
        let mut not_before = self.0.lock().unwrap();
        *not_before = (*not_before).max(Instant::now() + wait);
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    ok: bool,
    error: Option<String>,
    messages: Option<SearchMessages>,
}

#[derive(Debug, Deserialize)]
struct SearchMessages {
    matches: Vec<SearchMatch>,
    paging: Option<SearchPaging>,
}

#[derive(Debug, Deserialize)]
struct SearchPaging {
    pages: u32,
}

#[derive(Debug, Deserialize)]
struct SearchMatch {
    ts: String,
    permalink: String,
    channel: SearchChannel,
}

#[derive(Debug, Deserialize)]
struct SearchChannel {
    id: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepliesResponse {
    ok: bool,
    #[serde(default)]
    messages: Vec<ThreadParent>,
}

#[derive(Debug, Deserialize)]
struct ThreadParent {
    ts: String,
    text: Option<String>,
    user: Option<String>,
    reply_count: Option<u64>,
    latest_reply: Option<String>,
    subscribed: Option<bool>,
    last_read: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Thread {
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub thread_ts: String,
    pub user: Option<String>,
    pub text: String,
    pub reply_count: u64,
    pub latest_reply: String,
    pub last_read: Option<String>,
    pub unread: bool,
    pub permalink: String,
    pub latest_reply_at: Option<DateTime<Utc>>,
}

impl HumanReadable for Thread {
    fn print_human(&self) {
        let time = self
            .latest_reply_at
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| self.latest_reply.clone());
        let channel = self.channel_name.as_deref().unwrap_or(&self.channel_id);
        let marker = if self.unread {
            " [unread]".red().to_string()
        } else {
            String::new()
        };
        println!(
            "{} #{} ({} replies){}",
            time.dimmed(),
            channel.cyan(),
            self.reply_count,
            marker
        );
        let text = self.text.replace('\n', " ");
        let text = if text.len() > 120 {
            format!("{}...", &text[..text.floor_char_boundary(120)])
        } else {
            text
        };
        println!("  {}", text);
        println!("  {}", self.permalink.dimmed());
        println!();
    }
}

/// Parent `ts` of the thread a search match belongs to. Replies carry it in the
/// permalink as `thread_ts=`; a match without it is the parent itself.
fn thread_ts_of(permalink: &str, ts: &str) -> String {
    permalink
        .split(['?', '&'])
        .find_map(|part| part.strip_prefix("thread_ts="))
        .unwrap_or(ts)
        .to_string()
}

/// Slack timestamps are `seconds.micros` with a fixed 6-digit fraction, but compare
/// them numerically to stay safe.
fn ts_after(a: &str, b: &str) -> bool {
    let parse = |s: &str| s.parse::<f64>().unwrap_or(0.0);
    parse(a) > parse(b)
}

fn is_unread(parent: &ThreadParent) -> bool {
    match (&parent.latest_reply, &parent.last_read) {
        (Some(latest), Some(read)) => ts_after(latest, read),
        (Some(_), None) => true,
        _ => false,
    }
}

async fn get_json<T: for<'de> Deserialize<'de>>(
    http: &reqwest::Client,
    gate: &RateGate,
    token: &str,
    url: &str,
) -> Result<T> {
    for _ in 0..MAX_RETRIES {
        gate.wait().await;
        let response = http
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| SlackCliError::Api(e.to_string()))?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let wait = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(5);
            gate.back_off(Duration::from_secs(wait));
            continue;
        }

        return response
            .json()
            .await
            .map_err(|e| SlackCliError::Api(e.to_string()));
    }
    Err(SlackCliError::RateLimit(url.to_string()))
}

/// Candidate threads keyed by (channel, thread_ts), with the channel name.
async fn find_candidates(
    http: &reqwest::Client,
    gate: &RateGate,
    token: &str,
    date_filter: &str,
) -> Result<BTreeMap<(String, String), Option<String>>> {
    let mut candidates = BTreeMap::new();
    for base in SEARCH_QUERIES {
        let query = format!("{} {}", base, date_filter);
        for page in 1..=MAX_SEARCH_PAGES {
            let url = format!(
                "https://slack.com/api/search.messages?query={}&count={}&page={}&sort=timestamp&sort_dir=desc",
                urlencoding::encode(&query),
                SEARCH_PAGE_SIZE,
                page
            );
            let response: SearchResponse = get_json(http, gate, token, &url).await?;
            if !response.ok {
                return Err(SlackCliError::Api(
                    response
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string()),
                ));
            }
            let Some(messages) = response.messages else {
                break;
            };
            for m in messages.matches {
                let thread_ts = thread_ts_of(&m.permalink, &m.ts);
                candidates
                    .entry((m.channel.id, thread_ts))
                    .or_insert(m.channel.name);
            }
            if page >= messages.paging.map(|p| p.pages).unwrap_or(0) {
                break;
            }
        }
    }
    Ok(candidates)
}

/// List the threads you follow, like the sidebar "Threads" view
pub async fn threads(
    client: &Client,
    output: &Output,
    since: &str,
    until: Option<&str>,
    show_all: bool,
) -> Result<()> {
    let token = client.token();
    let http = reqwest::Client::new();
    let gate = RateGate::new();

    let since_ts = crate::timeparse::parse_time_expr(since).map_err(SlackCliError::Config)?;
    let until_ts = until
        .map(crate::timeparse::parse_time_expr)
        .transpose()
        .map_err(SlackCliError::Config)?;
    // `after:` and `before:` are exclusive and day-granular, so widen each by one day.
    let search_day = |ts: &str, shift: i64| {
        let secs = ts.parse::<f64>().unwrap_or(0.0) as i64;
        DateTime::from_timestamp(secs + shift, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d")
            .to_string()
    };
    let mut date_filter = format!("after:{}", search_day(&since_ts, -86_400));
    // Your message in a thread is never newer than its latest reply.
    if let Some(until) = until_ts.as_deref() {
        date_filter.push_str(&format!(" before:{}", search_day(until, 86_400)));
    }

    output.status("Searching for threads you took part in...");
    let mut candidates = find_candidates(&http, &gate, token, &date_filter).await?;
    // A thread started after `until` cannot have its latest reply before it.
    if let Some(until) = until_ts.as_deref() {
        candidates.retain(|(_, thread_ts), _| !ts_after(thread_ts, until));
    }
    let total = candidates.len();
    output.status(&format!("Checking read state of {} threads...", total));

    let done = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);

    let results: Vec<Option<Thread>> = stream::iter(candidates)
        .map(|((channel_id, thread_ts), channel_name)| {
            let (http, gate, done, failed) = (&http, &gate, &done, &failed);
            let since_ts = &since_ts;
            let until_ts = until_ts.as_deref();
            async move {
                let url = format!(
                    "https://slack.com/api/conversations.replies?channel={}&ts={}&limit=1",
                    channel_id, thread_ts
                );
                let response: Option<RepliesResponse> =
                    get_json(http, gate, token, &url).await.ok();
                let count = done.fetch_add(1, Ordering::Relaxed) + 1;
                if count % PROGRESS_EVERY == 0 && count < total {
                    output.status(&format!("  {}/{} threads checked", count, total));
                }
                let response = response.filter(|r| r.ok).or_else(|| {
                    failed.fetch_add(1, Ordering::Relaxed);
                    None
                })?;
                let parent = response.messages.into_iter().next()?;
                if parent.subscribed != Some(true) {
                    return None;
                }
                let latest_reply = parent.latest_reply.clone()?;
                if !ts_after(&latest_reply, since_ts)
                    || until_ts.is_some_and(|until| ts_after(&latest_reply, until))
                {
                    return None;
                }
                let unread = is_unread(&parent);
                let latest_secs = latest_reply.parse::<f64>().unwrap_or(0.0) as i64;
                Some(Thread {
                    permalink: format!(
                        "https://slack.com/archives/{}/p{}",
                        channel_id,
                        parent.ts.replace('.', "")
                    ),
                    channel_id,
                    channel_name,
                    thread_ts: parent.ts,
                    user: parent.user,
                    text: parent.text.unwrap_or_default(),
                    reply_count: parent.reply_count.unwrap_or(0),
                    latest_reply,
                    last_read: parent.last_read,
                    unread,
                    latest_reply_at: DateTime::from_timestamp(latest_secs, 0),
                })
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;

    let failed = failed.into_inner();
    if failed > 0 {
        output.status(&format!(
            "Warning: could not read {} of {} threads (rate limit or no access), the list may be incomplete",
            failed, total
        ));
    }

    let mut threads: Vec<Thread> = results
        .into_iter()
        .flatten()
        .filter(|t| show_all || t.unread)
        .collect();
    threads.sort_by(|a, b| b.latest_reply.cmp(&a.latest_reply));

    let title = if show_all {
        "Followed Threads"
    } else {
        "Unread Threads"
    };
    output.print_list(&threads, title);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent(latest_reply: Option<&str>, last_read: Option<&str>) -> ThreadParent {
        ThreadParent {
            ts: "1700000000.000100".to_string(),
            text: None,
            user: None,
            reply_count: Some(1),
            latest_reply: latest_reply.map(String::from),
            subscribed: Some(true),
            last_read: last_read.map(String::from),
        }
    }

    #[test]
    fn thread_ts_comes_from_the_reply_permalink() {
        let permalink =
            "https://x.slack.com/archives/C1/p1794600246693569?thread_ts=1794576240.407429&cid=C1";
        assert_eq!(
            thread_ts_of(permalink, "1794600246.693569"),
            "1794576240.407429"
        );
    }

    #[test]
    fn thread_ts_falls_back_to_the_match_for_a_parent() {
        let permalink = "https://x.slack.com/archives/C1/p1794576240407429";
        assert_eq!(
            thread_ts_of(permalink, "1794576240.407429"),
            "1794576240.407429"
        );
    }

    #[test]
    fn thread_is_unread_when_a_reply_is_newer_than_last_read() {
        assert!(is_unread(&parent(
            Some("1700000200.000000"),
            Some("1700000100.000000")
        )));
    }

    #[test]
    fn thread_is_read_when_last_read_reaches_the_latest_reply() {
        assert!(!is_unread(&parent(
            Some("1700000200.000000"),
            Some("1700000200.000000")
        )));
        // Slack can move last_read past latest_reply (edits, "also send to channel").
        assert!(!is_unread(&parent(
            Some("1700000200.000000"),
            Some("1700000300.000000")
        )));
    }

    #[test]
    fn thread_without_replies_is_never_unread() {
        assert!(!is_unread(&parent(None, None)));
    }
}
