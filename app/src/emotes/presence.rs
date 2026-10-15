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

/// V1051-03 The account's 7TV user id, as the cosmetics menu learned it.
pub fn remember_seventv(app: &AppHandle, account: &str, id: Option<String>) {
    app.state::<Presence>()
        .seventv
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(account.to_owned(), id);
}

/// Chats the account was in this recently count for `resend`.
const RECENT: Duration = Duration::from_secs(60 * 60);

/// V1051-03 After a paint or badge change: tell 7TV again in every chat this account was
/// in lately, once, past the minute's limit, so others see the change soon.
pub fn resend(app: &AppHandle, account: &str) {
    let logins = {
        let presence = app.state::<Presence>();
        let mut last = presence.last.lock().unwrap_or_else(|p| p.into_inner());
        let recent: Vec<(String, String)> = last
            .iter()
            .filter(|(_, t)| t.elapsed() < RECENT)
            .map(|(k, _)| k.clone())
            .collect();
        let logins = due_for(account, &recent);
        for login in &logins {
            last.remove(&(account.to_owned(), login.clone()));
        }
        logins
    };
    if logins.is_empty() {
        return;
    }
    let app = app.clone();
    let account = account.to_owned();
    tauri::async_runtime::spawn(async move {
        // 7TV caches the user for a second (research §8).
        tokio::time::sleep(Duration::from_secs(1)).await;
        // `touch` still checks the presence setting and that the chat is joined, so
        // turning presence off turns this off too.
        for login in logins {
            touch(&app, &account, &login);
        }
    });
}

/// The logins of this account's chats among `keys` (account, login).
fn due_for(account: &str, keys: &[(String, String)]) -> Vec<String> {
    keys.iter()
        .filter(|(a, _)| a == account)
        .map(|(_, l)| l.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::due_for;

    #[test]
    fn picks_only_that_accounts_logins() {
        let keys = vec![
            ("1".to_owned(), "a".to_owned()),
            ("2".to_owned(), "b".to_owned()),
            ("1".to_owned(), "c".to_owned()),
        ];
        assert_eq!(due_for("1", &keys), vec!["a", "c"]);
        assert!(due_for("3", &keys).is_empty());
    }
}
