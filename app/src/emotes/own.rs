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
}
