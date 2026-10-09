//! Chat data from GQL (1.9, 1.40): badges and cheermotes, global once per run and per
//! channel on join; reward names for redemptions; the pinned message, checked every
//! minute while a chat is on screen (live pin updates come with Hermes in phase 9).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cyex_chat::build::Me;
use cyex_chat::model::{ChatMessage, Fragment, Pinned, Provider, Sender};
use cyex_twitch::gql::{self, BadgeInfo, ChannelChat, Cheermote, PinFragment, PinnedMessage};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tokio::sync::OnceCell;
use ts_rs::TS;

use super::{Chat, RoomKey, now_ms, queue};
use crate::auth::Auth;

const PIN_POLL: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct Catalogs {
    global: OnceCell<Arc<GlobalCatalog>>,
    channels: Mutex<HashMap<String, Arc<ChannelChat>>>,
}

/// Global badges and cheermotes, sent to the UI once.
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GlobalCatalog {
    pub badges: Vec<BadgeInfo>,
    pub cheermotes: Vec<Cheermote>,
    #[serde(skip)]
    cheer_colors: Vec<(u32, String)>,
}

/// One channel's badges and cheermotes. Looked up before the global ones.
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelCatalog {
    pub channel_id: String,
    pub badges: Vec<BadgeInfo>,
    pub cheermotes: Vec<Cheermote>,
}

pub fn init(app: &AppHandle) {
    app.manage(Catalogs::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(PIN_POLL).await;
            poll_pins(&app).await;
        }
    });
}

pub async fn global(app: &AppHandle) -> Result<Arc<GlobalCatalog>, String> {
    let catalogs = app.state::<Catalogs>();
    let http = app.state::<Auth>().http.clone();
    catalogs
        .global
        .get_or_try_init(|| async {
            let g = gql::global_chat(&http).await.map_err(|e| e.to_string())?;
            Ok(Arc::new(GlobalCatalog {
                badges: g.badges,
                cheermotes: g.cheermotes,
                cheer_colors: g.cheer_colors,
            }))
        })
        .await
        .cloned()
}

/// Cached per run unless `fresh`. `None` if the channel doesn't exist.
pub async fn channel(app: &AppHandle, login: &str, fresh: bool) -> Result<Option<Arc<ChannelChat>>, String> {
    if !fresh && let Some(c) = cached(app, login) {
        return Ok(Some(c));
    }
    let colors = global(app).await.map(|g| g.cheer_colors.clone()).unwrap_or_default();
    let http = app.state::<Auth>().http.clone();
    let Some(c) = gql::channel_chat(&http, login, &colors).await.map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let c = Arc::new(c);
    app.state::<Catalogs>()
        .channels
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(login.to_owned(), c.clone());
    Ok(Some(c))
}

pub fn cached(app: &AppHandle, login: &str) -> Option<Arc<ChannelChat>> {
    app.state::<Catalogs>()
        .channels
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(login)
        .cloned()
}

pub fn channel_view(c: &ChannelChat) -> ChannelCatalog {
    ChannelCatalog {
        channel_id: c.channel_id.clone(),
        badges: c.badges.clone(),
        cheermotes: c.cheermotes.clone(),
    }
}

/// After a fresh join: reward names into the room, then the current pin.
pub fn load_for_room(app: AppHandle, key: RoomKey) {
    tauri::async_runtime::spawn(async move {
        let c = match channel(&app, &key.1, false).await {
            Ok(Some(c)) => c,
            Ok(None) => return,
            Err(e) => {
                eprintln!("chat: no catalog for {}: {e}", key.1);
                return;
            }
        };
        let rewards: HashMap<String, (String, u32)> = c
            .points
            .iter()
            .flat_map(|p| p.custom.iter())
            .map(|r| (r.id.clone(), (r.title.clone(), r.cost)))
            .collect();
        {
            let chat = app.state::<Chat>();
            let mut inner = chat.lock();
            if let Some(entry) = inner.rooms.get_mut(&key) {
                entry.room.set_rewards(rewards);
            }
        }
        refresh_pin(&app, &key, &c.channel_id).await;
    });
}

/// A view opened on a room that was joined already: its pin may be old.
pub fn refresh_pin_now(app: AppHandle, key: RoomKey) {
    tauri::async_runtime::spawn(async move {
        if let Some(c) = cached(&app, &key.1) {
            refresh_pin(&app, &key, &c.channel_id).await;
        }
    });
}

/// Rooms on screen only. Tabs in the background catch up when shown.
async fn poll_pins(app: &AppHandle) {
    let rooms: Vec<RoomKey> = {
        let chat = app.state::<Chat>();
        let inner = chat.lock();
        inner
            .rooms
            .iter()
            .filter(|(_, e)| !e.views.is_empty())
            .map(|(k, _)| k.clone())
            .collect()
    };
    for key in rooms {
        if let Some(c) = cached(app, &key.1) {
            refresh_pin(app, &key, &c.channel_id).await;
        }
    }
}

async fn refresh_pin(app: &AppHandle, key: &RoomKey, channel_id: &str) {
    let http = app.state::<Auth>().http.clone();
    let Ok(pin) = gql::pinned(&http, channel_id).await else { return };
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let Some(entry) = inner.rooms.get_mut(key) else { return };
    let me = entry.room.me().cloned();
    let pin = pin.map(|p| to_pinned(p, me.as_ref(), entry.room.colors()));
    if let Some(ev) = entry.room.set_pinned(pin) {
        queue(entry, [ev], &chat.wake);
    }
}

fn to_pinned(p: PinnedMessage, me: Option<&Me>, colors: &mut cyex_chat::build::Colors) -> Pinned {
    let sender = p.sender.map(|s| {
        let raw = s
            .color
            .clone()
            .unwrap_or_else(|| cyex_chat::build::sender::default_color(&s.id, &s.login).to_owned());
        Sender {
            color: colors.get(&raw),
            badges: Vec::new(),
            id: s.id,
            login: s.login,
            display: s.display_name,
        }
    });
    let body = p
        .fragments
        .into_iter()
        .map(|f| match f {
            PinFragment::Text(text) => Fragment::Text { text },
            PinFragment::Emote { id, name } => Fragment::Emote {
                provider: Provider::Twitch,
                id,
                name,
            },
            PinFragment::Mention { text, login } => Fragment::Mention { text, login },
        })
        .collect();
    let own = me.is_some_and(|m| sender.as_ref().is_some_and(|s| s.id == m.user_id));
    Pinned {
        id: p.id,
        by: p.pinned_by,
        ends_at: p.ends_at,
        paid: p.kind == "PAID",
        message: ChatMessage {
            id: p.message_id,
            ts: now_ms(),
            sender,
            body,
            notice: None,
            reply: None,
            reward: None,
            source: None,
            flags: if own { vec![cyex_chat::model::Flag::Own] } else { Vec::new() },
        },
    }
}
