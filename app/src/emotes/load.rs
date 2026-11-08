//! Fetching sets: global ones once per run, a channel's when its chat joins, and an
//! account's own Twitch emotes (GQL sign-in first, else Helix with `user:read:emotes`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cyex_chat::model::Provider;
use cyex_emotes::seventv::{self, socket::EventApi, socket::Topic as SevenTopic};
use cyex_emotes::{Emote, EmoteSet, Lookup, Scope};
use cyex_twitch::gql::smilies::{self, SmileyStyle};
use cyex_twitch::gql::{self, EmoteSet as TwitchSet};
use cyex_twitch::{Http, helix, ws};
use tauri::{AppHandle, Emitter, Manager};

use super::providers::Which;
use super::{Channel, Emotes};
use crate::auth::{Auth, gql_token, helix_scoped};
use crate::chat::identity;

/// An account's Twitch emotes are fetched again after this.
const MINE_FRESH: Duration = Duration::from_secs(300);

fn http(app: &AppHandle) -> Http {
    app.state::<Auth>().http.clone()
}

/// Global sets: those not in yet, unless a fetch or its retries run (B4: every provider
/// that failed for now is asked again). Lookups are rebuilt when they arrive.
pub(super) fn globals(app: &AppHandle) {
    let want = {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        let want = inner.idle.not().minus(inner.globals_done);
        if inner.globals_loading || !want.any() {
            return;
        }
        inner.globals_loading = true;
        want
    };
    super::retry::globals(app.clone(), want);
}

/// A channel's sets for join `generation`. B1 Put in like a retry's (`retry::merge`), in
/// place of what a socket change or an earlier join put in first.
pub fn channel(app: AppHandle, login: String, id: String, generation: u64) {
    let idle = app.state::<Emotes>().lock().idle;
    channel_of(app, login, id, generation, idle.not());
}

/// V110-E13 The sets of those in `want` only: a join asks for what's switched on, turning a
/// provider on asks for just that one.
pub(super) fn channel_of(app: AppHandle, login: String, id: String, generation: u64, want: Which) {
    globals(&app);
    if !want.any() {
        // Nothing to fetch: the room is as ready as it gets.
        super::retry::merge(&app, &login, generation, want, super::providers::Fetched::default());
        return;
    }
    tauri::async_runtime::spawn(async move {
        let http = http(&app);
        let lane = crate::chat::shown::lane(&app, &login);
        let sets_off = super::providers::current(&app).sets_off;
        let got = super::providers::channel(&http, &id, lane, want, sets_off).await;
        let failed = got.failed;
        if super::retry::merge(&app, &login, generation, want, got) && failed.any() {
            super::retry::spawn(app, login, id, generation, failed);
        }
    });
}

/// E2 Puts a channel's 7TV account and set in, in place of what it had (a set switch may
/// have come in first). Call it under the Emotes lock: a leave in between would miss
/// the topics and leak them (V107-R1).
pub(super) fn put_seventv(pool: &ws::Pool<EventApi>, c: &mut Channel, ch: seventv::api::Channel) {
    pool.add(SevenTopic::User(ch.user_id.clone()));
    if let Some(old) = c.seventv_user.replace(ch.user_id) {
        pool.remove(&SevenTopic::User(old));
    }
    swap_seventv_set(pool, c, ch.set);
}

/// Swaps the channel's 7TV set and its topic, under the Emotes lock (V107-R1). The new
/// topic goes on before the old comes off, so the same set stays subscribed.
fn swap_seventv_set(pool: &ws::Pool<EventApi>, c: &mut Channel, set: Option<EmoteSet>) {
    if let Some(s) = &set {
        pool.add(SevenTopic::Set(s.id.clone()));
    }
    let old: Vec<String> = c
        .sets
        .iter()
        .filter(|s| s.provider == Provider::Seventv)
        .map(|s| s.id.clone())
        .collect();
    c.sets.retain(|s| s.provider != Provider::Seventv);
    c.sets.extend(set.map(Arc::new));
    for id in old {
        pool.remove(&SevenTopic::Set(id));
    }
}

/// V108-F32 A 7TV set switch was asked for: its ticket.
fn next_switch(c: &mut Channel) -> u64 {
    c.seventv_switch += 1;
    c.seventv_switch
}

/// V108-F32 No newer switch was asked for since `ticket`.
fn still_newest(c: &Channel, ticket: u64) -> bool {
    c.seventv_switch == ticket
}

/// Replaces a channel's 7TV set, e.g. after the channel switched sets. V108-C31 Says so in
/// chat with the new set's name, unless it is the set already there.
pub fn seventv_set(app: AppHandle, login: String, set_id: Option<String>, actor: Option<String>) {
    tauri::async_runtime::spawn(async move {
        let (generation, ticket) = {
            let state = app.state::<Emotes>();
            let mut inner = state.lock();
            // V110-E13 With 7TV emotes off a channel's set switch changes nothing here.
            if inner.sets_off.seventv {
                return;
            }
            let Some(c) = inner.channels.get_mut(&login) else {
                return;
            };
            // Even a switch to the set already here makes older answers in flight stale.
            let ticket = next_switch(c);
            let mut have = c.sets.iter().filter(|s| s.provider == Provider::Seventv);
            let same = match &set_id {
                Some(id) => have.any(|s| s.id == *id),
                None => have.next().is_none(),
            };
            if same {
                return;
            }
            (c.generation, ticket)
        };
        let lane = crate::chat::shown::lane(&app, &login);
        let set = match &set_id {
            Some(id) => match seventv::api::sets(&http(&app), std::slice::from_ref(id), lane).await {
                Ok(mut s) if !s.is_empty() => {
                    let mut set = s.remove(0);
                    set.scope = Scope::Channel;
                    Some(set)
                }
                Ok(_) => return,
                Err(e) => {
                    eprintln!("emotes: no 7TV set {id} for {login}: {}", e.describe("7TV"));
                    return;
                }
            },
            None => None,
        };
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        // Gone, rejoined, or a newer switch was asked for meanwhile: this answer is stale.
        let Some(c) = inner.live(&login, generation).filter(|c| still_newest(c, ticket)) else {
            return;
        };
        let name = set.as_ref().and_then(|s| s.name.clone());
        swap_seventv_set(&state.seventv, c, set);
        inner.rebuild(&login);
        drop(inner);
        let text = super::live::switch_line(actor.as_deref(), name.as_deref(), set_id.is_none());
        super::change_line(&app, &login, &text);
    });
}

/// The Twitch sets an account can use in a channel (1.40), cached for a few minutes.
/// Also keeps the account's lookup for its own lines.
pub async fn twitch_sets(
    app: &AppHandle,
    account: &str,
    channel_id: &str,
    fresh: bool,
) -> Result<Arc<Vec<TwitchSet>>, String> {
    let state = app.state::<Emotes>();
    let key = (account.to_owned(), channel_id.to_owned());
    if !fresh
        && let Some((at, sets)) = state.lock().twitch_sets.get(&key)
        && at.elapsed() < MINE_FRESH
    {
        return Ok(sets.clone());
    }
    let (sets, style) = tokio::join!(fetch_sets(app, account, channel_id), smiley_style(app, account));
    Ok(store_sets(&mut state.lock(), key, sets?, style))
}

/// V108-C35 Reads the account's Twitch sets in a channel again and returns the emotes that
/// weren't there before: against that channel's last read, else every channel's. Nothing
/// known yet gives nothing new. A change drops the account's other channels' reads (they
/// are read again when asked for) and tells the pages (`emotes:mine-changed`), so an open
/// menu reads again; the same list again sends nothing.
pub async fn refresh_diff(app: &AppHandle, account: &str, channel_id: &str) -> Result<Vec<gql::TwitchEmote>, String> {
    let before = known(app, account, channel_id);
    refresh_diff_from(app, account, channel_id, before).await
}

/// V108-C35g The baseline `refresh_diff` would take now. A hype train takes it at its end,
/// so a menu read in the 5 s before the card's look can't become the baseline.
pub fn known(app: &AppHandle, account: &str, channel_id: &str) -> Option<Baseline> {
    let state = app.state::<Emotes>();
    let inner = state.lock();
    known_ids(&inner.twitch_sets, account, channel_id)
}

/// V108-C35g `refresh_diff` against a baseline taken earlier with `known`.
pub async fn refresh_diff_from(
    app: &AppHandle,
    account: &str,
    channel_id: &str,
    before: Option<Baseline>,
) -> Result<Vec<gql::TwitchEmote>, String> {
    let sets = twitch_sets(app, account, channel_id, true).await?;
    let (added, changed) = diff(before.as_ref(), &sets);
    if changed {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        inner.twitch_sets.retain(|(a, c), _| a != account || c == channel_id);
        rebuild_mine(&mut inner, account);
        drop(inner);
        let _ = app.emit(MINE_CHANGED, serde_json::json!({ "account": account }));
    }
    Ok(added)
}

const MINE_CHANGED: &str = "emotes:mine-changed";

/// Whether the ids are of the same channel, and the ids (see `known_ids`).
pub type Baseline = (bool, HashSet<String>);

/// V108-C35 The emote ids of the account's last read in `channel_id`, else of all its
/// channels; `None` when there is none. Ids are by emote, not set.
fn known_ids(
    kept: &HashMap<(String, String), (Instant, super::TwitchSets)>,
    account: &str,
    channel_id: &str,
) -> Option<Baseline> {
    let ids = |s: &super::TwitchSets| s.iter().flat_map(|s| s.emotes.iter().map(|e| e.id.clone())).collect::<Vec<_>>();
    if let Some((_, s)) = kept.get(&(account.to_owned(), channel_id.to_owned())) {
        return Some((true, ids(s).into_iter().collect()));
    }
    let all: HashSet<String> = kept.iter().filter(|((a, _), _)| a == account).flat_map(|(_, (_, s))| ids(s)).collect();
    (!all.is_empty()).then_some((false, all))
}

/// V108-C35 What came in, and whether the list changed: an emote came, or (against the same
/// channel) one went.
fn diff(before: Option<&Baseline>, sets: &[TwitchSet]) -> (Vec<gql::TwitchEmote>, bool) {
    let Some((same, before)) = before else { return (Vec::new(), false) };
    let mut seen = HashSet::new();
    let added: Vec<gql::TwitchEmote> = sets
        .iter()
        .flat_map(|s| s.emotes.iter())
        .filter(|e| !before.contains(&e.id) && seen.insert(e.id.clone()))
        .cloned()
        .collect();
    let now: HashSet<&str> = sets.iter().flat_map(|s| s.emotes.iter().map(|e| e.id.as_str())).collect();
    let gone = *same && before.iter().any(|id| !now.contains(id.as_str()));
    let changed = !added.is_empty() || gone;
    (added, changed)
}

/// Caches fetched sets in the account's smiley style. A style picked while the fetch ran
/// wins over `read`, the one read alongside it.
fn store_sets(inner: &mut super::Inner, key: (String, String), mut sets: Vec<TwitchSet>, read: SmileyStyle) -> super::TwitchSets {
    smilies::apply(&mut sets, inner.smilies.get(&key.0).copied().unwrap_or(read));
    let sets = Arc::new(sets);
    let account = key.0.clone();
    inner.twitch_sets.insert(key, (Instant::now(), sets.clone()));
    rebuild_mine(inner, &account);
    sets
}

/// The Twitch sets as Twitch lists them, through GQL or Helix.
async fn fetch_sets(app: &AppHandle, account: &str, channel_id: &str) -> Result<Vec<TwitchSet>, String> {
    let http = http(app);
    // V108-C35 Test builds: the hype train mock answers the account's emotes too.
    #[cfg(feature = "mocks")]
    let http = crate::chat::hype_mock::http(app, http);
    if let Some((token, client_id)) = gql_token(app, account) {
        let auth = crate::auth::gql_auth(&token, &client_id);
        gql::available_emotes(&http, auth, channel_id).await
    } else if let Some((token, client_id)) = helix_scoped(app, account, "user:read:emotes") {
        let auth = crate::auth::helix_auth(&token, &client_id);
        let mut sets = helix::user_emotes(&http, auth, account, channel_id).await;
        if let Ok(sets) = &mut sets {
            name_owners(&http, sets).await;
        }
        sets
    } else {
        return Err("Sign in to use your Twitch emotes.".into());
    }
    .map_err(|e| e.to_string())
}

/// The account's lookup for its own lines, from every channel's sets it has.
fn rebuild_mine(inner: &mut super::Inner, account: &str) {
    let mine: Vec<EmoteSet> = inner
        .twitch_sets
        .iter()
        .filter(|((a, _), _)| a == account)
        .flat_map(|(_, (_, s))| s.iter())
        .map(|s| {
            let emotes = s.emotes.iter().map(twitch_emote).collect();
            EmoteSet::new(s.id.clone(), Provider::Twitch, Scope::Global, emotes)
        })
        .collect();
    inner
        .mine
        .insert(account.to_owned(), Arc::new(Lookup::build(mine.iter())));
}

/// V107-Mk The account's smiley style: read once a run with the GQL sign-in, Robot without.
/// A failed read answers Robot this time but is not kept, so the next fetch reads again;
/// opening the chat identity reads it too. Test builds with the identity mock set read it
/// there (or nothing, when it is unusable), never from Twitch.
async fn smiley_style(app: &AppHandle, account: &str) -> SmileyStyle {
    if let Some(s) = app.state::<Emotes>().lock().smilies.get(account) {
        return *s;
    }
    let Ok(base) = identity::mock_base(app) else {
        return SmileyStyle::Robot;
    };
    let Some((token, client_id)) = identity::gql(app, account, base.as_deref()) else {
        return SmileyStyle::Robot;
    };
    let auth = crate::auth::gql_auth(&token, &client_id);
    match smilies::smilies(&http(app), auth, base.as_deref()).await {
        Ok(s) => *app.state::<Emotes>().lock().smilies.entry(account.to_owned()).or_insert(s.current),
        Err(e) => {
            eprintln!("emotes: no smiley style for {account}: {e}");
            SmileyStyle::Robot
        }
    }
}

/// V107-Mk A new smiley style: the menu and own lines switch art at once, no refetch.
/// Cached sets always carry the stored style, so the same style again changes nothing.
pub fn set_smiley_style(app: &AppHandle, account: &str, style: SmileyStyle) {
    restyle(&mut app.state::<Emotes>().lock(), account, style);
}

fn restyle(inner: &mut super::Inner, account: &str, style: SmileyStyle) {
    if inner.smilies.insert(account.to_owned(), style) == Some(style) {
        return;
    }
    for ((a, _), (_, sets)) in inner.twitch_sets.iter_mut() {
        if a == account {
            let mut fresh = sets.as_ref().clone();
            smilies::apply(&mut fresh, style);
            *sets = Arc::new(fresh);
        }
    }
    rebuild_mine(inner, account);
}

/// Loads the account's Twitch emotes in the background, once per run unless `fresh`.
pub fn mine(app: AppHandle, account: String, channel_id: String, fresh: bool) {
    {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        if !fresh && (inner.mine.contains_key(&account) || !inner.mine_loading.insert(account.clone())) {
            return;
        }
    }
    tauri::async_runtime::spawn(async move {
        if let Err(e) = twitch_sets(&app, &account, &channel_id, fresh).await {
            eprintln!("emotes: no Twitch emotes for {account}: {e}");
        }
        app.state::<Emotes>().lock().mine_loading.remove(&account);
    });
}

pub fn twitch_emote(e: &gql::TwitchEmote) -> Emote {
    let mut emote = Emote::new(Provider::Twitch, e.id.clone(), e.name.clone());
    emote.animated = e.animated;
    emote
}

/// Helix only gives owner ids; names come from the public user lookup.
async fn name_owners(http: &Http, sets: &mut [TwitchSet]) {
    let mut ids: Vec<String> = sets
        .iter()
        .filter_map(|s| s.owner.as_ref().map(|o| o.id.clone()))
        .collect();
    ids.sort();
    ids.dedup();
    let Ok(cards) = gql::cards(http, &ids).await else {
        return;
    };
    for s in sets.iter_mut() {
        if let Some(o) = &mut s.owner
            && let Some(c) = cards.iter().find(|c| c.id == o.id)
        {
            o.login.clone_from(&c.login);
            o.name.clone_from(&c.name);
            o.avatar.clone_from(&c.avatar);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_twitch::gql::{EmoteKind, TwitchEmote};

    fn sets() -> Vec<TwitchSet> {
        let emote = |id: &str, name: &str| TwitchEmote {
            id: id.into(),
            name: name.into(),
            set_id: "0".into(),
            kind: EmoteKind::Global,
            animated: false,
            modifiers: Vec::new(),
        };
        vec![TwitchSet { id: "0".into(), owner: None, emotes: vec![emote("25", "Kappa"), emote("1", ":)")], tier: None }]
    }

    #[test]
    fn v108_c35_new_emotes_against_the_last_read() {
        let mut kept = HashMap::new();
        assert_eq!(known_ids(&kept, "me", "1"), None);
        // Nothing known: nothing is new, nothing changed.
        assert_eq!(diff(None, &sets()), (Vec::new(), false));
        let mut old = sets();
        old[0].emotes.truncate(1);
        kept.insert(("me".to_owned(), "2".to_owned()), (Instant::now(), Arc::new(old.clone())));
        // Another channel's read stands in for this one.
        let other = known_ids(&kept, "me", "1").unwrap();
        assert!(!other.0 && other.1.contains("25"));
        let (added, changed) = diff(Some(&other), &sets());
        assert_eq!(added.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), [":)"]);
        assert!(changed);
        // The same list again sends nothing.
        kept.insert(("me".to_owned(), "1".to_owned()), (Instant::now(), Arc::new(sets())));
        let same = known_ids(&kept, "me", "1").unwrap();
        assert!(same.0);
        assert_eq!(diff(Some(&same), &sets()), (Vec::new(), false));
        // One gone from the same channel is a change with nothing new.
        assert_eq!(diff(Some(&same), &old), (Vec::new(), true));
        // But one channel lacking another's emote is not.
        assert_eq!(diff(Some(&other), &old), (Vec::new(), false));
    }

    #[test]
    fn v108_c35g_a_baseline_taken_earlier_outlives_a_refresh() {
        let mut old = sets();
        old[0].emotes.truncate(1);
        let mut kept = HashMap::new();
        kept.insert(("me".to_owned(), "1".to_owned()), (Instant::now(), Arc::new(old)));
        let baseline = known_ids(&kept, "me", "1").unwrap();
        // Something else (the menu) reads the new list into the cache meanwhile.
        kept.insert(("me".to_owned(), "1".to_owned()), (Instant::now(), Arc::new(sets())));
        let live = known_ids(&kept, "me", "1").unwrap();
        // The live cache as baseline finds nothing; the one taken earlier finds the emote.
        assert_eq!(diff(Some(&live), &sets()), (Vec::new(), false));
        let (added, changed) = diff(Some(&baseline), &sets());
        assert_eq!(added.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), [":)"]);
        assert!(changed);
    }

    #[test]
    fn v108_f32_only_the_newest_switch_lands() {
        let mut inner = super::super::Inner::default();
        let (ready, _) = tokio::sync::watch::channel(false);
        inner.channels.insert(
            "c".into(),
            Channel {
                id: "1".into(),
                generation: 3,
                retry: None,
                sets: Vec::new(),
                seventv_user: None,
                seventv_switch: 0,
                ffz_badges: Default::default(),
                lookup: Arc::default(),
                whole: false,
                ready,
            },
        );
        let a = next_switch(inner.channels.get_mut("c").unwrap());
        let b = next_switch(inner.channels.get_mut("c").unwrap());
        assert!(inner.live("c", 3).is_some_and(|c| still_newest(c, b)));
        assert!(!inner.live("c", 3).is_some_and(|c| still_newest(c, a)));
        // A rejoin is another generation.
        assert!(inner.live("c", 4).is_none());
    }

    fn own(inner: &super::super::Inner, name: &str) -> String {
        inner.mine["me"].get(name).map(|e| e.id.to_string()).unwrap_or_default()
    }

    #[test]
    fn v107_mk_own_smileys_follow_the_style() {
        let mut inner = super::super::Inner::default();
        let key = ("me".to_owned(), "9".to_owned());
        store_sets(&mut inner, key.clone(), sets(), SmileyStyle::Robot);
        assert_eq!(own(&inner, ":)"), "1");

        restyle(&mut inner, "me", SmileyStyle::Glitch);
        assert_eq!(own(&inner, ":)"), "440");
        assert_eq!(own(&inner, "Kappa"), "25");
        // The same style again leaves the cached sets alone.
        let before = inner.twitch_sets[&key].1.clone();
        restyle(&mut inner, "me", SmileyStyle::Glitch);
        assert!(Arc::ptr_eq(&before, &inner.twitch_sets[&key].1));

        // A fetch that read Robot before the pick still lands in Glitch.
        store_sets(&mut inner, ("me".into(), "10".into()), sets(), SmileyStyle::Robot);
        assert!(inner.twitch_sets.values().all(|(_, s)| s.iter().flat_map(|s| &s.emotes).all(|e| e.id != "1")));

        restyle(&mut inner, "me", SmileyStyle::Robot);
        assert_eq!(own(&inner, ":)"), "1");
    }
}
