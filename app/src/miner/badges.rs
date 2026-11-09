//! V110-U68 Badge drops at run time: which live channels can earn a wanted badge (focus
//! goes to the best one, `watch.rs`), helper channels found when none in the list can (5 per
//! campaign, 10 adds per campaign per session, looked for again after 30 minutes, kept in
//! memory only and shown as "For a badge"), and what the inventory says was earned.

use cyex_miner::badges::{self as core, Badge, Helper, StreamInfo};
use cyex_settings::schema::Miner as MinerSettings;
use cyex_twitch::gql::drops::Inventory;
use cyex_twitch::gql::miner as gm;
use cyex_twitch::gql::{self as gql, StreamFilter};
use tauri::AppHandle;

use super::log::{self, LogKind};
use super::{State, gql_auth, http, lock, now_ms};
use crate::cards::{self, Card, Kind as CardKind};
use crate::toasts::Open;

/// Channels a directory search looks at.
const DIRECTORY_FIRST: usize = 30;
/// Earned badges kept for the view.
const EARNED_MAX: usize = 200;
/// Minutes in a row a helper's stream can't be reached before it is let go.
const HELPER_FAILS: u32 = 3;
/// A helper not seen live within this long after it was added is let go.
const HELPER_GRACE_SECS: i64 = 3 * 60;
/// Campaigns looked for per channel refresh; the rest wait for the next one.
const FIND_PER_ROUND: usize = 3;

fn now_secs() -> i64 {
    now_ms() / 1000
}

/// What badges care about in a channel.
fn info(c: &super::Chan) -> StreamInfo<'_> {
    StreamInfo {
        channel_id: &c.id,
        login: &c.login,
        live: c.stream.is_some(),
        game_id: c
            .stream
            .as_ref()
            .and_then(|s| s.game.as_ref())
            .map(|g| g.0.as_str()),
        campaigns: &c.campaigns,
    }
}

/// The badge targets now: channel ids that earn a wanted badge (list first, helpers after,
/// streams playing here left out), and a stream playing here that earns one, if any.
pub(super) struct Targets {
    pub ids: Vec<String>,
    pub playing: Option<(String, String)>,
}

pub(super) fn targets(st: &State, s: &MinerSettings) -> Targets {
    let wanted = core::wanted(&st.badges, s, now_secs());
    let mut out = Targets {
        ids: Vec::new(),
        playing: None,
    };
    if wanted.is_empty() {
        return out;
    }
    for c in &st.channels {
        let Some(b) = core::target_badge(&info(c), &wanted) else {
            continue;
        };
        if st.watching.contains(&c.id) {
            out.playing.get_or_insert((c.id.clone(), b.campaign_id.clone()));
        } else {
            out.ids.push(c.id.clone());
        }
    }
    out
}

/// Some badge is wanted and still open.
pub(super) fn wanted_any(app: &AppHandle, s: &MinerSettings) -> bool {
    !core::wanted(&lock(app).badges, s, now_secs()).is_empty()
}

/// The campaign a channel would earn for, of the wanted ones.
pub(super) fn campaign_of(st: &State, s: &MinerSettings, channel_id: &str) -> Option<String> {
    let wanted = core::wanted(&st.badges, s, now_secs());
    let c = st.channels.iter().find(|c| c.id == channel_id)?;
    core::target_badge(&info(c), &wanted).map(|b| b.campaign_id.clone())
}

/// The badge by campaign, for log lines.
pub(super) fn name_of(st: &State, campaign_id: &str) -> String {
    core::current(&st.badges, campaign_id).map_or_else(String::new, |b| b.name.clone())
}

/// When a badge that waits is looked for again: the earliest retry of a wanted campaign no
/// live channel can earn. None while one is being earned.
pub(super) fn retry_at(st: &State, s: &MinerSettings, now: i64) -> Option<i64> {
    if st.focus.is_some() {
        return None;
    }
    core::wanted(&st.badges, s, now)
        .iter()
        .filter_map(|b| st.helper_tried.get(&b.campaign_id))
        .map(|t| t + core::RETRY_SECS)
        .min()
}

/// Helpers as (id, login, name) for the channel list, after the list's own channels.
pub(super) fn helper_channels(st: &State) -> Vec<(String, String, String)> {
    st.helpers
        .iter()
        .map(|h| (h.channel_id.clone(), h.login.clone(), h.login.clone()))
        .collect()
}

/// Lets go of helpers no longer needed: badge earned, campaign over or unpicked, stream
/// offline, or unreachable.
pub(super) fn prune(app: &AppHandle, s: &MinerSettings) {
    let gone: Vec<(String, String)> = {
        let mut st = lock(app);
        let mut drop = core::helpers_to_drop(&st.helpers, &st.badges, s, now_secs());
        if !s.enabled || st.account.is_none() {
            drop = st.helpers.iter().map(|h| h.channel_id.clone()).collect();
        }
        let now = now_secs();
        for h in &st.helpers {
            let c = st.channels.iter().find(|c| c.id == h.channel_id);
            // A new helper gets a few minutes for its first live check.
            let offline = c.is_none_or(|c| c.stream.is_none()) && now - h.added_at >= HELPER_GRACE_SECS;
            let failing = c.is_some_and(|c| c.fails >= HELPER_FAILS);
            if (offline || failing) && !drop.contains(&h.channel_id) {
                drop.push(h.channel_id.clone());
            }
        }
        let mut gone = Vec::new();
        for id in drop {
            if let Some(c) = st.channels.iter().find(|c| c.id == id)
                && c.fails >= HELPER_FAILS
            {
                st.helper_skip.insert(id.clone());
            }
            if let Some(i) = st.helpers.iter().position(|h| h.channel_id == id) {
                let h = st.helpers.remove(i);
                gone.push((h.login, id));
            }
        }
        let kept: std::collections::HashSet<String> =
            st.helpers.iter().map(|h| h.channel_id.clone()).collect();
        st.channels
            .retain(|c| c.helper_for.is_none() || kept.contains(&c.id));
        let slots: Vec<String> = st
            .slots
            .iter()
            .filter(|id| st.channels.iter().any(|c| &c.id == *id))
            .cloned()
            .collect();
        st.slots = slots;
        gone
    };
    for (login, _) in &gone {
        log::add(
            app,
            LogKind::Badge,
            Some(login),
            "Not needed for a badge anymore.",
        );
    }
    if !gone.is_empty() {
        super::watch::topics(app);
        super::emit_status(app);
    }
}

/// Looks for helper channels for every wanted badge that nothing live in the list can
/// earn, when its retry is due.
pub(super) async fn find(app: &AppHandle, account: &str, s: &MinerSettings) {
    let now = now_secs();
    let due: Vec<Badge> = {
        let mut st = lock(app);
        let wanted: Vec<Badge> = core::wanted(&st.badges, s, now).into_iter().cloned().collect();
        let mut due = Vec::new();
        for b in wanted {
            let live = st.channels.iter().any(|c| core::is_target(&info(c), &b));
            let adds = st.helper_adds.get(&b.campaign_id).copied().unwrap_or(0);
            let room = core::helpers_wanted(&st.helpers, &b.campaign_id, adds, s);
            if live || room == 0 || !core::retry_due(st.helper_tried.get(&b.campaign_id).copied(), now) {
                continue;
            }
            if due.len() >= FIND_PER_ROUND {
                break;
            }
            st.helper_tried.insert(b.campaign_id.clone(), now);
            due.push(b);
        }
        due
    };
    if due.is_empty() {
        return;
    }
    let Some((token, client_id)) = crate::auth::gql_token(app, account) else {
        return;
    };
    let mut added = false;
    for (i, b) in due.into_iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        }
        let found = candidates(app, &b).await;
        let known: Vec<String> = {
            let st = lock(app);
            st.channels
                .iter()
                .map(|c| c.id.clone())
                .chain(st.helper_skip.iter().cloned())
                .collect()
        };
        let fresh: Vec<(String, String)> = found.into_iter().filter(|(id, _)| !known.contains(id)).collect();
        if fresh.is_empty() {
            continue;
        }
        let ids: Vec<String> = fresh.iter().map(|f| f.0.clone()).collect();
        let running = match super::source::channels(app, &http(app), gql_auth(&token, &client_id), &ids).await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("miner: badge channels: {e}");
                continue;
            }
        };
        let picked: Vec<(String, String)> = {
            let mut st = lock(app);
            let adds = st.helper_adds.get(&b.campaign_id).copied().unwrap_or(0);
            let room = core::helpers_wanted(&st.helpers, &b.campaign_id, adds, s);
            let picked: Vec<(String, String)> = fresh
                .into_iter()
                .filter(|(id, _)| {
                    running
                        .iter()
                        .any(|(c, camps)| c == id && camps.contains(&b.campaign_id))
                })
                .take(room)
                .collect();
            for (id, login) in &picked {
                st.helpers.push(Helper {
                    channel_id: id.clone(),
                    login: login.clone(),
                    campaign_id: b.campaign_id.clone(),
                    added_at: now,
                });
            }
            *st.helper_adds.entry(b.campaign_id.clone()).or_default() += picked.len() as u32;
            picked
        };
        for (_, login) in &picked {
            log::add(
                app,
                LogKind::Badge,
                Some(login),
                format!("Found for the {}.", core::phrase(&b.name)),
            );
            added = true;
        }
    }
    if added {
        super::channels::nudge(app);
    }
}

/// Live channels that may earn `b`: its allow list, else its game's directory by viewers.
async fn candidates(app: &AppHandle, b: &Badge) -> Vec<(String, String)> {
    if let Some(mocked) = super::source::mock_helpers(app) {
        return mocked;
    }
    let http = http(app);
    if !b.allowed.is_empty() {
        let logins: Vec<String> = b.allowed.iter().take(100).map(|a| a.login.clone()).collect();
        return match gm::streams_by_logins(&http, &logins).await {
            Ok(found) => found
                .into_iter()
                .filter(|c| {
                    c.stream.as_ref().is_some_and(|s| {
                        b.game_id.is_empty() || s.game.as_ref().is_some_and(|g| g.0 == b.game_id)
                    })
                })
                .take(DIRECTORY_FIRST)
                .map(|c| (c.id, c.login))
                .collect(),
            Err(e) => {
                eprintln!("miner: badge allow list: {e}");
                Vec::new()
            }
        };
    }
    if b.game_slug.is_empty() {
        return Vec::new();
    }
    let filter = StreamFilter::default();
    match gql::category(&http, gql::Auth::PUBLIC, &b.game_slug, &filter, None).await {
        Ok(Some(cat)) => cat
            .streams
            .items
            .into_iter()
            .filter(|c| c.stream.as_ref().is_some_and(|s| !s.rerun))
            .take(DIRECTORY_FIRST)
            .map(|c| (c.id, c.login))
            .collect(),
        Ok(None) => Vec::new(),
        Err(e) => {
            eprintln!("miner: badge directory: {e}");
            Vec::new()
        }
    }
}

/// The inventory's progress and earned rewards, applied to the badges; earned ones get a
/// log line and a card.
pub(super) fn apply_inventory(app: &AppHandle, inv: &Inventory) {
    let now = now_secs();
    let s = super::settings(app);
    let newly = {
        let mut st = lock(app);
        let added = core::add_from_inventory(&mut st.badges, inv, now);
        if !added.is_empty() {
            eprintln!("miner: {} badge(s) from the inventory", added.len());
        }
        let focus = st.focus.clone();
        let watched: Vec<(String, String, Vec<String>)> = st
            .channels
            .iter()
            .filter(|c| st.slots.contains(&c.id) || st.watching.contains(&c.id))
            .map(|c| (c.id.clone(), c.name.clone(), c.campaigns.clone()))
            .collect();
        let channel = |campaign: &str| {
            if let Some((ch, camp)) = &focus
                && camp == campaign
            {
                return watched
                    .iter()
                    .find(|w| &w.0 == ch)
                    .map(|w| (w.0.clone(), w.1.clone()));
            }
            watched
                .iter()
                .find(|w| w.2.iter().any(|c| c == campaign))
                .map(|w| (w.0.clone(), w.1.clone()))
        };
        let mut badges = std::mem::take(&mut st.badges);
        let newly = core::apply_inventory(&mut badges, inv, now, channel);
        st.badges = badges;
        st.earned.extend(newly.iter().cloned());
        let over = st.earned.len().saturating_sub(EARNED_MAX);
        st.earned.drain(..over);
        newly
    };
    for e in &newly {
        let text = match &e.channel {
            Some(ch) => format!("Earned the {} watching {ch}.", core::phrase(&e.name)),
            None => format!("Earned the {}.", core::phrase(&e.name)),
        };
        log::add(app, LogKind::Badge, e.channel.as_deref(), text);
        let body = if e.game.is_empty() {
            e.name.clone()
        } else {
            format!("{}\n{}", e.name, e.game)
        };
        let image = (!e.image.is_empty()).then(|| e.image.clone());
        cards::show(
            app,
            Card {
                kind: CardKind::Drop,
                title: "Badge earned".into(),
                text: body,
                image,
                open: Open::Drops,
            },
        );
    }
    if !newly.is_empty() {
        prune(app, &s);
    }
}

/// A minute for a helper failed or worked: unreachable ones go after a few tries.
pub(super) fn reached(app: &AppHandle, channel_id: &str, ok: bool) {
    let mut st = lock(app);
    if let Some(c) = st.channels.iter_mut().find(|c| c.id == channel_id) {
        c.fails = if ok { 0 } else { c.fails + 1 };
    }
}
