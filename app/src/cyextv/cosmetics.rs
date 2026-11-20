//! V112-16 The picker's cyex.tv side: the badges and paints the account owns on cyex.tv
//! (`myCosmetics`) and the pick (`selectCosmetics`), None included.

use cyex_emotes::cyextv::gql::{Choice, Owned};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{gql, with_bearer};
use crate::auth::Auth;

#[derive(Debug, Serialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export)]
pub enum TvCosmetics {
    SignedOut,
    /// cyex.tv ended the session: "Sign in again".
    Ended,
    Ready { owned: Owned },
    Offline { message: String },
}

#[derive(Clone, Copy, Debug, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TvKind {
    Badge,
    Paint,
}

fn signed_out(app: &AppHandle, account: &str) -> TvCosmetics {
    if app.state::<Auth>().cyextv.ended(account) {
        TvCosmetics::Ended
    } else {
        TvCosmetics::SignedOut
    }
}

pub async fn view(app: &AppHandle, account: &str) -> TvCosmetics {
    let Some(was) = app.state::<Auth>().cyextv.who(account) else {
        return signed_out(app, account);
    };
    let api = gql(app);
    let got = with_bearer(app, account, |t| {
        let api = api.clone();
        async move { api.my_cosmetics(&t).await }
    })
    .await;
    match got {
        Ok(owned) => {
            let who = cyex_auth::cyextv::sessions::Who {
                id: owned.account.id.clone(),
                name: owned.account.display_name.clone(),
            };
            if who != was {
                app.state::<Auth>().cyextv.rename(account, &who).await;
                crate::auth::notify(app);
            }
            TvCosmetics::Ready { owned }
        }
        // The session went while asking.
        Err(_) if app.state::<Auth>().cyextv.who(account).is_none() => signed_out(app, account),
        Err(message) => TvCosmetics::Offline { message },
    }
}

/// Wears `id` (or nothing, with `None`) of `kind`; the other kind stays. Errs with the line to show.
pub async fn select(app: &AppHandle, account: &str, kind: TvKind, id: Option<String>) -> Result<Owned, String> {
    let api = gql(app);
    with_bearer(app, account, |t| {
        let api = api.clone();
        let id = id.clone();
        async move {
            let pick = id.as_deref().map_or(Choice::None, Choice::Wear);
            let (badge, paint) = match kind {
                TvKind::Badge => (pick, Choice::Keep),
                TvKind::Paint => (Choice::Keep, pick),
            };
            api.select_cosmetics(&t, badge, paint, None).await
        }
    })
    .await
}
