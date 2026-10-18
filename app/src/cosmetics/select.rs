//! V1051-02/03 Picking a paint or badge: only the latest pick per account and kind goes
//! to 7TV, one call at a time. Batch 3's `/paint`, `/badge` and `/combo` call this too.

use std::sync::Arc;

use cyex_auth::SevenTvTokens;
use cyex_emotes::seventv::account::Error;
use cyex_emotes::seventv::account::model::Inventory;
use cyex_emotes::seventv::account::view::badge_item;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{Cosmetics, has_consent, lock, release_later, state};
use crate::auth::{self, Auth};
use crate::emotes::{self, Worn, presence};

#[derive(Clone, Copy, Debug, Deserialize, TS, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Kind {
    Paint,
    Badge,
}

#[derive(Debug, Serialize, TS, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum Picked {
    Done {
        id: Option<String>,
    },
    /// A newer pick came while this one waited. Nothing was sent.
    Superseded,
}

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum SelectError {
    SignInAgain,
    NotAllowed,
    RateLimited {
        #[ts(type = "number")]
        seconds: u64,
    },
    Down,
    NoConsent,
    SignedOut,
}

/// Whether a newer pick (`latest`) overtook this one (`mine`).
pub fn superseded(latest: u64, mine: u64) -> bool {
    latest > mine
}

fn from_api(e: Error) -> SelectError {
    match e {
        Error::SignInAgain => SelectError::SignInAgain,
        Error::NotAllowed => SelectError::NotAllowed,
        Error::RateLimited { seconds } => SelectError::RateLimited { seconds },
        Error::Down | Error::Bad => SelectError::Down,
    }
}

/// Everything one pick needs besides the client.
pub(super) struct Pick<'a> {
    pub tokens: &'a SevenTvTokens,
    pub account: &'a str,
    pub kind: Kind,
    pub id: Option<String>,
    pub mine: u64,
    pub user: Option<String>,
    pub now: i64,
    /// Asked again once it is this pick's turn: the consent may have been withdrawn while it waited.
    pub consent: &'a (dyn Fn() -> bool + Sync),
}

impl Cosmetics {
    /// Counts a new pick; the number says which one is newest.
    pub(super) fn ticket(&self, account: &str, kind: Kind) -> u64 {
        let mut latest = lock(&self.latest);
        let n = latest.entry((account.to_owned(), kind)).or_insert(0);
        *n += 1;
        *n
    }

    fn newest(&self, account: &str, kind: Kind) -> u64 {
        lock(&self.latest)
            .get(&(account.to_owned(), kind))
            .copied()
            .unwrap_or(0)
    }

    fn gate(&self, account: &str, kind: Kind) -> super::Gate {
        lock(&self.gates)
            .entry((account.to_owned(), kind))
            .or_default()
            .clone()
    }

    /// Waits for the account's turn, then sends the pick unless a newer one came.
    pub(super) async fn send(&self, p: Pick<'_>) -> Result<Picked, SelectError> {
        let Pick {
            tokens,
            account,
            kind,
            id,
            mine,
            user,
            now,
            consent,
        } = p;
        let gate = self.gate(account, kind);
        let _turn = gate.lock().await;
        if superseded(self.newest(account, kind), mine) {
            return Ok(Picked::Superseded);
        }
        if !consent() {
            return Err(SelectError::NoConsent);
        }
        let Some(token) = tokens.get(account) else {
            return Err(SelectError::SignedOut);
        };
        if token.expired(now) {
            return Err(SelectError::SignInAgain);
        }
        let Some(user) = user.or_else(|| token.user_id()) else {
            return Err(SelectError::SignInAgain);
        };
        let sent = match kind {
            Kind::Paint => self.api.set_paint(token.secret(), &user, id.as_deref()).await,
            Kind::Badge => self.api.set_badge(token.secret(), &user, id.as_deref()).await,
        };
        match sent {
            Ok(_) => Ok(Picked::Done { id }),
            Err(Error::SignInAgain) => {
                // Rejected: that token is dead, so it goes - unless a new sign-in replaced it meanwhile.
                if forget_if_same(tokens, account, &token) {
                    Err(SelectError::SignInAgain)
                } else {
                    Err(SelectError::Down)
                }
            }
            Err(e) => Err(from_api(e)),
        }
    }

    /// The cached inventory with this kind's active id changed.
    fn set_active(&self, account: &str, kind: Kind, id: Option<&str>) {
        let mut cache = lock(&self.cache);
        let Some((_, inv)) = cache.get_mut(account) else {
            return;
        };
        let mut next: Inventory = (**inv).clone();
        let slot = match kind {
            Kind::Paint => &mut next.paint,
            Kind::Badge => &mut next.badge,
        };
        *slot = id.map(str::to_owned);
        *inv = Arc::new(next);
    }
}

/// 7TV rejected `used`: deleted if still stored, and the account says "Sign in again" until the
/// next sign-in. False when a new sign-in replaced it meanwhile.
pub(crate) fn forget_if_same(tokens: &SevenTvTokens, account: &str, used: &cyex_auth::SevenTvToken) -> bool {
    match tokens.get(account) {
        // A sibling pick in the same burst already rejected it (combos 18): still "Sign in again".
        None => return tokens.rejected(account),
        Some(t) if t.secret() != used.secret() => return false,
        Some(_) => {}
    }
    if let Err(e) = tokens.reject(account) {
        eprintln!("cosmetics: {account}: couldn't delete the 7TV session: {e}");
    }
    true
}

/// What the own look shows after the pick.
fn worn(inv: &Inventory, kind: Kind, id: Option<&str>) -> Worn {
    match kind {
        Kind::Paint => Worn::Paint(id.and_then(|id| {
            inv.paints
                .iter()
                .find(|e| e.style.id == id)
                .map(|e| e.style.clone())
        })),
        Kind::Badge => {
            Worn::Badge(id.and_then(|id| inv.badges.iter().find(|e| e.id == id).map(|e| badge_item(e).def)))
        }
    }
}

/// The pick: `id` of `None` takes the paint or badge off.
pub async fn select(
    app: &AppHandle,
    account: &str,
    kind: Kind,
    id: Option<String>,
) -> Result<Picked, SelectError> {
    if !has_consent(app, account) {
        return Err(SelectError::NoConsent);
    }
    let st = state(app);
    let mine = st.ticket(account, kind);
    let user = st.cached(account).map(|(_, i)| i.seventv_id.clone());
    let done = st
        .send(Pick {
            tokens: &app.state::<Auth>().seventv,
            account,
            kind,
            id,
            mine,
            user,
            now: auth::unix_now(),
            consent: &|| has_consent(app, account),
        })
        .await
        .inspect_err(|e| {
            eprintln!("cosmetics: {account}: {e:?}");
            // The session was deleted: Settings → Accounts stops saying "Signed in until …".
            if *e == SelectError::SignInAgain {
                auth::notify(app);
            }
        })?;
    let Picked::Done { id } = &done else {
        return Ok(done);
    };
    st.set_active(account, kind, id.as_deref());
    // The cached copy has the new id; a fresh read could still answer with the old one.
    let inv = match st.cached(account) {
        Some((_, inv)) => Some(inv),
        None => {
            let fetched = st.inventory(account, false).await.ok().flatten();
            // A refill is let go like the menu's: it isn't kept for good.
            release_later(app, account);
            fetched
        }
    };
    if let Some(inv) = inv {
        emotes::wear(app, account, worn(&inv, kind, id.as_deref()));
    }
    presence::resend(app, account);
    Ok(done)
}
