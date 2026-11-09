//! Drops (1.30, 1.32): the account's inventory every minute while a watched channel has
//! drops (every 15 minutes otherwise); earned drops are claimed 5-10 s apart. Twitch
//! wants the game account linked before it hands a drop out. V110-U68: the same read feeds
//! badge progress (`badges.rs`); while a badge is being earned it runs every minute, while
//! hidden and idle every 30 minutes. Badge drops of wanted campaigns are claimed even with
//! drop claims off.

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use cyex_miner::cadence;
use cyex_miner::stats::{Kind, Record};
use cyex_twitch::gql::drops::both_refused;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::log::LogKind;
use crate::cards::{self, Card, Kind as CardKind};
use crate::toasts::Open;
use super::{gql_auth, http, lock, log, now_ms};


/// A drop still being earned, for the miner view.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DropProgress {
    pub name: String,
    pub game: Option<String>,
    pub image: Option<String>,
    pub minutes: u32,
    pub required: u32,
}

/// V108-C37 A card for a drop that came in; a click opens the miner page.
fn card(app: &AppHandle, title: &str, name: &str, game: Option<&str>, image: Option<String>) {
    let text = game.map_or_else(|| name.to_owned(), |g| format!("{name}\n{g}"));
    cards::show(
        app,
        Card { kind: CardKind::Drop, title: title.into(), text, image, open: Open::Drops },
    );
}

/// V108-C37 True the first time this drop instance is seen refused, so a drop that stays
/// unclaimable doesn't pop up every minute.
fn told_once(instance: &str) -> bool {
    static SEEN: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Mutex::default);
    first_time(&mut SEEN.lock().unwrap_or_else(|p| p.into_inner()), instance)
}

/// V108-C37d True when `instance` wasn't in `seen`; the set starts over past `SEEN_MAX`.
fn first_time(seen: &mut HashSet<String>, instance: &str) -> bool {
    if seen.len() > SEEN_MAX {
        seen.clear();
    }
    seen.insert(instance.to_owned())
}

/// V108-C37d Refused instances kept before the list starts over.
const SEEN_MAX: usize = 500;

pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // Let the first live check run first.
        tokio::time::sleep(Duration::from_secs(20)).await;
        let idle_wake = app.state::<super::Miner>().idle_wake.clone();
        loop {
            let started = tokio::time::Instant::now();
            let mut busy = tick(&app).await;
            // A window showing or a badge starting can make the wait shorter.
            loop {
                let wait = if busy {
                    cadence::INVENTORY_BUSY
                } else if super::idle::idle(&app) {
                    cadence::INVENTORY_HIDDEN
                } else {
                    cadence::INVENTORY_IDLE
                };
                tokio::select! {
                    () = tokio::time::sleep_until(started + wait) => break,
                    () = idle_wake.notified() => busy = focused(&app),
                }
            }
        }
    });
}

fn focused(app: &AppHandle) -> bool {
    lock(app).focus.is_some()
}

/// One read of the inventory. True when it should run again within the minute.
async fn tick(app: &AppHandle) -> bool {
    let s = super::settings(app);
    let badges_wanted = s.badges.auto || !s.badges.picked.is_empty();
    if !s.claim_drops && !badges_wanted {
        return false;
    }
    let Some((account, token, client_id)) = super::signed_in(app) else {
        return false;
    };
    let (busy, credit_to) = {
        let st = lock(app);
        let watched: Vec<&super::Chan> = st
            .channels
            .iter()
            .filter(|c| st.slots.contains(&c.id) || st.watching.contains(&c.id))
            .collect();
        let with_drops = watched.iter().find(|c| c.drops);
        (
            (s.claim_drops && with_drops.is_some()) || st.focus.is_some(),
            with_drops.or(watched.first()).map(|c| c.id.clone()),
        )
    };
    let http = http(app);
    let auth = gql_auth(&token, &client_id);
    let inv = match super::source::inventory(app, &http, auth).await {
        Ok(i) => i,
        Err(e) => {
            eprintln!("miner: drops: {e}");
            let text = if both_refused(&e) {
                "Couldn’t read the drops: Twitch turned both ways of asking down."
            } else {
                "Couldn’t read the drops."
            };
            log::add(app, LogKind::Error, None, text);
            return busy;
        }
    };
    super::badges::apply_inventory(app, &inv);
    let campaigns = inv.campaigns;
    let done: HashSet<String> = campaigns
        .iter()
        .filter(|c| !c.drops.is_empty() && c.drops.iter().all(|d| d.claimed))
        .map(|c| c.id.clone())
        .collect();
    // Badge drops of these are claimed even with drop claims off.
    let badge_campaigns: HashSet<String> = {
        let st = lock(app);
        cyex_miner::badges::wanted(&st.badges, &s, now_ms() / 1000)
            .iter()
            .map(|b| b.campaign_id.clone())
            .collect()
    };
    let progress: Vec<DropProgress> = campaigns
        .iter()
        .flat_map(|c| c.drops.iter().map(move |d| (c, d)))
        .filter(|(_, d)| !d.claimed && d.instance_id.is_none())
        .map(|(c, d)| DropProgress {
            name: d.name.clone(),
            game: c.game.clone(),
            image: d.image.clone(),
            minutes: d.watched_minutes,
            required: d.required_minutes,
        })
        .collect();
    let changed = {
        let mut st = lock(app);
        let changed = st.drop_progress != progress;
        st.drop_progress = progress;
        st.done_campaigns = done;
        changed
    };
    if changed {
        super::emit_status(app);
    }
    let mut first = true;
    for c in &campaigns {
        let claim_all = s.claim_drops;
        let badge_campaign = badge_campaigns.contains(&c.id);
        for d in c.drops.iter().filter(|d| d.claimable() && (claim_all || (badge_campaign && d.badge))) {
            if !first {
                let pause = 5_000 + (now_ms() % 5_000) as u64;
                tokio::time::sleep(Duration::from_millis(pause)).await;
            }
            first = false;
            let Some(instance) = d.instance_id.as_deref() else {
                continue;
            };
            match super::source::claim(app, &http, auth, instance).await {
                Ok(true) => {
                    let game = c.game.as_deref().map(|g| format!(" ({g})")).unwrap_or_default();
                    log::add(app, LogKind::Drop, None, format!("Claimed the drop “{}”{game}.", d.name));
                    card(app, "Drop claimed", &d.name, c.game.as_deref(), d.image.clone());
                    if let Some(id) = &credit_to {
                        let rec = Record {
                            n: Some(d.name.clone()),
                            ..Record::new(now_ms(), Kind::Drop, 0)
                        };
                        super::stats::record(app, &account, id, &rec);
                    }
                }
                Ok(false) => {
                    log::add(
                        app,
                        LogKind::Drop,
                        None,
                        format!(
                            "Couldn’t claim “{}”. Is the game account linked on Twitch?",
                            d.name
                        ),
                    );
                    // V108-C37 Ready but Twitch said no: worth a card, once per drop.
                    if told_once(instance) {
                        card(app, "Drop ready", &d.name, c.game.as_deref(), d.image.clone());
                    }
                }
                Err(e) => {
                    eprintln!("miner: drop claim: {e}");
                    log::add(app, LogKind::Error, None, "Couldn’t claim the drop.");
                }
            }
        }
    }
    busy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v108_c37d_a_refused_drop_is_told_once_and_the_list_stays_small() {
        let mut seen = HashSet::new();
        assert!(first_time(&mut seen, "a"));
        assert!(!first_time(&mut seen, "a"));
        assert!(first_time(&mut seen, "b"));
        for i in 0..=SEEN_MAX {
            first_time(&mut seen, &i.to_string());
        }
        // Past the cap it starts over: "a" is new again, the set never grows far beyond it.
        assert!(first_time(&mut seen, "a"));
        assert!(seen.len() <= SEEN_MAX + 1);
    }
}
