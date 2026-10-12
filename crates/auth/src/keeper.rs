//! Keeps tokens alive: validates each one hourly (Twitch requires it) and refreshes it
//! before it expires.

use std::time::Duration;

use cyex_twitch::{Error, Http, id};

use crate::TokenState;
use crate::secrets::Secret;

pub const VALIDATE_EVERY: Duration = Duration::from_secs(60 * 60);
pub const RETRY_OFFLINE: Duration = Duration::from_secs(60);
/// Refresh this many seconds before expiry.
pub const REFRESH_MARGIN: i64 = 10 * 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Validate,
    Refresh,
}

/// What to do with a token right now, if anything. `now` and `expires_at` are unix seconds.
pub fn next_action(
    state: TokenState,
    expires_at: Option<i64>,
    can_refresh: bool,
    since_check: Option<Duration>,
    now: i64,
) -> Option<Action> {
    if !state.usable() {
        return None;
    }
    if can_refresh && expires_at.is_some_and(|t| t - now <= REFRESH_MARGIN) {
        return Some(Action::Refresh);
    }
    let due = match since_check {
        None => true,
        Some(elapsed) if state == TokenState::Offline => elapsed >= RETRY_OFFLINE,
        Some(elapsed) => elapsed >= VALIDATE_EVERY,
    };
    due.then_some(Action::Validate)
}

#[derive(Debug)]
pub enum Outcome {
    /// Still good. `refreshed` holds the new token if it was refreshed.
    Valid {
        refreshed: Option<Secret>,
        validation: id::Validation,
    },
    Invalid,
    Offline,
}

pub async fn run(http: &Http, secret: &Secret, action: Action) -> Outcome {
    if action == Action::Refresh {
        return refresh_then_validate(http, secret).await;
    }
    match id::validate(http, &secret.access).await {
        Ok(validation) => Outcome::Valid {
            refreshed: None,
            validation,
        },
        // Expired early or revoked: one refresh attempt before giving up.
        Err(e) if e.is_unauthorized() && secret.refresh.is_some() => {
            refresh_then_validate(http, secret).await
        }
        Err(e) => failed(&e),
    }
}

async fn refresh_then_validate(http: &Http, secret: &Secret) -> Outcome {
    let Some(refresh_token) = &secret.refresh else {
        return Outcome::Invalid;
    };
    let grant = match id::refresh(http, &secret.client_id, refresh_token).await {
        Ok(grant) => grant,
        Err(e) => return failed(&e),
    };
    let fresh = Secret {
        access: grant.access_token,
        // Refresh tokens are one-time use; the old one is spent either way.
        refresh: grant.refresh_token,
        client_id: secret.client_id.clone(),
    };
    match id::validate(http, &fresh.access).await {
        Ok(validation) => Outcome::Valid {
            refreshed: Some(fresh),
            validation,
        },
        Err(e) => failed(&e),
    }
}

fn failed(e: &Error) -> Outcome {
    if e.is_network() || e.is_server_error() {
        Outcome::Offline
    } else {
        Outcome::Invalid
    }
}

/// Unix seconds when a token with `expires_in` seconds left runs out. 0 means never.
pub fn expires_at(expires_in: u64, now: i64) -> Option<i64> {
    (expires_in > 0).then(|| now.saturating_add(i64::try_from(expires_in).unwrap_or(i64::MAX)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;
    const HOUR: Duration = VALIDATE_EVERY;

    #[test]
    fn validates_on_start_and_hourly() {
        let later = Some(NOW + 3 * 3600);
        assert_eq!(
            next_action(TokenState::Checking, later, true, None, NOW),
            Some(Action::Validate)
        );
        assert_eq!(
            next_action(TokenState::Valid, later, true, Some(HOUR / 2), NOW),
            None
        );
        assert_eq!(
            next_action(TokenState::Valid, later, true, Some(HOUR), NOW),
            Some(Action::Validate)
        );
    }

    #[test]
    fn refreshes_before_expiry_when_it_can() {
        let soon = Some(NOW + REFRESH_MARGIN - 1);
        assert_eq!(
            next_action(TokenState::Valid, soon, true, Some(HOUR / 2), NOW),
            Some(Action::Refresh)
        );
        // Pasted tokens can't refresh; they get validated and turn invalid once expired.
        assert_eq!(
            next_action(TokenState::Valid, soon, false, Some(HOUR / 2), NOW),
            None
        );
        // Non-expiring tokens never refresh.
        assert_eq!(
            next_action(TokenState::Valid, None, true, Some(HOUR / 2), NOW),
            None
        );
    }

    #[test]
    fn offline_retries_sooner_and_dead_tokens_are_left_alone() {
        let later = Some(NOW + 3 * 3600);
        assert_eq!(
            next_action(TokenState::Offline, later, true, Some(RETRY_OFFLINE), NOW),
            Some(Action::Validate)
        );
        assert_eq!(next_action(TokenState::Invalid, later, true, None, NOW), None);
        assert_eq!(next_action(TokenState::Missing, later, true, None, NOW), None);
    }

    #[test]
    fn expiry_from_seconds_left() {
        assert_eq!(expires_at(0, NOW), None);
        assert_eq!(expires_at(14400, NOW), Some(NOW + 14400));
    }
}
