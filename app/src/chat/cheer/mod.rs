//! V107-Bn Cheering from the chat box: the picker's rules and balance, and the one send
//! path for typed and picked cheers. Every send reads the balance and rules fresh and
//! checks again here. Same gate as power-ups (GQL sign-in). Spends real Bits.
//!
//! Mocks builds on a test data folder: `CYEX_GQL_CHEER_MOCK=http://127.0.0.1:<port>` sends
//! everything to the local mock (`tools/mock/cheer.mjs`) with a fake token.

mod attempts;
mod bits_on;

use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use cyex_twitch::gql::Auth;
use cyex_twitch::gql::cheer::{self, CheerReply, CheerRules, SendCheer, check, codes};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::powerups::{Ctx, ctx, http};
use attempts::{Attempts, Ended};
use bits_on::BitsOn;

const NO_GQL: &str = "Cheering needs the GQL sign-in (Settings → Accounts).";
const MOCK_TOKEN: &str = "mock-token";
#[cfg(feature = "mocks")]
const MOCK_VAR: &str = "CYEX_GQL_CHEER_MOCK";

static ATTEMPTS: LazyLock<Mutex<Attempts>> = LazyLock::new(Mutex::default);
static BITS_ON: LazyLock<Mutex<BitsOn>> = LazyLock::new(Mutex::default);

/// What the Bits picker needs.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CheerInfo {
    pub enabled: bool,
    pub cheer_min: u32,
    pub emote_min: u32,
    #[ts(type = "number | null")]
    pub balance: Option<u64>,
    /// Lowercase prefixes a cheer may use here.
    pub prefixes: Vec<String>,
}

/// What a send did: went out, or asks first (`again`: the same cheer went out moments ago).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum CheerSent {
    Done {
        bits: u32,
        #[ts(type = "number | null")]
        balance: Option<u64>,
    },
    Ask {
        bits: u32,
        again: bool,
    },
    /// Bits are off here: cheer words are plain chat, so send it as text.
    Text,
}

/// Who calls and where: Twitch with the account's token, or the mock with a fake one.
struct Target {
    token: String,
    client_id: String,
    base: Option<String>,
}

impl Target {
    fn auth(&self) -> Auth<'_> {
        crate::auth::gql_auth(&self.token, &self.client_id)
    }
}

fn target(app: &AppHandle, c: &Ctx) -> Result<Target, String> {
    if c.account.is_none() {
        return Err("Sign in to cheer.".into());
    }
    let base = mock_base(app)?;
    let (token, client_id) = match &base {
        Some(_) => (MOCK_TOKEN.to_owned(), cyex_twitch::clients::WEB.to_owned()),
        None => c.token.clone().ok_or(NO_GQL)?,
    };
    Ok(Target { token, client_id, base })
}

#[cfg(feature = "mocks")]
fn mock_base(app: &AppHandle) -> Result<Option<String>, String> {
    let Some(raw) = std::env::var(MOCK_VAR).ok().filter(|r| !r.trim().is_empty()) else {
        return Ok(None);
    };
    // Set but unusable: nothing is sent at all, never Twitch instead.
    if crate::paths::webview(app).is_none() {
        return Err("The cheer mock runs only on a test data folder.".into());
    }
    crate::mock_url::loopback_base(&raw)
        .map(Some)
        .ok_or_else(|| "The cheer mock must be http://127.0.0.1:<port>.".into())
}

#[cfg(not(feature = "mocks"))]
fn mock_base(_: &AppHandle) -> Result<Option<String>, String> {
    Ok(None)
}

/// `None`: the channel has no Bits settings. Every read also updates `BITS_ON`.
async fn rules(app: &AppHandle, login: &str, auth: Auth<'_>, base: Option<&str>) -> Result<Option<CheerRules>, String> {
    let read = cheer::rules(&http(app), auth, base, login).await;
    let mut b = bits_on();
    match &read {
        Ok(r) => b.set(login, r.as_ref().is_some_and(|r| r.enabled), Instant::now()),
        Err(_) => b.failed(login),
    }
    drop(b);
    read.map_err(|e| e.to_string())
}

/// Reads whether `login` takes cheers, signed out (or from the mock).
fn read_bits_on(app: &AppHandle, login: &str) {
    let (app, login) = (app.clone(), login.to_owned());
    tauri::async_runtime::spawn(async move {
        let Ok(base) = mock_base(&app) else {
            return bits_on().failed(&login);
        };
        let auth = match base {
            Some(_) => crate::auth::gql_auth(MOCK_TOKEN, cyex_twitch::clients::WEB),
            None => Auth::PUBLIC,
        };
        let _ = rules(&app, &login, auth, base.as_deref()).await;
    });
}

pub async fn info(app: &AppHandle, view: u32) -> Result<CheerInfo, String> {
    let c = ctx(app, view)?;
    let t = target(app, &c)?;
    let r = rules(app, &c.login, t.auth(), t.base.as_deref())
        .await?
        .ok_or("This channel can’t take cheers.")?;
    Ok(CheerInfo {
        enabled: r.enabled,
        cheer_min: r.cheer_min,
        emote_min: r.emote_min,
        balance: r.balance,
        prefixes: r.usable(),
    })
}

/// Sends `text` as a cheer of `bits` (what the box showed). From the confirm threshold up,
/// for the same cheer again within moments and when `careful` (pasted or recalled), only
/// `confirmed`: else it answers `Ask`. With Bits off it answers `Text`.
pub async fn send(
    app: &AppHandle,
    view: u32,
    text: &str,
    bits: u32,
    confirmed: bool,
    careful: bool,
) -> Result<CheerSent, String> {
    let text = text.trim();
    let c = ctx(app, view)?;
    let t = target(app, &c)?;
    let rules = rules(app, &c.login, t.auth(), t.base.as_deref()).await?;
    let Some(r) = rules.filter(|r| r.enabled) else {
        return Ok(CheerSent::Text);
    };
    let cheer = check::check(text, &r).map_err(|e| e.text())?;
    if cheer.bits != bits {
        return Err("The amount changed. Check it and try again.".into());
    }
    let key = (
        c.account.clone().unwrap_or_default(),
        r.channel_id.clone(),
        cheer.bits,
        text.to_owned(),
    );
    let ask_from = app
        .state::<crate::settings::Settings>()
        .read::<ChatSettings>(Partition::Chat)
        .cheer_confirm_bits;
    let again = lock().again(&key, Instant::now());
    if !confirmed && (careful || again || cheer.bits >= ask_from) {
        return Ok(CheerSent::Ask { bits: cheer.bits, again });
    }
    let id = lock()
        .begin(key.clone(), Instant::now(), cheer::new_id)
        .ok_or("That cheer is on its way.")?;
    let sent = cheer::send(
        &http(app),
        t.auth(),
        t.base.as_deref(),
        &SendCheer {
            id: &id,
            channel_id: &r.channel_id,
            bits: cheer.bits,
            content: text,
        },
    )
    .await;
    // No answer at all: maybe spent, so it reads like `UNKNOWN`.
    let reply = sent.unwrap_or(CheerReply {
        balance: None,
        refused: Some(codes::UNKNOWN.into()),
    });
    lock().end(&key, Instant::now(), ended(&reply));
    match reply.refused {
        None => Ok(CheerSent::Done {
            bits: cheer.bits,
            balance: reply.balance,
        }),
        Some(code) => Err(codes::text(&code).into()),
    }
}

fn ended(reply: &CheerReply) -> Ended {
    match reply.refused.as_deref() {
        None => Ended::Sent,
        Some(code) if codes::unsure(code) => Ended::Unsure,
        Some(_) => Ended::Refused,
    }
}

fn lock() -> std::sync::MutexGuard<'static, Attempts> {
    ATTEMPTS.lock().unwrap_or_else(|p| p.into_inner())
}

fn bits_on() -> std::sync::MutexGuard<'static, BitsOn> {
    BITS_ON.lock().unwrap_or_else(|p| p.into_inner())
}

const CHECKING: &str = "One moment, checking Bits for this channel.";
const NOT_TEXT: &str = "A cheer word can’t go out as plain text.";

/// Whether a plain message may go out. Cheers never go out as text: only the chat box's
/// cheer path sends them. `send_to` has turned commands away already, so whatever reaches
/// here goes out as text (".gg Cheer100" too). With Bits off, cheer words are just text;
/// before that's known it waits for a read instead of guessing. Before the global
/// cheermotes load, any cheer-shaped word counts.
pub fn plain_ok(app: &AppHandle, login: &str, text: &str) -> Result<(), String> {
    let channel = super::catalog::cached(app, login);
    let global = app.try_state::<super::catalog::Catalogs>().and_then(|c| c.global_cached());
    let known = |p: &str| {
        let hit = |list: &[cyex_twitch::gql::Cheermote]| list.iter().any(|c| c.prefix.eq_ignore_ascii_case(p));
        global.as_ref().is_none_or(|g| hit(&g.cheermotes)) || channel.as_ref().is_some_and(|c| hit(&c.cheermotes))
    };
    if !check::words(text, known).iter().any(|w| w.amount > 0) {
        return Ok(());
    }
    let (on, read) = bits_on().lookup(login, Instant::now());
    if read {
        read_bits_on(app, login);
    }
    plain_verdict(on).map_err(str::to_owned)
}

/// A plain message with a cheer word, by whether the channel takes cheers.
fn plain_verdict(bits_on: Option<bool>) -> Result<(), &'static str> {
    match bits_on {
        Some(false) => Ok(()),
        Some(true) => Err(NOT_TEXT),
        None => Err(CHECKING),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(code: Option<&str>) -> CheerReply {
        CheerReply {
            balance: None,
            refused: code.map(str::to_owned),
        }
    }

    #[test]
    fn v107_bn_only_a_clear_no_drops_the_id() {
        assert_eq!(ended(&reply(None)), Ended::Sent);
        assert_eq!(ended(&reply(Some("INSUFFICIENT_BALANCE"))), Ended::Refused);
        assert_eq!(ended(&reply(Some(codes::UNKNOWN))), Ended::Unsure);
        assert_eq!(ended(&reply(Some("REQUEST_THROTTLED"))), Ended::Unsure);
    }

    #[test]
    fn v107_bn_plain_cheer_words_wait_for_bits_then_follow_it() {
        assert_eq!(plain_verdict(Some(false)), Ok(()), "Bits off: just text");
        assert_eq!(plain_verdict(Some(true)), Err(NOT_TEXT));
        assert_eq!(plain_verdict(None), Err(CHECKING));
    }
}
