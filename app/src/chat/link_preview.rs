//! V108-C25 Link cards (1.52): Twitch clips, VODs and channels from our own lookups; trusted
//! sites from our own resolver (V110-U70); then Chatterino's resolver (D8) for the rest or
//! when those fail. V111-54 Settings > Chat > Link previews from: "Chatterino" skips our own
//! resolver, Twitch links keep their lookups. One answer per link is shared
//! by everyone asking at once; good answers last 30 minutes (a channel's 2, it goes live),
//! misses 2, so a hiccup doesn't stick for the whole run.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cyex_chat::model::LinkTarget;
use cyex_settings::Partition;
pub use cyex_settings::schema::LinkResolver;
use cyex_chat::tokenize::detect_link;
use cyex_twitch::channel_input::{ChannelQuery, parse};
use cyex_twitch::preview::{self, LinkPreview, PreviewKind};
use cyex_twitch::embed::{self, Next};
use cyex_twitch::resolver::LinkInfo;
use cyex_twitch::{Http, gql, resolver};
use tauri::{AppHandle, Manager};
use tokio::sync::OnceCell;

use crate::settings::Settings;

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

/// V110-F96 What a lookup found, and whether a service was down while it looked: such a miss
/// is kept for seconds, not minutes, so a link asked after the service heals is asked afresh.
#[derive(Clone)]
pub struct Found {
    card: Option<LinkPreview>,
    down: bool,
}

const DOWN_TTL: Duration = Duration::from_secs(15);

fn found_ttl(f: &Found) -> Duration {
    if f.down && f.card.is_none() { DOWN_TTL } else { ttl(&f.card) }
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

/// V111-54 The three places a card can come from, so the order can be tested without them.
trait Sources {
    async fn twitch(&self, link: &TwitchLink) -> Option<LinkPreview>;
    async fn ours(&self, url: &str) -> Next;
    async fn chatterino(&self, url: &str) -> cyex_twitch::Result<Option<LinkInfo>>;
}

struct Live<'a>(&'a Http);

impl Sources for Live<'_> {
    async fn twitch(&self, link: &TwitchLink) -> Option<LinkPreview> {
        twitch(self.0, link).await
    }
    async fn ours(&self, url: &str) -> Next {
        embed::lookup(self.0, url).await
    }
    async fn chatterino(&self, url: &str) -> cyex_twitch::Result<Option<LinkInfo>> {
        resolver::resolve(self.0, url).await
    }
}

/// Twitch first, then our own resolver for trusted sites, Chatterino's after. `None` when
/// none has anything. V111-54 With `LinkResolver::Chatterino` our own resolver is never asked.
async fn load(src: &impl Sources, url: &str, via: LinkResolver) -> Found {
    let hit = |card| Found { card, down: false };
    if let Some(link) = twitch_link(url)
        && let Some(p) = src.twitch(&link).await
    {
        return hit(Some(p));
    }
    let mut down = false;
    if via == LinkResolver::Cyex && asks_worker(url) {
        match src.ours(url).await {
            Next::Use(p) => return hit(Some(*p)),
            // "Nothing to show" is an answer, not a failure.
            Next::Nothing => return hit(None),
            Next::Fallback => {}
            Next::Down => down = true,
        }
    }
    match src.chatterino(url).await {
        Ok(Some(info)) => hit(preview::from_resolver(url, info)),
        Ok(None) => hit(None),
        Err(e) => Found {
            card: None,
            down: down || resolver::is_down(&e),
        },
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

/// V111-54 Which resolver the settings pick, read on every ask so a switch applies at once.
pub fn via(app: &AppHandle) -> LinkResolver {
    app.state::<Settings>().with(|s| {
        s.get(Partition::Chat)
            .get("linkResolver")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    })
}

/// V111-54 Answers are kept per resolver, so a switch never shows the other one's old card.
fn key(via: LinkResolver, url: &str) -> String {
    match via {
        LinkResolver::Cyex => format!("cyex {url}"),
        LinkResolver::Chatterino => format!("chatterino {url}"),
    }
}

/// The card for `url`, cached.
pub async fn get(http: &Http, url: String, via: LinkResolver) -> Option<LinkPreview> {
    static CACHE: std::sync::OnceLock<Cache<Found>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| Cache::new(CAP, found_ttl));
    let live = Live(http);
    cache
        .get_at(&key(via, &url), Instant::now(), || load(&live, &url, via))
        .await
        .card
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

    #[test]
    fn v110_f96_a_miss_while_a_service_was_down_is_kept_briefly() {
        let miss = |down| Found { card: None, down };
        assert_eq!(found_ttl(&miss(false)), MISS_TTL);
        assert_eq!(found_ttl(&miss(true)), DOWN_TTL);
        assert!(DOWN_TTL < MISS_TTL && DOWN_TTL < cyex_twitch::breaker::PAUSE);
    }

    /// V111-54 Canned answers; remembers who was asked.
    struct Fake {
        ours: fn() -> Next,
        chatterino: fn() -> cyex_twitch::Result<Option<LinkInfo>>,
        asked: Mutex<Vec<&'static str>>,
    }

    fn card(title: &str) -> LinkPreview {
        preview::from_resolver(
            "https://www.youtube.com/watch?v=a",
            LinkInfo {
                lines: vec![vec![cyex_twitch::resolver::LinkText { text: title.into(), bold: true }]],
                ..Default::default()
            },
        )
        .unwrap()
    }

    impl Fake {
        fn new(ours: fn() -> Next) -> Self {
            Self {
                ours,
                chatterino: || Ok(Some(LinkInfo {
                    lines: vec![vec![cyex_twitch::resolver::LinkText { text: "From Chatterino".into(), bold: true }]],
                    ..Default::default()
                })),
                asked: Mutex::default(),
            }
        }
        fn asked(&self) -> Vec<&'static str> {
            std::mem::take(&mut self.asked.lock().unwrap())
        }
    }

    impl Sources for Fake {
        async fn twitch(&self, _: &TwitchLink) -> Option<LinkPreview> {
            self.asked.lock().unwrap().push("twitch");
            Some(card("From Twitch"))
        }
        async fn ours(&self, _: &str) -> Next {
            self.asked.lock().unwrap().push("ours");
            (self.ours)()
        }
        async fn chatterino(&self, _: &str) -> cyex_twitch::Result<Option<LinkInfo>> {
            self.asked.lock().unwrap().push("chatterino");
            (self.chatterino)()
        }
    }

    fn title(f: &Found) -> Option<&str> {
        f.card.as_ref().and_then(|c| c.title.as_deref())
    }

    #[tokio::test]
    async fn v111_54_each_resolver_mode_asks_the_right_service() {
        let yt = "https://www.youtube.com/watch?v=a";
        let src = Fake::new(|| Next::Use(Box::new(card("From cyex"))));
        // Ours: our resolver answers, Chatterino's isn't asked.
        let f = load(&src, yt, LinkResolver::Cyex).await;
        assert_eq!((title(&f), src.asked()), (Some("From cyex"), vec!["ours"]));
        // Chatterino: ours is never asked, even for a trusted link.
        let f = load(&src, yt, LinkResolver::Chatterino).await;
        assert_eq!((title(&f), src.asked()), (Some("From Chatterino"), vec!["chatterino"]));
        // Twitch links keep their own lookups in both modes.
        for via in [LinkResolver::Cyex, LinkResolver::Chatterino] {
            let f = load(&src, "https://clips.twitch.tv/Funny-AbC", via).await;
            assert_eq!((title(&f), src.asked()), (Some("From Twitch"), vec!["twitch"]));
        }
        // Untrusted links go to Chatterino's in both modes.
        for via in [LinkResolver::Cyex, LinkResolver::Chatterino] {
            let f = load(&src, "https://example.com/", via).await;
            assert_eq!((title(&f), src.asked()), (Some("From Chatterino"), vec!["chatterino"]));
        }
    }

    #[tokio::test]
    async fn v111_54_ours_falls_back_to_chatterino_and_404_does_not() {
        let yt = "https://www.youtube.com/watch?v=a";
        let src = Fake::new(|| Next::Fallback);
        let f = load(&src, yt, LinkResolver::Cyex).await;
        assert_eq!((title(&f), src.asked()), (Some("From Chatterino"), vec!["ours", "chatterino"]));
        let src = Fake::new(|| Next::Down);
        let f = load(&src, yt, LinkResolver::Cyex).await;
        assert_eq!((title(&f), f.down, src.asked()), (Some("From Chatterino"), false, vec!["ours", "chatterino"]));
        let src = Fake::new(|| Next::Nothing);
        let f = load(&src, yt, LinkResolver::Cyex).await;
        assert_eq!((title(&f), src.asked()), (None, vec!["ours"]));
        // Chatterino mode: Chatterino down is a short miss, with nothing else asked.
        let mut src = Fake::new(|| Next::Nothing);
        src.chatterino = || Err(cyex_twitch::Error::Api { status: 503, message: "paused".into() });
        let f = load(&src, yt, LinkResolver::Chatterino).await;
        assert_eq!((title(&f), f.down, src.asked()), (None, true, vec!["chatterino"]));
    }

    #[test]
    fn v111_54_cache_keys_differ_by_resolver() {
        let u = "https://www.youtube.com/watch?v=a";
        assert_ne!(key(LinkResolver::Cyex, u), key(LinkResolver::Chatterino, u));
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
