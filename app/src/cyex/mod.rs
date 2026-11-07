//! V109-B02 cyex cosmetics per account: a cyex session made from the account's own Twitch token
//! (no new sign-in), what it owns and wears (`/me`), equip changes and codes. The session token
//! lives in the token store (`<twitch id>.cyex`), never in settings or logs. Chat badges
//! themselves are `crate::emotes::cyex`.

mod api;
mod model;
mod words;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use cyex_auth::secrets::{KeyringStore, MemoryStore, SecretStore};
use cyex_auth::{CyexSession, CyexSessions};
use cyex_emotes::badges::cyex::{self as badges, Held, Images};
use cyex_twitch::Http;
use serde_json::Value;
use tauri::{AppHandle, Manager};

pub use model::{CyexEquipped, CyexHistory, CyexMe, CyexView};

use api::ApiError;

/// Mocks builds only: a local cyex API (`http://127.0.0.1:<port>`, test data folder only).
#[cfg(feature = "mocks")]
const URL_VAR: &str = "CYEX_API_URL";

pub struct Cyex {
    sessions: CyexSessions,
    /// The last `/me` per account, for the chat preview and equip changes.
    me: Mutex<HashMap<String, CyexMe>>,
    /// One session made at a time.
    making: tokio::sync::Mutex<()>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// After `auth::init`.
pub fn init(app: &AppHandle) {
    // Test runs keep sessions in memory, like 7TV's.
    let store: Arc<dyn SecretStore> = if crate::paths::webview(app).is_some() {
        Arc::new(MemoryStore::default())
    } else {
        Arc::new(KeyringStore)
    };
    app.manage(Cyex {
        sessions: CyexSessions::new(store),
        me: Mutex::default(),
        making: tokio::sync::Mutex::new(()),
    });
}

/// `https://api.cyex.app`, or the mock in a mocks build. A mock set but refused goes nowhere.
pub fn base(app: &AppHandle) -> String {
    #[cfg(feature = "mocks")]
    match crate::mock_url::gated(app, &[URL_VAR], "cyex API") {
        Ok(Some(b)) => return b,
        Ok(None) => {}
        Err(_) => return "http://127.0.0.1:0".into(),
    }
    let _ = app;
    badges::BASE.to_owned()
}

fn http(app: &AppHandle) -> Http {
    app.state::<crate::auth::Auth>().http.clone()
}

/// The account's Twitch tokens, the deck's own first, each with whether it is a Helix (Sign in
/// with Twitch) one. A mock never gets a real one.
fn twitch_tokens(app: &AppHandle, account: &str, mocked: bool) -> Vec<(String, bool)> {
    if mocked {
        return vec![(format!("mock-token:{account}"), true)];
    }
    let mut out: Vec<(String, bool)> = [(crate::auth::helix_token(app, account), true), (crate::auth::gql_token(app, account), false)]
        .into_iter()
        .filter_map(|(t, helix)| t.map(|(access, _)| (access, helix)))
        .collect();
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

enum Op<'a> {
    Me,
    Equip(Option<&'a str>, bool),
    Redeem(&'a str),
    History,
    Restore(u64),
    Preview(&'a str),
}

enum Out {
    Me(Box<CyexMe>),
    Equipped(CyexEquipped),
    Reward(Value),
    History(CyexHistory),
    Done,
    Bytes(Vec<u8>),
}

enum Fail {
    /// No Helix token (Sign in with Twitch) on the account.
    NoToken,
    /// V109-B16 A Helix token the server refused.
    Refused,
    Api(ApiError),
}

async fn run(http: &Http, base: &str, token: &str, op: &Op<'_>) -> Result<Out, ApiError> {
    match op {
        Op::Me => api::me(http, base, token).await.map(|m| Out::Me(Box::new(m))),
        Op::Equip(main, custom) => api::equip(http, base, token, *main, *custom).await.map(Out::Equipped),
        Op::Redeem(code) => api::redeem(http, base, token, code).await.map(Out::Reward),
        Op::History => api::history(http, base, token).await.map(|h| Out::History(h.tidy())),
        Op::Restore(id) => api::restore(http, base, token, *id).await.map(|()| Out::Done),
        Op::Preview(url) => api::preview(http, url, token).await.map(Out::Bytes),
    }
}

/// A new session from the account's Twitch token. A token the server won't take (other client,
/// revoked) makes way for the next.
async fn make(app: &AppHandle, account: &str, http: &Http, base: &str) -> Result<String, Fail> {
    let st = app.state::<Cyex>();
    let _one = st.making.lock().await;
    let mut last = Fail::NoToken;
    for (access, helix) in twitch_tokens(app, account, base != badges::BASE) {
        match api::session(http, base, &access).await {
            Ok(s) => {
                let Some(session) = CyexSession::new(&s.session_token, &s.expires_at) else {
                    return Err(Fail::Api(ApiError::Offline));
                };
                if let Err(e) = st.sessions.put(account, &session) {
                    eprintln!("cyex: couldn't keep a session: {e}");
                }
                return Ok(session.token);
            }
            Err(e) if matches!(e.code(), "wrong_client" | "bad_token") => {
                if helix {
                    last = Fail::Refused;
                }
            }
            Err(e) => return Err(Fail::Api(e)),
        }
    }
    Err(last)
}

/// Runs `op` with the account's session; a session the server forgot is made again once, then
/// it gives up quietly.
async fn with_session(app: &AppHandle, account: &str, op: Op<'_>) -> Result<Out, Fail> {
    let (http, base) = (http(app), base(app));
    let kept = app.state::<Cyex>().sessions.get(account).map(|s| s.token);
    let fresh = kept.is_none();
    let token = match kept {
        Some(t) => t,
        None => make(app, account, &http, &base).await?,
    };
    match run(&http, &base, &token, &op).await {
        Err(e) if e.signed_out() && !fresh => {
            let _ = app.state::<Cyex>().sessions.forget(account);
            let token = make(app, account, &http, &base).await?;
            run(&http, &base, &token, &op).await.map_err(Fail::Api)
        }
        r => r.map_err(Fail::Api),
    }
}

fn known(app: &AppHandle, account: &str) -> bool {
    crate::auth::accounts(app).get(account).is_some()
}

/// What the account's chat lines show now, from `equipped.shown`. V109-B10 Any one-time badge
/// the account owns, with its number.
pub(crate) fn own_list(me: &CyexMe) -> Vec<Held> {
    let mut out = Vec::new();
    for id in me.equipped.shown.iter().take(2) {
        let held = match id.as_str() {
            badges::CUSTOM => me.plus.live.as_ref().map(|p| Held {
                id: badges::CUSTOM.into(),
                number: None,
                images: Some(Images {
                    x1: p.x1.clone(),
                    x2: p.x2.clone(),
                    x4: p.x4.clone(),
                }),
            }),
            kind if badges::kind_ok(kind) && me.owns(kind) => Some(Held {
                id: kind.to_owned(),
                number: me.number_of(kind),
                images: None,
            }),
            _ => None,
        };
        out.extend(held);
    }
    out
}

fn keep_me(app: &AppHandle, account: &str, me: &CyexMe) {
    lock(&app.state::<Cyex>().me).insert(account.to_owned(), me.clone());
    // The account's own chat shows the change at once, not after the next snapshot.
    crate::emotes::cyex::set_own(app, account, Some(own_list(me)));
}

pub async fn view(app: &AppHandle, account: &str) -> CyexView {
    if !known(app, account) {
        return CyexView::NoToken;
    }
    match with_session(app, account, Op::Me).await {
        Ok(Out::Me(me)) => {
            keep_me(app, account, &me);
            let held: Vec<String> = me.badges.iter().map(|b| b.id.clone()).collect();
            let kinds = crate::emotes::cyex::catalog(app, &held)
                .await
                .into_iter()
                .map(|k| model::CyexKind {
                    id: k.id,
                    title: k.title,
                    images: model::CyexPics {
                        x1: k.images.x1,
                        x2: k.images.x2,
                        x4: k.images.x4,
                    },
                    stills: k.still.map(|s| model::CyexPics { x1: s.x1, x2: s.x2, x4: s.x4 }),
                })
                .collect();
            CyexView::Ready { me, kinds }
        }
        Ok(_) | Err(Fail::Api(ApiError::Offline)) => CyexView::Offline,
        Err(f) => failed_view(&f),
    }
}

/// V109-B16 What the pop-up shows when the session failed: nothing to sign in with, a sign-in the
/// server refused, or offline.
fn failed_view(f: &Fail) -> CyexView {
    match f {
        Fail::NoToken => CyexView::NoToken,
        Fail::Refused => CyexView::Refused,
        Fail::Api(e) if e.signed_out() || matches!(e.code(), "bad_token" | "wrong_client") => CyexView::Refused,
        Fail::Api(_) => CyexView::Offline,
    }
}

fn fail_text(f: Fail) -> String {
    match f {
        Fail::NoToken | Fail::Refused => "cyex didn’t take this account’s sign-in. Sign in again.".to_owned(),
        Fail::Api(e) => words::error(&e),
    }
}

pub async fn equip(app: &AppHandle, account: &str, main: Option<String>, custom: bool) -> Result<CyexEquipped, String> {
    if !known(app, account) {
        return Err("Unknown account.".into());
    }
    // V109-B10 Any one-time badge; the server checks it's owned.
    if main.as_deref().is_some_and(|m| m == badges::CUSTOM || !badges::kind_ok(m)) {
        return Err("Pick one of your badges or None.".into());
    }
    match with_session(app, account, Op::Equip(main.as_deref(), custom)).await {
        Ok(Out::Equipped(eq)) => {
            let me = {
                let st = app.state::<Cyex>();
                let mut map = lock(&st.me);
                map.get_mut(account).map(|m| {
                    m.equipped = eq.clone();
                    m.clone()
                })
            };
            if let Some(me) = me {
                keep_me(app, account, &me);
            }
            Ok(eq)
        }
        Ok(_) => Err(words::OFFLINE.into()),
        Err(f) => Err(fail_text(f)),
    }
}

pub async fn redeem(app: &AppHandle, account: &str, code: &str) -> Result<String, String> {
    let code = code.trim();
    if code.is_empty() || code.len() > 64 || code.chars().any(char::is_control) {
        return Err("That code doesn’t work.".into());
    }
    if !known(app, account) {
        return Err("Unknown account.".into());
    }
    match with_session(app, account, Op::Redeem(code)).await {
        Ok(Out::Reward(r)) => Ok(words::reward(&r)),
        Ok(_) => Err(words::OFFLINE.into()),
        Err(f) => Err(fail_text(f)),
    }
}

/// V109-B06 The account's Supporter+ uploads, for the panel. Rejects with the message to show.
pub async fn history(app: &AppHandle, account: &str) -> Result<CyexHistory, String> {
    if !known(app, account) {
        return Err("Unknown account.".into());
    }
    match with_session(app, account, Op::History).await {
        Ok(Out::History(h)) => Ok(h),
        Ok(_) => Err(words::OFFLINE.into()),
        Err(f) => Err(fail_text(f)),
    }
}

/// V109-B06 "Use again": an earlier approved upload goes live. Rejects with the message to show.
pub async fn restore(app: &AppHandle, account: &str, id: u64) -> Result<(), String> {
    if !known(app, account) {
        return Err("Unknown account.".into());
    }
    match with_session(app, account, Op::Restore(id)).await {
        Ok(Out::Done) => Ok(()),
        Ok(_) => Err(words::OFFLINE.into()),
        Err(f) => Err(fail_text(f)),
    }
}

/// V109-B07 The picture of one of the account's own uploads that isn't public yet (pending,
/// rejected): only an owner preview URL on the cyex API. Rejects with the message to show.
pub async fn preview(app: &AppHandle, account: &str, url: &str) -> Result<Vec<u8>, String> {
    if !known(app, account) {
        return Err("Unknown account.".into());
    }
    if !api::preview_ok(&base(app), url) {
        return Err("That picture isn’t yours to load.".into());
    }
    match with_session(app, account, Op::Preview(url)).await {
        Ok(Out::Bytes(b)) => Ok(b),
        Ok(_) => Err(words::OFFLINE.into()),
        Err(f) => Err(fail_text(f)),
    }
}

/// The account was removed: its session ends on the server (best effort) and here.
pub fn forget_account(app: &AppHandle, account: &str) {
    let Some(st) = app.try_state::<Cyex>() else {
        return;
    };
    let token = st.sessions.get(account).map(|s| s.token);
    let _ = st.sessions.forget(account);
    lock(&st.me).remove(account);
    crate::emotes::cyex::set_own(app, account, None);
    if let Some(token) = token {
        let (http, base) = (http(app), base(app));
        tauri::async_runtime::spawn(async move { api::end(&http, &base, &token).await });
    }
}

#[cfg(test)]
mod tests {
    use super::model::{CyexOwned, CyexPics};
    use super::*;

    fn me(shown: &[&str]) -> CyexMe {
        let mut m = CyexMe {
            badges: vec![
                CyexOwned {
                    id: "founder".into(),
                    number: Some(37),
                },
                CyexOwned {
                    id: "supporter".into(),
                    number: None,
                },
            ],
            ..CyexMe::default()
        };
        m.plus.live = Some(CyexPics {
            x1: "https://a/1.png".into(),
            x2: "https://a/2.png".into(),
            x4: "https://a/4.png".into(),
        });
        m.equipped.shown = shown.iter().map(|s| (*s).to_owned()).collect();
        m
    }

    #[test]
    fn sign_in_states_tell_none_from_refused() {
        assert_eq!(failed_view(&Fail::NoToken), CyexView::NoToken);
        assert_eq!(failed_view(&Fail::Refused), CyexView::Refused);
        assert_eq!(failed_view(&Fail::Api(ApiError::Offline)), CyexView::Offline);
    }

    #[test]
    fn own_chat_shows_what_is_equipped() {
        let l = own_list(&me(&["founder", "custom"]));
        assert_eq!(l.len(), 2);
        assert_eq!((l[0].id.as_str(), l[0].number), ("founder", Some(37)));
        assert_eq!(l[1].images.as_ref().unwrap().x2, "https://a/2.png");
        assert!(own_list(&me(&[])).is_empty(), "both off: nothing shows");
        assert_eq!(own_list(&me(&["supporter", "custom", "founder"])).len(), 2, "two at most");
        let mut streak = me(&["streak-1y", "custom"]);
        streak.badges.push(CyexOwned {
            id: "streak-1y".into(),
            number: Some(4),
        });
        let l = own_list(&streak);
        assert_eq!((l[0].id.as_str(), l[0].number), ("streak-1y", Some(4)), "V109-B10 any owned kind, numbered");
        assert!(own_list(&me(&["gold"])).is_empty(), "not owned: nothing");
        let mut bare = me(&["founder", "custom"]);
        bare.badges.clear();
        bare.plus.live = None;
        assert!(own_list(&bare).is_empty(), "nothing owned: nothing shows");
    }
}
