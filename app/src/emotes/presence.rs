//! D21 V106-08 "Send presence to 7TV and BTTV": tells 7TV and BTTV which chat a signed-in
//! account is in, so others see its paint, badge and personal emotes. 7TV as the 7TV extension
//! does it (on opening a chat, passive, every time; on sending, at most every 10 s per channel,
//! to every channel of a shared chat), BTTV at most once a minute. The switch turns all of it off.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cyex_emotes::{bttv, seventv};
use cyex_settings::schema::Chat as ChatSettings;
use cyex_settings::Partition;
use tauri::{AppHandle, Manager};

use super::Emotes;
use crate::auth::Auth;
use crate::settings::Settings;

const SEVENTV_EVERY: Duration = Duration::from_secs(10);
const BTTV_EVERY: Duration = Duration::from_secs(60);

/// Why a presence goes out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// A view opened the chat as the account. 7TV: passive, never held back (O2).
    Opened,
    /// The account sent a message there.
    Sent,
    /// V1051-03 Paint or badge changed.
    Restyled,
}

/// When one (account, chat) last told BTTV, and was last used.
#[derive(Clone, Copy, Debug, Default)]
struct Told {
    bttv: Option<Instant>,
    seen: Option<Instant>,
}

#[derive(Default)]
struct State {
    /// By (account, login).
    told: HashMap<(String, String), Told>,
    /// By (account, channel id): last normal 7TV presence; each channel waits its own 10 s (O4).
    sent: HashMap<(String, String), Instant>,
}

/// What goes out now: 7TV posts (channel id, passive) and whether BTTV is told.
#[derive(Debug, PartialEq, Eq)]
struct Out {
    seventv: Vec<(String, bool)>,
    bttv: bool,
}

fn due(last: Option<Instant>, every: Duration, now: Instant) -> bool {
    last.is_none_or(|l| now.saturating_duration_since(l) >= every)
}

/// `ids`: the chat's channel id first, then the other channels of its shared chat (Sent only).
fn plan(s: &mut State, account: &str, login: &str, ids: &[String], why: Why, now: Instant) -> Out {
    s.sent.retain(|_, t| now.saturating_duration_since(*t) < RECENT);
    s.told.retain(|_, t| {
        [t.bttv, t.seen]
            .into_iter()
            .flatten()
            .any(|at| now.saturating_duration_since(at) < RECENT)
    });
    let t = s.told.entry((account.to_owned(), login.to_owned())).or_default();
    let mut out = Out {
        seventv: Vec::new(),
        bttv: false,
    };
    if why != Why::Restyled {
        t.seen = Some(now);
        if due(t.bttv, BTTV_EVERY, now) {
            t.bttv = Some(now);
            out.bttv = true;
        }
    }
    match why {
        Why::Opened => out.seventv.extend(ids.first().map(|id| (id.clone(), true))),
        Why::Sent => {
            for id in ids {
                let key = (account.to_owned(), id.clone());
                if due(s.sent.get(&key).copied(), SEVENTV_EVERY, now) {
                    s.sent.insert(key, now);
                    out.seventv.push((id.clone(), false));
                }
            }
        }
        Why::Restyled => {
            if let Some(id) = ids.first() {
                s.sent.insert((account.to_owned(), id.clone()), now);
                out.seventv.push((id.clone(), false));
            }
        }
    }
    out
}

#[derive(Default)]
pub struct Presence {
    state: Mutex<State>,
    /// Each account's 7TV user id, `None` when it has no 7TV profile.
    seventv: Mutex<HashMap<String, Option<String>>>,
}

pub fn init(app: &AppHandle) {
    app.manage(Presence::default());
}

/// The account is in `login`'s chat. `shared`: the other channel ids of its shared chat,
/// told on `Why::Sent` only.
pub fn touch(app: &AppHandle, account: &str, login: &str, why: Why, shared: &[String]) {
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
    let mut ids = vec![channel_id.clone()];
    for id in shared {
        if !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    let out = {
        let presence = app.state::<Presence>();
        let mut state = presence.state.lock().unwrap_or_else(|p| p.into_inner());
        plan(&mut state, account, login, &ids, why, Instant::now())
    };
    if out.bttv {
        app.state::<Emotes>()
            .bttv
            .send(bttv::socket::broadcast_me(account, &channel_id));
    }
    if out.seventv.is_empty() {
        return;
    }
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
        let Some(user) = user else { return };
        for (id, passive) in out.seventv {
            if let Err(e) = seventv::api::post_presence(&http, &user, &id, passive).await {
                eprintln!("emotes: 7TV presence failed: {e}");
            }
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
/// in lately, once, past the 10 s limit, so others see the change soon.
pub fn resend(app: &AppHandle, account: &str) {
    let logins = {
        let presence = app.state::<Presence>();
        let state = presence.state.lock().unwrap_or_else(|p| p.into_inner());
        let recent: Vec<(String, String)> = state
            .told
            .iter()
            .filter(|(_, t)| t.seen.is_some_and(|s| s.elapsed() < RECENT))
            .map(|(k, _)| k.clone())
            .collect();
        due_for(account, &recent)
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
            touch(&app, &account, &login, Why::Restyled, &[]);
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
    use super::*;

    fn at(base: Instant, secs: u64) -> Instant {
        base.checked_add(Duration::from_secs(secs)).unwrap()
    }

    fn go(s: &mut State, why: Why, ids: &[&str], now: Instant) -> Out {
        let ids: Vec<String> = ids.iter().map(|i| (*i).to_owned()).collect();
        plan(s, "acc", "chan", &ids, why, now)
    }

    fn posts(list: &[(&str, bool)]) -> Vec<(String, bool)> {
        list.iter().map(|(i, p)| ((*i).to_owned(), *p)).collect()
    }

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

    #[test]
    fn opened_is_passive_every_time() {
        let (mut s, t) = (State::default(), Instant::now());
        assert_eq!(go(&mut s, Why::Opened, &["1"], t).seventv, posts(&[("1", true)]));
        assert_eq!(
            go(&mut s, Why::Opened, &["1"], at(t, 1)).seventv,
            posts(&[("1", true)])
        );
    }

    #[test]
    fn sent_is_normal_and_waits_10s() {
        let (mut s, t) = (State::default(), Instant::now());
        assert_eq!(go(&mut s, Why::Sent, &["1"], t).seventv, posts(&[("1", false)]));
        assert!(go(&mut s, Why::Sent, &["1"], at(t, 9)).seventv.is_empty());
        assert_eq!(
            go(&mut s, Why::Sent, &["1"], at(t, 10)).seventv,
            posts(&[("1", false)])
        );
    }

    #[test]
    fn opened_does_not_hold_back_a_send() {
        let (mut s, t) = (State::default(), Instant::now());
        go(&mut s, Why::Opened, &["1"], t);
        assert_eq!(
            go(&mut s, Why::Sent, &["1"], at(t, 1)).seventv,
            posts(&[("1", false)])
        );
    }

    #[test]
    fn bttv_keeps_its_minute() {
        let (mut s, t) = (State::default(), Instant::now());
        let bttv: Vec<bool> = [0, 11, 59, 60]
            .iter()
            .map(|n| go(&mut s, Why::Sent, &["1"], at(t, *n)).bttv)
            .collect();
        assert_eq!(bttv, [true, false, false, true]);
    }

    #[test]
    fn restyled_skips_bttv_and_the_wait() {
        let (mut s, t) = (State::default(), Instant::now());
        go(&mut s, Why::Sent, &["1"], t);
        let out = go(&mut s, Why::Restyled, &["1"], at(t, 1));
        assert_eq!(
            out,
            Out {
                seventv: posts(&[("1", false)]),
                bttv: false
            }
        );
        assert!(go(&mut s, Why::Sent, &["1"], at(t, 2)).seventv.is_empty());
    }

    #[test]
    fn shared_chat_tells_each_channel_with_its_own_wait() {
        let (mut s, t) = (State::default(), Instant::now());
        assert_eq!(
            go(&mut s, Why::Sent, &["1", "2"], t).seventv,
            posts(&[("1", false), ("2", false)])
        );
        assert_eq!(
            go(&mut s, Why::Sent, &["1", "2", "3"], at(t, 5)).seventv,
            posts(&[("3", false)])
        );
        assert_eq!(
            go(&mut s, Why::Sent, &["1", "2", "3"], at(t, 10)).seventv,
            posts(&[("1", false), ("2", false)])
        );
    }

    #[test]
    fn shared_channels_only_on_send() {
        let (mut s, t) = (State::default(), Instant::now());
        assert_eq!(
            go(&mut s, Why::Opened, &["1", "2"], t).seventv,
            posts(&[("1", true)])
        );
        assert_eq!(
            go(&mut s, Why::Restyled, &["1", "2"], t).seventv,
            posts(&[("1", false)])
        );
    }
}
