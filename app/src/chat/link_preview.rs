//! V108-C25 Link cards (1.52): Twitch clips, VODs and channels from our own lookups; trusted
//! sites from our own resolver (V110-U70); then Chatterino's resolver (D8) for the rest or
//! when those fail. One answer per link is shared
//! by everyone asking at once; good answers last 30 minutes (a channel's 2, it goes live),
//! misses 2, so a hiccup doesn't stick for the whole run.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cyex_chat::model::LinkTarget;
use cyex_chat::tokenize::detect_link;
use cyex_twitch::channel_input::{ChannelQuery, parse};
use cyex_twitch::preview::{self, LinkPreview, PreviewKind};
use cyex_twitch::embed::{self, Next};
use cyex_twitch::{Http, gql, resolver};
use tokio::sync::OnceCell;

const HIT_TTL: Duration = Duration::from_secs(30 * 60);
const LIVE_TTL: Duration = Duration::from_secs(2 * 60);
const MISS_TTL: Duration = Duration::from_secs(2 * 60);
const CAP: usize = 500;

/// How long an answer is good for.
fn ttl(p: &Option<LinkPreview>) -> Duration {
    match p {
        Some(p) if p.kind == PreviewKind::Channel => LIVE_TTL,
        Some(_) => HIT_TTL,
        None => MISS_TTL,
    }
}

/// A Twitch link we look up ourselves.
#[derive(Debug, PartialEq, Eq)]
enum TwitchLink {
    Clip(String),
    Video(String),
    Channel(String),
}

fn twitch_link(url: &str) -> Option<TwitchLink> {
    match detect_link(url).and_then(|l| l.target) {
        Some(LinkTarget::Clip { slug }) => return Some(TwitchLink::Clip(slug)),
        Some(LinkTarget::Video { id, .. }) => return Some(TwitchLink::Video(id)),
        None => {}
    }
    match parse(url)? {
        ChannelQuery::Login(l) | ChannelQuery::Either(l) if url.contains("twitch.tv/") => {
            Some(TwitchLink::Channel(l))
        }
        _ => None,
    }
}

async fn twitch(http: &Http, link: &TwitchLink) -> Option<LinkPreview> {
    match link {
        TwitchLink::Clip(slug) => gql::clip_info(http, slug)
            .await
            .ok()
            .flatten()
            .map(preview::from_clip),
        TwitchLink::Video(id) => gql::video(http, id).await.ok().flatten().map(preview::from_video),
        TwitchLink::Channel(login) => {
            let found = gql::lookup(http, login).await.ok()?;
            found
                .into_iter()
                .find(|c| c.login.eq_ignore_ascii_case(login))
                .map(preview::from_channel)
        }
    }
}

/// V110-U70 Whether our own resolver is asked: only trusted links, and never twitch.tv (those
/// have their own lookups above; the Worker doesn't handle them). Untrusted links (hover only)
/// go to Chatterino's resolver, the Worker refuses them by design.
fn asks_worker(url: &str) -> bool {
    cyex_chat::tokenize::trusted_link(url)
        && !preview::host(url).is_some_and(|h| h == "twitch.tv" || h.ends_with(".twitch.tv"))
}

/// Twitch first, then our own resolver for trusted sites, Chatterino's after. `None` when
/// none has anything.
async fn load(http: &Http, url: &str) -> Option<LinkPreview> {
    if let Some(link) = twitch_link(url)
        && let Some(p) = twitch(http, &link).await
    {
        return Some(p);
    }
    if asks_worker(url) {
        match embed::lookup(http, url).await {
            Next::Use(p) => return Some(*p),
            // "Nothing to show" is an answer, not a failure.
            Next::Nothing => return None,
            Next::Fallback => {}
        }
    }
    match resolver::resolve(http, url).await {
        Ok(Some(info)) => preview::from_resolver(url, info),
        _ => None,
    }
}

struct Slot<V> {
    cell: Arc<OnceCell<(V, Instant)>>,
    used: Instant,
}

/// Answers by key with their own lifetimes; the least recently used go first past `cap`.
pub struct Cache<V> {
    slots: Mutex<HashMap<String, Slot<V>>>,
    cap: usize,
    ttl: fn(&V) -> Duration,
}

impl<V: Clone> Cache<V> {
    pub fn new(cap: usize, ttl: fn(&V) -> Duration) -> Self {
        Self {
            slots: Mutex::default(),
            cap,
            ttl,
        }
    }

    /// The cell for `key`: a fresh one when there's none or its answer is past its time.
    fn cell(&self, key: &str, now: Instant) -> Arc<OnceCell<(V, Instant)>> {
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(slot) = slots.get_mut(key)
            && slot.cell.get().is_none_or(|(_, until)| *until > now)
        {
            slot.used = now;
            return slot.cell.clone();
        }
        if slots.len() >= self.cap
            && !slots.contains_key(key)
            && let Some(old) = slots.iter().min_by_key(|(_, s)| s.used).map(|(k, _)| k.clone())
        {
            slots.remove(&old);
        }
        let cell = Arc::new(OnceCell::new());
        slots.insert(
            key.to_owned(),
            Slot {
                cell: cell.clone(),
                used: now,
            },
        );
        cell
    }

    /// The answer for `key`, loading it once however many ask at the same time.
    pub async fn get_at<F, Fut>(&self, key: &str, now: Instant, load: F) -> V
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = V>,
    {
        let cell = self.cell(key, now);
        let ttl = self.ttl;
        cell.get_or_init(|| async move {
            let v = load().await;
            let until = now + ttl(&v);
            (v, until)
        })
        .await
        .0
        .clone()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.slots.lock().unwrap().len()
    }
}

/// The card for `url`, cached.
pub async fn get(http: &Http, url: String) -> Option<LinkPreview> {
    static CACHE: std::sync::OnceLock<Cache<Option<LinkPreview>>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| Cache::new(CAP, ttl));
    cache.get_at(&url, Instant::now(), || load(http, &url)).await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    #[test]
    fn twitch_links() {
        assert_eq!(
            twitch_link("https://clips.twitch.tv/Funny-AbC"),
            Some(TwitchLink::Clip("Funny-AbC".into()))
        );
        assert_eq!(
            twitch_link("https://www.twitch.tv/lirik/clip/Funny-AbC"),
            Some(TwitchLink::Clip("Funny-AbC".into()))
        );
        assert_eq!(
            twitch_link("https://www.twitch.tv/videos/12?t=1h"),
            Some(TwitchLink::Video("12".into()))
        );
        assert_eq!(
            twitch_link("https://www.twitch.tv/LIRIK"),
            Some(TwitchLink::Channel("lirik".into()))
        );
        assert_eq!(twitch_link("https://twitch.tv/directory"), None);
        assert_eq!(twitch_link("https://youtube.com/lirik"), None);
        assert_eq!(twitch_link("https://example.com"), None);
    }

    #[test]
    fn v110_u70_who_asks_our_resolver() {
        for u in [
            "https://www.youtube.com/watch?v=a",
            "https://youtu.be/a",
            "https://x.com/a/status/1",
            "https://i.imgur.com/a.png",
        ] {
            assert!(asks_worker(u), "{u}");
        }
        for u in [
            "https://clips.twitch.tv/Funny-AbC",
            "https://www.twitch.tv/lirik",
            "https://twitch.tv/directory",
            "http://youtube.com/watch?v=a",
            "https://youtube.com.evil.io/a",
            "https://user@youtube.com/a",
            "https://example.com/",
        ] {
            assert!(!asks_worker(u), "{u}");
        }
    }

    fn short(v: &u32) -> Duration {
        Duration::from_secs(if *v == 0 { 10 } else { 100 })
    }

    #[tokio::test]
    async fn shares_one_load() {
        let cache = Cache::new(10, short);
        let calls = AtomicU32::new(0);
        let now = Instant::now();
        let load = || async {
            calls.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            7
        };
        let (a, b) = tokio::join!(cache.get_at("k", now, load), cache.get_at("k", now, load));
        assert_eq!((a, b), (7, 7));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(cache.get_at("k", now + Duration::from_secs(50), load).await, 7);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // Past its time: loaded again.
        cache.get_at("k", now + Duration::from_secs(101), load).await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn misses_expire_sooner() {
        let cache = Cache::new(10, short);
        let now = Instant::now();
        assert_eq!(cache.get_at("k", now, || async { 0 }).await, 0);
        assert_eq!(
            cache
                .get_at("k", now + Duration::from_secs(5), || async { 1 })
                .await,
            0
        );
        assert_eq!(
            cache
                .get_at("k", now + Duration::from_secs(11), || async { 1 })
                .await,
            1
        );
    }

    #[tokio::test]
    async fn drops_least_recent() {
        let cache = Cache::new(2, short);
        let t = Instant::now();
        let s = Duration::from_secs;
        cache.get_at("a", t, || async { 1 }).await;
        cache.get_at("b", t + s(1), || async { 2 }).await;
        // `a` is used again, so `b` goes.
        cache.get_at("a", t + s(2), || async { 9 }).await;
        cache.get_at("c", t + s(3), || async { 3 }).await;
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get_at("a", t + s(4), || async { 9 }).await, 1);
        assert_eq!(cache.get_at("b", t + s(4), || async { 20 }).await, 20);
    }
}
