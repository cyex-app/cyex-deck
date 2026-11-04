//! 2.2 Live alerts: once a minute, which channels the default account follows went live.
//! Each alerts when its Twitch bell is on, unless the user picked on or off for it in
//! Settings → Notifications. With only a Helix sign-in there's no bell to read, so every
//! followed channel counts. Channels already live when the app starts stay quiet.
//! V106-25 A channel that is only in muted tabs alerts without its sound.
//! V108-C39 `live.scope` can add (or swap in) the channels that have a tab, read from the tabs'
//! own live poll (`tabs::live`), so no extra request. A card or pop-up offers to open or jump
//! to that channel's tab.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use cyex_settings::Partition;
use cyex_settings::schema::{LiveAlerts, Notifications, Tabs};
use cyex_twitch::gql::{self, ChannelCard, ChannelInfo, GameRef, LiveStream};
use tauri::{AppHandle, Manager};

use crate::auth;
use crate::cards::{self, Card, Kind as CardKind};
use crate::settings::Settings;
use crate::toasts::{self, Open, Pop};

const EVERY: Duration = Duration::from_secs(60);

#[derive(Default)]
struct Seen {
    account: Option<String>,
    /// Channel id → when its stream started (and who, for the offline line), for every live followed channel.
    live: HashMap<String, Known>,
    /// Nothing polled yet for this account.
    fresh: bool,
    /// V108-C39 Tab channels as of the last poll: id -> start time while live, and every id seen
    /// (a tab opened since is a first look, silent).
    tab_live: HashMap<String, String>,
    tab_ids: HashSet<String>,
}

/// V106-07 A live followed channel as last seen.
#[derive(Clone)]
struct Known {
    started: String,
    login: String,
    name: String,
    avatar: Option<String>,
}

#[derive(Default)]
struct State(Mutex<Seen>);

pub fn init(app: &AppHandle) {
    app.manage(State::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            poll(&app).await;
            tokio::time::sleep(EVERY).await;
        }
    });
}

async fn poll(app: &AppHandle) {
    let n: Notifications = app.state::<Settings>().read(Partition::Notifications);
    let account = auth::accounts(app).default_id;
    {
        let mut seen = lock(app);
        if !n.live.enabled || seen.account != account {
            *seen = Seen {
                account: account.clone(),
                live: HashMap::new(),
                fresh: true,
                tab_live: HashMap::new(),
                tab_ids: HashSet::new(),
            };
        }
    }
    let Some(account) = account.filter(|_| n.live.enabled) else {
        return;
    };
    let Some(live) = live_now(app, &account).await else {
        return;
    };

    let tab_cards = tab_channels(app);
    let (alerts, flips) = {
        let mut seen = lock(app);
        if seen.account.as_deref() != Some(account.as_str()) {
            return; // the default account changed while we asked
        }
        let quiet = seen.fresh && n.live.skip_at_start;
        let prior = Prior { followed: &seen.live, tab_live: &seen.tab_live, tab_ids: &seen.tab_ids };
        let alerts = wanted(&n.live, &prior, &live, &tab_cards, quiet);
        seen.tab_live = started_map(&tab_cards);
        seen.tab_ids = tab_cards.iter().map(|c| c.id.clone()).collect();
        // V106-07 Live notifications feed: quiet on the first check (Q5), no bell filter.
        let flips = if seen.fresh { Vec::new() } else { live_flips(&seen.live, &live) };
        seen.live = live
            .iter()
            .filter_map(|(c, _)| {
                let known = Known {
                    started: c.stream.as_ref()?.started_at.clone(),
                    login: c.login.clone(),
                    name: c.name.clone(),
                    avatar: c.avatar.clone(),
                };
                Some((c.id.clone(), known))
            })
            .collect();
        seen.fresh = false;
        (alerts, flips)
    };
    // The feed first: it doesn't wait on the go-live messages.
    for f in flips {
        crate::feeds::live::flip(app, f);
    }
    // Only read the messages when a card or pop-up will show them.
    let mut said = if n.cards.live || n.live.toast { messages(app, &alerts).await } else { HashMap::new() };
    for card in alerts {
        let message = said.remove(&card.id);
        alert(app, &n, card, message);
    }
}

/// Followed channels live now, each with its Twitch bell. `None` when nothing can say.
async fn live_now(app: &AppHandle, account: &str) -> Option<Vec<(ChannelCard, bool)>> {
    if let Some((token, client_id)) = auth::gql_token(app, account) {
        let http = app.state::<auth::Auth>().http.clone();
        let auth = crate::auth::gql_auth(&token, &client_id);
        match gql::followed_live_alerts(&http, auth).await {
            Ok(Some(list)) => return Some(list.into_iter().map(|a| (a.card, a.bell)).collect()),
            Ok(None) => return None,
            Err(e) => eprintln!("alerts: {e}"),
        }
    }
    let cards = crate::browse::follows::follows(app, account, false).await.ok()?;
    Some(
        cards
            .into_iter()
            .filter(|c| c.stream.is_some())
            .map(|c| (c, true))
            .collect(),
    )
}

/// Channels that went live since the last poll: new ones, and ones whose stream started
/// again. `quiet` (the first poll after start) lets nothing through.
fn news<'a>(
    before: &HashMap<String, Known>,
    now: &'a [(ChannelCard, bool)],
    quiet: bool,
) -> Vec<&'a (ChannelCard, bool)> {
    if quiet {
        return Vec::new();
    }
    now.iter()
        .filter(|(c, _)| {
            let Some(s) = &c.stream else { return false };
            !before.get(&c.id).is_some_and(|k| same_start(&k.started, &s.started_at))
        })
        .collect()
}

/// V108-F56h Whether two `started_at` texts are the same instant: the GQL and the Helix
/// fallback may write it differently (`Z` or an offset, fractions), which isn't a restart.
/// Texts that don't parse compare as text.
fn same_start(a: &str, b: &str) -> bool {
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};
    match (OffsetDateTime::parse(a, &Rfc3339), OffsetDateTime::parse(b, &Rfc3339)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// V108-C39 What the last poll knew, to tell what is new now.
struct Prior<'a> {
    followed: &'a HashMap<String, Known>,
    tab_live: &'a HashMap<String, String>,
    tab_ids: &'a HashSet<String>,
}

fn started_map(cards: &[ChannelCard]) -> HashMap<String, String> {
    cards
        .iter()
        .filter_map(|c| Some((c.id.clone(), c.stream.as_ref()?.started_at.clone())))
        .collect()
}

/// V108-C39 The channels that have a tab, with their stream when live, from the tabs' own
/// poll (up to a minute old). Empty before it first ran.
fn tab_channels(app: &AppHandle) -> Vec<ChannelCard> {
    let ids: HashSet<String> = app.state::<Settings>().read::<Tabs>(Partition::Tabs).channel_ids().into_iter().collect();
    crate::tabs::live(app).iter().filter(|c| ids.contains(&c.id)).map(tab_card).collect()
}

fn tab_card(c: &ChannelInfo) -> ChannelCard {
    ChannelCard {
        id: c.id.clone(),
        login: c.login.clone(),
        name: c.name.clone(),
        avatar: c.avatar.clone(),
        stream: c.stream.as_ref().map(|s| LiveStream {
            title: s.title.clone(),
            viewers: s.viewers,
            started_at: s.started_at.clone(),
            rerun: s.rerun,
            game: s.game.as_ref().map(|name| GameRef { id: String::new(), slug: String::new(), name: name.clone() }),
            tags: Vec::new(),
        }),
        last_live: None,
        last_game: None,
        last_title: None,
    }
}

/// V108-C39 Who alerts now: followed channels by the bell (when the scope has them), channels
/// with a tab (when it has those; no bell, so on unless turned off for that channel), each
/// once. A tab that opened since the last poll is a first look and stays quiet, so opening a
/// live channel doesn't alert. A channel the followed side already alerted for isn't repeated
/// when the tabs' poll catches up a minute later.
fn wanted(
    live: &LiveAlerts,
    prior: &Prior<'_>,
    followed: &[(ChannelCard, bool)],
    tabs: &[ChannelCard],
    quiet: bool,
) -> Vec<ChannelCard> {
    let mut out: Vec<ChannelCard> = Vec::new();
    if live.scope.followed() {
        out.extend(
            news(prior.followed, followed, quiet)
                .into_iter()
                .filter(|(c, bell)| live.wants(&c.id, *bell))
                .map(|(c, _)| c.clone()),
        );
    }
    if live.scope.tabs() && !quiet {
        for c in tabs {
            let Some(s) = &c.stream else { continue };
            let known = prior.tab_ids.contains(&c.id);
            let new = !prior.tab_live.get(&c.id).is_some_and(|t| same_start(t, &s.started_at));
            let done = out.iter().any(|o| o.id == c.id) || {
                // The followed side saw this very stream earlier and (if its scope is on) alerted.
                live.scope.followed()
                    && prior.followed.get(&c.id).is_some_and(|k| same_start(&k.started, &s.started_at))
                    && followed.iter().any(|(f, bell)| f.id == c.id && live.wants(&f.id, *bell))
            };
            if known && new && !done && live.wants(&c.id, true) {
                out.push(c.clone());
            }
        }
    }
    out
}

/// V106-07 Streams that started (new, or started again) or ended since the last check.
fn live_flips(before: &HashMap<String, Known>, now: &[(ChannelCard, bool)]) -> Vec<crate::feeds::live::Flip> {
    let mut flips: Vec<crate::feeds::live::Flip> = now
        .iter()
        .filter_map(|(c, _)| {
            let s = c.stream.as_ref()?;
            (!before.get(&c.id).is_some_and(|k| same_start(&k.started, &s.started_at))).then(|| crate::feeds::live::Flip {
                id: c.id.clone(),
                login: c.login.clone(),
                name: c.name.clone(),
                avatar: c.avatar.clone(),
                live: true,
                title: Some(s.title.clone()),
                game: s.game.as_ref().map(|g| g.name.clone()),
                started: Some(s.started_at.clone()),
            })
        })
        .collect();
    for (id, k) in before {
        if !now.iter().any(|(c, _)| &c.id == id && c.stream.is_some()) {
            flips.push(crate::feeds::live::Flip {
                id: id.clone(),
                login: k.login.clone(),
                name: k.name.clone(),
                avatar: k.avatar.clone(),
                live: false,
                title: None,
                game: None,
                started: None,
            });
        }
    }
    flips
}

/// V108-C37b The go-live messages the streamers set themselves, one read for all that just
/// went live. Empty when there are none to ask about or the read fails (the title shows then).
async fn messages(app: &AppHandle, alerts: &[ChannelCard]) -> HashMap<String, String> {
    if alerts.is_empty() {
        return HashMap::new();
    }
    let http = app.state::<auth::Auth>().http.clone();
    let ids: Vec<String> = alerts.iter().map(|c| c.id.clone()).collect();
    gql::live_message::live_messages(&http, &ids).await.unwrap_or_else(|e| {
        eprintln!("alerts: {e}");
        HashMap::new()
    })
}

/// V108-F56f The longest go-live message or title an alert shows.
const TEXT_MAX: usize = 140;

/// The first line of `t`, trimmed and cut at `max` characters (with a `…`).
fn short_line(t: &str, max: usize) -> String {
    let first = t.trim().lines().next().unwrap_or_default().trim();
    if first.chars().count() <= max {
        return first.to_owned();
    }
    let mut out: String = first.chars().take(max - 1).collect();
    out.truncate(out.trim_end().len());
    out.push('…');
    out
}

/// The alert's text: the streamer's go-live message, else the stream title; the game under it.
fn alert_text(card: &ChannelCard, message: Option<String>) -> String {
    let stream = card.stream.as_ref();
    let line = |t: &str| short_line(t, TEXT_MAX);
    let mut text = line(&message.unwrap_or_else(|| stream.map(|s| s.title.clone()).unwrap_or_default()));
    if let Some(game) = stream.and_then(|s| s.game.as_ref()) {
        text = if text.is_empty() {
            game.name.clone()
        } else {
            format!("{text}\n{}", game.name)
        };
    }
    text
}

fn alert(app: &AppHandle, n: &Notifications, card: ChannelCard, message: Option<String>) {
    if n.live.sound && !crate::tabs::mute::hushes_id(app, &card.id) {
        crate::sound::alert(app, None);
    }
    let text = alert_text(&card, message);
    let title = format!("{} is live", card.name);
    // V108-C37 The same alert as a card in the app's corner; it has its own switch.
    cards::show(
        app,
        Card {
            kind: CardKind::Live,
            title: title.clone(),
            text: text.clone(),
            image: card.avatar.clone(),
            open: Open::Channel { login: card.login.clone() },
        },
    );
    if !n.live.toast {
        return;
    }
    // V108-C39 The button follows the tabs as they are now ("Go to tab" or "Open tab").
    let has_tab = app.state::<Settings>().read::<Tabs>(Partition::Tabs).channel_ids().contains(&card.id);
    let action = Some(toasts::tab_action(&card.login, has_tab));
    toasts::show(
        app,
        Pop {
            title,
            text,
            avatar: card.avatar.clone(),
            open: Open::Channel { login: card.login },
            action,
        },
    );
}

fn lock(app: &AppHandle) -> std::sync::MutexGuard<'_, Seen> {
    app.state::<State>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

#[cfg(test)]
mod tests {
    use cyex_settings::schema::LiveScope;

    use super::*;

    fn live(id: &str, started: &str) -> (ChannelCard, bool) {
        let card = ChannelCard {
            id: id.into(),
            login: id.into(),
            name: id.into(),
            avatar: None,
            stream: Some(LiveStream {
                title: "t".into(),
                viewers: 1,
                started_at: started.into(),
                rerun: false,
                game: None,
                tags: Vec::new(),
            }),
            last_live: None,
            last_game: None,
            last_title: None,
        };
        (card, true)
    }

    fn known(id: &str, started: &str) -> (String, Known) {
        let k = Known { started: started.into(), login: id.into(), name: id.into(), avatar: None };
        (id.to_owned(), k)
    }

    #[test]
    fn new_and_restarted_streams_alert() {
        let before = HashMap::from([known("a", "1"), known("b", "1")]);
        let now = [live("a", "1"), live("b", "2"), live("c", "1")];
        let ids: Vec<&str> = news(&before, &now, false)
            .iter()
            .map(|(c, _)| c.id.as_str())
            .collect();
        assert_eq!(ids, ["b", "c"]);
        assert!(news(&before, &now, true).is_empty());
    }

    fn scoped(scope: LiveScope) -> LiveAlerts {
        LiveAlerts { scope, ..LiveAlerts::default() }
    }

    fn ids(v: Vec<ChannelCard>) -> Vec<String> {
        v.into_iter().map(|c| c.id).collect()
    }

    fn offline(id: &str) -> ChannelCard {
        ChannelCard { stream: None, ..live(id, "x").0 }
    }

    #[test]
    fn scope_picks_followed_tabs_or_both() {
        let before = HashMap::new();
        let tab_live = HashMap::new();
        let tab_ids: HashSet<String> = ["t".to_owned(), "f".to_owned()].into();
        let prior = Prior { followed: &before, tab_live: &tab_live, tab_ids: &tab_ids };
        let followed = [live("f", "1"), live("g", "1")];
        let tabs = [live("t", "1").0, live("f", "1").0];
        let run = |s| ids(wanted(&scoped(s), &prior, &followed, &tabs, false));
        assert_eq!(run(LiveScope::Followed), ["f", "g"]);
        assert_eq!(run(LiveScope::Tabs), ["t", "f"]);
        assert_eq!(run(LiveScope::Both), ["f", "g", "t"]);
    }

    #[test]
    fn a_channel_in_both_places_alerts_once() {
        let before = HashMap::new();
        let tab_live = HashMap::new();
        let tab_ids: HashSet<String> = ["f".to_owned()].into();
        let prior = Prior { followed: &before, tab_live: &tab_live, tab_ids: &tab_ids };
        let got = wanted(&scoped(LiveScope::Both), &prior, &[live("f", "1")], &[live("f", "1").0], false);
        assert_eq!(ids(got), ["f"]);
        // The followed side alerted last minute; the tabs' poll catching up doesn't repeat it.
        let before = HashMap::from([known("f", "1")]);
        let prior = Prior { followed: &before, tab_live: &tab_live, tab_ids: &tab_ids };
        assert!(wanted(&scoped(LiveScope::Both), &prior, &[live("f", "1")], &[live("f", "1").0], false).is_empty());
    }

    #[test]
    fn a_tab_opened_since_stays_quiet_and_overrides_apply() {
        let before = HashMap::new();
        let tab_live = HashMap::new();
        let none = HashSet::new();
        let prior = Prior { followed: &before, tab_live: &tab_live, tab_ids: &none };
        let tabs = [live("t", "1").0];
        assert!(wanted(&scoped(LiveScope::Tabs), &prior, &[], &tabs, false).is_empty());
        // Known before as offline, live now: alerts, unless turned off or the first poll.
        let seen: HashSet<String> = ["t".to_owned()].into();
        let prior = Prior { followed: &before, tab_live: &tab_live, tab_ids: &seen };
        let mut l = scoped(LiveScope::Tabs);
        assert_eq!(ids(wanted(&l, &prior, &[], &tabs, false)), ["t"]);
        assert!(wanted(&l, &prior, &[], &tabs, true).is_empty());
        l.channels.insert("t".into(), false);
        assert!(wanted(&l, &prior, &[], &tabs, false).is_empty());
        // Offline tab channels never alert; a restart does.
        let off = [offline("t")];
        assert!(wanted(&scoped(LiveScope::Tabs), &prior, &[], &off, false).is_empty());
        let was = HashMap::from([("t".to_owned(), "1".to_owned())]);
        let prior = Prior { followed: &before, tab_live: &was, tab_ids: &seen };
        assert!(wanted(&scoped(LiveScope::Tabs), &prior, &[], &tabs, false).is_empty());
        assert_eq!(ids(wanted(&scoped(LiveScope::Tabs), &prior, &[], &[live("t", "2").0], false)), ["t"]);
    }

    #[test]
    fn feed_flips_on_start_restart_and_end() {
        let before = HashMap::from([known("a", "1"), known("b", "1"), known("gone", "1")]);
        let now = [live("a", "1"), live("b", "2"), live("c", "1")];
        let mut f: Vec<(String, bool)> = live_flips(&before, &now).into_iter().map(|f| (f.id, f.live)).collect();
        f.sort();
        assert_eq!(f, [("b".to_owned(), true), ("c".to_owned(), true), ("gone".to_owned(), false)]);
        assert_eq!(live_flips(&before, &now).iter().find(|f| f.id == "b").and_then(|f| f.title.clone()).as_deref(), Some("t"));
    }

    #[test]
    fn the_streamers_message_replaces_the_title() {
        let (card, _) = live("a", "1");
        assert!(alert_text(&card, Some("Ranked grind!".into())).starts_with("Ranked grind!"));
        assert!(alert_text(&card, None).starts_with('t'));
    }

    #[test]
    fn long_messages_are_cut_to_one_line() {
        assert_eq!(short_line("  one
two ", 140), "one");
        let cut = short_line(&"é".repeat(300), 140);
        assert_eq!(cut.chars().count(), 140);
        assert!(cut.ends_with('…'));
        assert_eq!(short_line(&"a".repeat(140), 140).chars().count(), 140);
    }

    #[test]
    fn same_instant_in_another_format_is_not_a_restart() {
        assert!(same_start("2026-11-04T10:00:00Z", "2026-11-04T11:00:00+01:00"));
        assert!(same_start("2026-11-04T10:00:00Z", "2026-11-04T10:00:00.000Z"));
        assert!(!same_start("2026-11-04T10:00:00Z", "2026-11-04T10:05:00Z"));
        assert!(same_start("1", "1") && !same_start("1", "2"));
        let before = HashMap::from([known("a", "2026-11-04T10:00:00Z")]);
        assert!(news(&before, &[live("a", "2026-11-04T10:00:00.000+00:00")], false).is_empty());
    }
}
