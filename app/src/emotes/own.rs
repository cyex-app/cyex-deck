//! V108-E1 The account's own 7TV paint and badge on its own lines. 7TV sends them only after
//! a presence (off in Settings, or sent before the app's socket listens in that chat), so on
//! opening a chat as the account its profile is read once (a public read, no token), at most
//! every few minutes per account.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use super::looks::{self, Worn};
use super::presence;

/// A read again only after this long; a paint changed elsewhere shows by then.
const EVERY: Duration = Duration::from_secs(10 * 60);

#[derive(Default)]
pub struct Own {
    /// Account → when its look was last asked for.
    asked: Mutex<HashMap<String, Instant>>,
}

pub fn init(app: &AppHandle) {
    app.manage(Own::default());
}

/// True when `account` is due a read, and marks it asked.
fn due(asked: &mut HashMap<String, Instant>, account: &str, now: Instant) -> bool {
    if asked
        .get(account)
        .is_some_and(|at| now.saturating_duration_since(*at) < EVERY)
    {
        return false;
    }
    asked.insert(account.to_owned(), now);
    true
}

/// The account opened a chat: its own look, read if due.
pub fn refresh(app: &AppHandle, account: &str) {
    let Some(own) = app.try_state::<Own>() else {
        return;
    };
    if !due(&mut own.asked.lock().unwrap_or_else(|p| p.into_inner()), account, Instant::now()) {
        return;
    }
    let app = app.clone();
    let account = account.to_owned();
    tauri::async_runtime::spawn(async move {
        let api = crate::cosmetics::api(&app);
        match api.wearing(&account).await {
            Ok(Some(w)) => {
                eprintln!(
                    "emotes: own 7TV look for {account}: paint {:?}, badge {:?}",
                    w.paint.as_ref().map(|p| p.id.as_str()),
                    w.badge.as_ref().map(|b| b.id.as_str())
                );
                presence::remember_seventv(&app, &account, Some(w.seventv_id));
                looks::wear(&app, &account, Worn::Paint(w.paint));
                looks::wear(&app, &account, Worn::Badge(w.badge));
            }
            Ok(None) => presence::remember_seventv(&app, &account, None),
            Err(e) => {
                eprintln!("emotes: own 7TV look for {account}: {e}");
                // Asked again on the next chat opened.
                if let Some(own) = app.try_state::<Own>() {
                    own.asked.lock().unwrap_or_else(|p| p.into_inner()).remove(&account);
                }
            }
        }
    });
}

/// V109-F78 Twitch ids are digits; anything else never reaches 7TV.
fn is_twitch_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit())
}

/// V109-F78 Most users kept in the asked list.
const MAX_ASKED: usize = 2000;

/// What a card shows for someone else: 7TV pushes a paint and badge only for chatters it met
/// through a presence in a chat this app listens to, so a user from the Mentions feed, a
/// channel not open here, or another window may have none. Reads their profile (public, no
/// token), at most every few minutes per user, and gives only what they wear.
pub fn card_look(app: &AppHandle, twitch_id: &str) {
    if !is_twitch_id(twitch_id) {
        return;
    }
    let Some(own) = app.try_state::<Own>() else {
        return;
    };
    {
        let mut asked = own.asked.lock().unwrap_or_else(|p| p.into_inner());
        if asked.len() >= MAX_ASKED {
            asked.clear();
        }
        if !due(&mut asked, twitch_id, Instant::now()) {
            return;
        }
    }
    let app = app.clone();
    let id = twitch_id.to_owned();
    tauri::async_runtime::spawn(async move {
        match crate::cosmetics::api(&app).wearing(&id).await {
            Ok(Some(w)) => {
                if w.paint.is_some() {
                    looks::wear(&app, &id, Worn::Paint(w.paint));
                }
                if w.badge.is_some() {
                    looks::wear(&app, &id, Worn::Badge(w.badge));
                }
            }
            Ok(None) => {}
            Err(_) => {
                if let Some(own) = app.try_state::<Own>() {
                    own.asked.lock().unwrap_or_else(|p| p.into_inner()).remove(&id);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn once_per_account_until_it_is_old() {
        let (mut asked, t) = (HashMap::new(), Instant::now());
        assert!(due(&mut asked, "1", t));
        assert!(!due(&mut asked, "1", t + Duration::from_secs(60)));
        assert!(due(&mut asked, "2", t + Duration::from_secs(60)));
        assert!(due(&mut asked, "1", t + EVERY));
        assert!(!due(&mut asked, "1", t + EVERY + Duration::from_secs(1)));
    }

    // V109-F78
    #[test]
    fn card_lookups_take_twitch_ids_only() {
        assert!(is_twitch_id("71092938"));
        assert!(!is_twitch_id(""));
        assert!(!is_twitch_id("12 3"));
        assert!(!is_twitch_id("abc\"){"));
    }

    // V109-F78
    #[test]
    fn a_card_user_is_read_once_until_it_is_old() {
        let (mut asked, t) = (HashMap::new(), Instant::now());
        assert!(due(&mut asked, "71092938", t));
        assert!(!due(&mut asked, "71092938", t + Duration::from_secs(5)));
    }
}
