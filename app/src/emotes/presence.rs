//! D21 Tells 7TV and BTTV which channel a signed-in account is in, so other viewers see
//! its paint, badge and personal emotes. Sent when the account opens a chat or sends a
//! message there, at most once a minute per channel. A setting turns it off.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cyex_emotes::{bttv, seventv};
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use tauri::{AppHandle, Manager};

use super::Emotes;
use crate::auth::Auth;
use crate::settings::Settings;

const EVERY: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct Presence {
    last: Mutex<HashMap<(String, String), Instant>>,
    /// Each account's 7TV user id, `None` when it has no 7TV profile.
    seventv: Mutex<HashMap<String, Option<String>>>,
}

pub fn init(app: &AppHandle) {
    app.manage(Presence::default());
}

/// The account is in `login`'s chat.
pub fn touch(app: &AppHandle, account: &str, login: &str) {
    if !app
        .state::<Settings>()
        .read::<ChatSettings>(Partition::Chat)
        .share_presence
    {
        return;
    }
    let Some(channel_id) = super::channel_id(app, login) else {
        return;
    };
    let presence = app.state::<Presence>();
    {
        let mut last = presence.last.lock().unwrap_or_else(|p| p.into_inner());
        let key = (account.to_owned(), login.to_owned());
        if last.get(&key).is_some_and(|t| t.elapsed() < EVERY) {
            return;
        }
        last.insert(key, Instant::now());
    }
    app.state::<Emotes>()
        .bttv
        .send(bttv::socket::broadcast_me(account, &channel_id));
    let app = app.clone();
    let account = account.to_owned();
    tauri::async_runtime::spawn(async move {
        let http = app.state::<Auth>().http.clone();
        let known = app
            .state::<Presence>()
            .seventv
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&account)
            .cloned();
        let user = match known {
            Some(u) => u,
            None => {
                let Ok(u) = seventv::api::user_id(&http, &account).await else {
                    return;
                };
                app.state::<Presence>()
                    .seventv
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .insert(account.clone(), u.clone());
                u
            }
        };
        if let Some(user) = user
            && let Err(e) = seventv::api::post_presence(&http, &user, &channel_id).await
        {
            eprintln!("emotes: 7TV presence failed: {e}");
        }
    });
}
