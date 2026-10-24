//! V108-7K "Paste a 7TV token": the session copied by hand from 7tv.app's localStorage in
//! the user's own browser. Same checks and the same vault entry as the sign-in window. The
//! token is never logged, and a refused one isn't logged out on 7TV: it's the browser's own
//! session.

use cyex_auth::{SevenTvToken, SevenTvTokens};
use cyex_emotes::seventv::account::{Client, Error};
use tauri::{AppHandle, Manager};

use crate::auth::{self, Auth};
use crate::emotes::presence;

/// Why a pasted token isn't kept.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    NoConsent,
    Format,
    RanOut,
    OtherAccount,
    Rejected,
    Down,
    NotSaved,
}

impl Refused {
    pub fn text(&self) -> &'static str {
        match self {
            Self::NoConsent => "Allow 7TV for this account first.",
            Self::Format => "That doesn’t look like a 7TV token.",
            Self::RanOut => "That token ran out. Sign in on 7tv.app again and copy a new one.",
            Self::OtherAccount => "This 7TV account is linked to a different Twitch account.",
            Self::Rejected => "7TV didn’t accept that token. Sign in on 7tv.app again and copy a new one.",
            Self::Down => "Couldn’t reach 7TV. Try again.",
            Self::NotSaved => "Couldn’t save the 7TV session.",
        }
    }
}

/// The token in what was pasted: trimmed, without the quotes the console shows around it.
pub(super) fn read(raw: &str) -> Option<SevenTvToken> {
    let t = raw.trim();
    let t = t
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| t.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(t);
    SevenTvToken::parse(t)
}

/// Checks the pasted token with 7TV and keeps it when its Twitch connection is `account`.
/// Returns the 7TV user id.
pub(super) async fn check(
    api: &Client,
    tokens: &SevenTvTokens,
    account: &str,
    raw: &str,
    now: i64,
) -> Result<String, Refused> {
    check_token(api, tokens, account, read(raw).ok_or(Refused::Format)?, now).await
}

/// `check` for a token already in shape (V108-7L the bookmark sends one).
pub(super) async fn check_token(
    api: &Client,
    tokens: &SevenTvTokens,
    account: &str,
    token: SevenTvToken,
    now: i64,
) -> Result<String, Refused> {
    if token.expired(now) {
        return Err(Refused::RanOut);
    }
    let me = api.me(token.secret()).await.map_err(|e| {
        eprintln!("cosmetics: {account}: pasted token check: {e}");
        match e {
            Error::SignInAgain | Error::NotAllowed => Refused::Rejected,
            _ => Refused::Down,
        }
    })?;
    if me.twitch_id.as_deref() != Some(account) {
        return Err(Refused::OtherAccount);
    }
    tokens.put(account, &token).map_err(|e| {
        eprintln!("cosmetics: {account}: couldn't keep the 7TV session: {e}");
        Refused::NotSaved
    })?;
    Ok(me.id)
}

/// Needs the account's consent, like the sign-in window.
pub async fn paste(app: &AppHandle, account: &str, raw: &str) -> Result<(), Refused> {
    if !super::has_consent(app, account) {
        return Err(Refused::NoConsent);
    }
    let id = check(&super::api(app), &app.state::<Auth>().seventv, account, raw, auth::unix_now()).await?;
    kept(app, account, id)
}

/// After a check passed: consent again (withdrawn while 7TV was asked: not kept), then the
/// same follow-up as the sign-in window.
pub(super) fn kept(app: &AppHandle, account: &str, id: String) -> Result<(), Refused> {
    let tokens = &app.state::<Auth>().seventv;
    // Withdrawn while 7TV was asked: the session isn't kept.
    if !super::has_consent(app, account) {
        let _ = tokens.forget(account);
        return Err(Refused::NoConsent);
    }
    super::state(app).clear(account);
    presence::remember_seventv(app, account, Some(id));
    auth::notify(app);
    Ok(())
}
