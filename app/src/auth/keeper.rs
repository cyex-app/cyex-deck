//! Background task: checks every token on start and hourly, refreshes before expiry
//! (cyex_auth::keeper decides), and refreshes names and avatars once per start.

use std::time::Duration;

use cyex_auth::keeper::{self, Outcome};
use cyex_auth::{Secret, TokenKind, TokenState, inspect, roster};
use tauri::{AppHandle, Manager};

use super::{Auth, accounts, notify, unix_now, update_accounts};

const TICK: Duration = Duration::from_secs(30);

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        refresh_profiles(&app).await;
        loop {
            check_all(&app).await;
            tokio::time::sleep(TICK).await;
        }
    });
}

async fn check_all(app: &AppHandle) {
    let auth = app.state::<Auth>();
    let mut changed = false;
    for account in accounts(app).list {
        for kind in TokenKind::ALL {
            let (Some(meta), Some(secret)) = (kind.meta(&account), auth.vault.secret(&account.user_id, kind))
            else {
                continue;
            };
            let action = keeper::next_action(
                auth.vault.state(&account.user_id, kind),
                meta.expires_at,
                secret.refresh.is_some(),
                auth.vault.since_check(&account.user_id, kind),
                unix_now(),
            );
            if let Some(action) = action {
                let outcome = keeper::run(&auth.http, &secret, action).await;
                changed |= apply(app, &account.user_id, kind, &secret, outcome);
            }
        }
    }
    if changed {
        notify(app);
    }
}

/// Stores the result. Returns true if the UI needs an update.
fn apply(app: &AppHandle, user_id: &str, kind: TokenKind, checked: &Secret, outcome: Outcome) -> bool {
    let auth = app.state::<Auth>();
    // Signed in again or removed while the check ran: the result is stale.
    if auth.vault.secret(user_id, kind).as_ref() != Some(checked) {
        return false;
    }
    match outcome {
        Outcome::Valid {
            refreshed,
            validation,
        } => {
            let mut changed = match refreshed {
                Some(fresh) => match auth.vault.put(user_id, kind, fresh) {
                    Ok(()) => true,
                    Err(e) => {
                        eprintln!("auth: couldn't save a refreshed token: {e}");
                        auth.vault.mark(user_id, kind, TokenState::Valid)
                    }
                },
                None => auth.vault.mark(user_id, kind, TokenState::Valid),
            };
            let now = unix_now();
            let _ = update_accounts(app, |a| {
                let updated = roster::record_check(a, user_id, kind, &validation, now);
                changed |= updated;
                updated
            });
            changed
        }
        Outcome::Invalid => auth.vault.mark(user_id, kind, TokenState::Invalid),
        Outcome::Offline => auth.vault.mark(user_id, kind, TokenState::Offline),
    }
}

/// Display names and avatars change on Twitch's side; public data, no token needed.
async fn refresh_profiles(app: &AppHandle) {
    let auth = app.state::<Auth>();
    let mut fresh = Vec::new();
    for account in accounts(app).list {
        if let Some(profile) = inspect::profile(&auth.http, &account.user_id, None).await {
            fresh.push((account.user_id, profile));
        }
    }
    let _ = update_accounts(app, |a| {
        let mut changed = false;
        for (user_id, (display_name, avatar)) in &fresh {
            if let Some(account) = a.get_mut(user_id)
                && (account.display_name != *display_name || account.avatar != *avatar)
            {
                account.display_name.clone_from(display_name);
                account.avatar.clone_from(avatar);
                changed = true;
            }
        }
        changed
    });
}
