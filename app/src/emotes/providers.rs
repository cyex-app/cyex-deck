//! BTTV and FFZ (1.38, 1.39): global and channel sets, badges, FFZ's per-user sets, and
//! BTTV's socket (C9 Pro emotes and badges, C10 emotes changing live).

use std::sync::Arc;

use cyex_chat::model::Provider;
use cyex_emotes::bttv::{self, socket::Event as BttvEvent};
use cyex_emotes::ffz::{self, api::RoomBadges};
use cyex_emotes::{EmoteSet, Scope};
use cyex_twitch::Http;
use cyex_twitch::pace::Lane;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::UnboundedReceiver;

use super::{Emotes, looks};

/// Global sets of both, plus their badges and FFZ's sets for some users.
pub async fn globals(app: &AppHandle, http: &Http) -> Vec<Arc<EmoteSet>> {
    let (b, f, bb, fb) = tokio::join!(
        bttv::api::global(http),
        ffz::api::global(http),
        bttv::api::badges(http),
        ffz::api::badges(http)
    );
    let mut sets = Vec::new();
    match b {
        Ok(s) => sets.push(Arc::new(s)),
        Err(e) => eprintln!("emotes: no BTTV globals: {}", e.describe("BTTV")),
    }
    let state = app.state::<Emotes>();
    let mut inner = state.lock();
    match f {
        Ok(g) => {
            sets.extend(g.sets.into_iter().map(Arc::new));
            for (set, users) in g.personal {
                let id = set.id.clone();
                inner.personal_sets.insert(id.clone(), Arc::new(set));
                for u in users {
                    let list = inner.user_sets.entry(u.clone()).or_default();
                    if !list.contains(&id) {
                        list.push(id.clone());
                    }
                    inner.rebuild_personal(&u);
                }
            }
        }
        Err(e) => eprintln!("emotes: no FFZ globals: {}", e.describe("FFZ")),
    }
    if let Ok((defs, users)) = bb {
        for d in defs {
            inner.cosmetics.add_badge(d);
        }
        for (user, badge) in users {
            inner.cosmetics.set_bttv_badge(&user, Some(&badge));
        }
    }
    if let Ok((defs, users)) = fb {
        for d in defs {
            inner.cosmetics.add_badge(d);
        }
        inner.cosmetics.set_ffz_users(users);
    }
    sets
}

/// A channel's BTTV and FFZ sets, and FFZ's mod and VIP pictures for it.
/// V107-P2 `lane`: the tab on screen goes ahead of background ones.
pub async fn channel(http: &Http, id: &str, lane: Lane) -> (Vec<Arc<EmoteSet>>, RoomBadges) {
    let (b, f) = tokio::join!(bttv::api::channel(http, id, lane), ffz::api::room(http, id, lane));
    let mut sets = Vec::new();
    let mut badges = RoomBadges::default();
    match b {
        Ok(s) => sets.extend(s.map(Arc::new)),
        Err(e) => eprintln!("emotes: no BTTV emotes for {id}: {}", e.describe("BTTV")),
    }
    match f {
        Ok(Some(room)) => {
            sets.extend(room.set.map(Arc::new));
            badges = room.badges;
        }
        Ok(None) => {}
        Err(e) => eprintln!("emotes: no FFZ emotes for {id}: {}", e.describe("FFZ")),
    }
    (sets, badges)
}

pub fn spawn_bttv(app: AppHandle, mut rx: UnboundedReceiver<BttvEvent>) {
    tauri::async_runtime::spawn(async move {
        while let Some(e) = rx.recv().await {
            bttv_event(&app, e);
        }
    });
}

fn bttv_event(app: &AppHandle, e: BttvEvent) {
    let state = app.state::<Emotes>();
    match e {
        BttvEvent::User {
            twitch_id,
            badge,
            emotes,
        } => {
            let mut inner = state.lock();
            if let Some(url) = &badge {
                inner
                    .cosmetics
                    .add_badge(bttv::api::badge(url, "BetterTTV Pro", url));
            }
            let changed = inner.cosmetics.set_bttv_badge(&twitch_id, badge.as_deref());
            let personal = !emotes.is_empty();
            if personal {
                let id = format!("bttv-pro-{twitch_id}");
                inner.personal_sets.insert(
                    id.clone(),
                    Arc::new(EmoteSet::new(id.clone(), Provider::Bttv, Scope::Personal, emotes)),
                );
                let list = inner.user_sets.entry(twitch_id.clone()).or_default();
                if !list.contains(&id) {
                    list.push(id);
                }
                inner.rebuild_personal(&twitch_id);
            }
            drop(inner);
            if personal {
                crate::chat::retokenize_users(app, std::slice::from_ref(&twitch_id));
            }
            if changed {
                looks::changed(app, [twitch_id]);
            }
        }
        BttvEvent::Added { channel_id, emote } => {
            let text = format!("{} was added to BTTV.", emote.name);
            change_channel(app, &channel_id, &text, |emotes| {
                if !emotes.iter().any(|e| e.id == emote.id) {
                    emotes.push(Arc::new(emote.clone()));
                }
            });
        }
        BttvEvent::Renamed { channel_id, id, name } => {
            let old = find_name(app, &channel_id, &id);
            let text = match &old {
                Some(old) => format!("BTTV emote {old} is now {name}."),
                None => format!("{name} was renamed on BTTV."),
            };
            change_channel(app, &channel_id, &text, |emotes| {
                if let Some(slot) = emotes.iter_mut().find(|e| e.id == id) {
                    let mut e = (**slot).clone();
                    e.name.clone_from(&name);
                    *slot = Arc::new(e);
                }
            });
        }
        BttvEvent::Removed { channel_id, id } => {
            let Some(name) = find_name(app, &channel_id, &id) else {
                return;
            };
            let text = format!("{name} was removed from BTTV.");
            change_channel(app, &channel_id, &text, |emotes| emotes.retain(|e| e.id != id));
        }
    }
}

fn find_name(app: &AppHandle, channel_id: &str, emote_id: &str) -> Option<String> {
    let state = app.state::<Emotes>();
    let inner = state.lock();
    inner
        .channels
        .values()
        .filter(|c| c.id == channel_id)
        .flat_map(|c| c.sets.iter())
        .filter(|s| s.provider == Provider::Bttv)
        .flat_map(|s| s.emotes.iter())
        .find(|e| e.id == emote_id)
        .map(|e| e.name.clone())
}

/// Edits the channel's BTTV set, then tells its chats (C10).
fn change_channel(
    app: &AppHandle,
    channel_id: &str,
    notice: &str,
    edit: impl Fn(&mut Vec<Arc<cyex_emotes::Emote>>),
) {
    let state = app.state::<Emotes>();
    let logins: Vec<String> = {
        let mut inner = state.lock();
        let logins: Vec<String> = inner
            .channels
            .iter()
            .filter(|(_, c)| c.id == channel_id)
            .map(|(l, _)| l.clone())
            .collect();
        for login in &logins {
            let Some(c) = inner.channels.get_mut(login) else {
                continue;
            };
            match c
                .sets
                .iter_mut()
                .find(|s| s.provider == Provider::Bttv && s.scope == Scope::Channel)
            {
                Some(set) => {
                    let mut next = (**set).clone();
                    edit(&mut next.emotes);
                    *set = Arc::new(next);
                }
                None => {
                    let mut set = EmoteSet::new(
                        format!("bttv-{channel_id}"),
                        Provider::Bttv,
                        Scope::Channel,
                        Vec::new(),
                    );
                    edit(&mut set.emotes);
                    c.sets.push(Arc::new(set));
                }
            }
            inner.rebuild(login);
        }
        logins
    };
    for login in logins {
        crate::chat::system_line(app, &login, notice);
    }
}
