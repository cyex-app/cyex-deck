//! Sending whispers from the default account: GQL when signed in that way (Twitch answers
//! with the stored message), else Helix. If Twitch turns the GQL token away and Helix can
//! send, Helix takes over. `/w` and `/r` in chat come here too.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use cyex_chat::irc::Window;
use cyex_twitch::gql::{self, whispers as api};
use tokio::time::Instant;
use cyex_twitch::helix;
use tauri::{AppHandle, Manager};

use super::{GqlCall, NO_ACCOUNT, current, lock};
use crate::auth;

/// Twitch's own cap for a whisper to someone new; longer ones get cut.
pub const MAX: usize = 500;
const SEND_SCOPE: &str = "user:manage:whispers";

/// The person a login names: from a known thread, else looked up.
pub(super) async fn resolve(app: &AppHandle, login: &str) -> Result<api::Person, String> {
    let login = login.trim().trim_start_matches('@').to_lowercase();
    if login.is_empty() {
        return Err("Who to? Type a name.".into());
    }
    if let Some(t) = lock(app).threads.iter().find(|t| t.view.other.login == login) {
        let o = &t.view.other;
        return Ok(api::Person {
            id: o.id.clone(),
            login: o.login.clone(),
            display: o.name.clone(),
            color: String::new(),
            avatar: o.avatar.clone(),
        });
    }
    let http = app.state::<auth::Auth>().http.clone();
    let found = gql::lookup(&http, &login).await.map_err(|e| {
        if e.is_network() {
            "Couldn’t reach Twitch.".to_owned()
        } else {
            format!("Couldn’t look up {login}.")
        }
    })?;
    let c = found
        .into_iter()
        .find(|c| c.login == login)
        .ok_or_else(|| format!("No one is called {login}."))?;
    Ok(api::Person {
        id: c.id,
        login: c.login,
        display: c.name,
        color: String::new(),
        avatar: c.avatar,
    })
}

/// Sends `text` to `to` (a login). The line reaches the view through `whispers:line`.
pub async fn send(app: &AppHandle, to: &str, text: &str) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Type something first.".into());
    }
    if text.chars().count() > MAX {
        return Err(format!("Whispers can be {MAX} characters at most."));
    }
    let me = current(app)?;
    let other = resolve(app, to).await?;
    if other.id == me.id {
        return Err("You can’t whisper yourself.".into());
    }
    pace(&me.id).await?;
    let thread_id = api::thread_id(&me.id, &other.id);
    if me.gql {
        let c = GqlCall::new(app, &me.id).map_err(|_| NO_ACCOUNT.to_owned())?;
        let nonce = cyex_twitch::random::hex(16);
        match api::send(&c.http, c.auth(), &other.id, text, &nonce).await {
            Ok(Some(m)) => {
                super::live::incoming(app, &thread_id, m, Some(other));
                return Ok(());
            }
            Ok(None) => {
                echo(app, &me, &thread_id, other, text);
                return Ok(());
            }
            Err(e) if refused(&e) && auth::helix_scoped(app, &me.id, SEND_SCOPE).is_some() => {}
            Err(e) => return Err(reason(e.api_message().unwrap_or_default(), e.is_network())),
        }
    }
    let Some((token, client_id)) = auth::helix_scoped(app, &me.id, SEND_SCOPE) else {
        return Err("Sending whispers needs a sign-in with whisper access (Settings → Accounts).".into());
    };
    let http = app.state::<auth::Auth>().http.clone();
    let a = crate::auth::helix_auth(&token, &client_id);
    helix::whispers::send(&http, a, &me.id, &other.id, text)
        .await
        .map_err(|e| helix_reason(&e))?;
    echo(app, &me, &thread_id, other, text);
    Ok(())
}

/// V107-N10d Twitch allows 3 whispers a second and 100 a minute per account; we stay under.
const SHORT: (usize, Duration) = (3, Duration::from_millis(1_200));
const LONG: (usize, Duration) = (90, Duration::from_secs(61));
const TOO_MANY: &str = "That’s a lot of whispers. Wait a minute.";

/// Both windows of one account.
struct Pace {
    short: Window,
    long: Window,
}

impl Pace {
    fn new() -> Self {
        Self { short: Window::new(SHORT.1), long: Window::new(LONG.1) }
    }

    /// `Ok(None)`: a slot was taken. `Ok(Some(d))`: wait `d` and ask again. `Err`: the minute is used up.
    fn take(&mut self, now: Instant) -> Result<Option<Duration>, &'static str> {
        if self.long.wait(LONG.0, now).is_some() {
            return Err(TOO_MANY);
        }
        if let Some(d) = self.short.wait(SHORT.0, now) {
            return Ok(Some(d));
        }
        self.short.try_take(SHORT.0, now);
        self.long.try_take(LONG.0, now);
        Ok(None)
    }
}

/// V107-N10d Holds a whisper back for a moment when a few go at once (`/foreach … > /w`),
/// and refuses it when the account's minute is used up.
async fn pace(account: &str) -> Result<(), String> {
    static PACES: LazyLock<Mutex<HashMap<String, Pace>>> = LazyLock::new(Default::default);
    loop {
        let wait = {
            let mut all = PACES.lock().unwrap_or_else(|e| e.into_inner());
            all.entry(account.to_owned()).or_insert_with(Pace::new).take(Instant::now())?
        };
        match wait {
            Some(d) => tokio::time::sleep(d).await,
            None => return Ok(()),
        }
    }
}

/// Who `/r` answers: the last person who whispered.
pub fn reply_target(app: &AppHandle) -> Option<String> {
    lock(app).last_from.clone()
}

/// Our own line when Twitch didn't send one back.
fn echo(app: &AppHandle, me: &super::Me, thread_id: &str, other: api::Person, text: &str) {
    let m = api::Message {
        id: cyex_chat::build::local_id(),
        nonce: None,
        sent_at: cyex_twitch::clock::rfc3339(auth::unix_now()),
        from: api::Person {
            id: me.id.clone(),
            login: me.login.clone(),
            display: me.display.clone(),
            color: String::new(),
            avatar: None,
        },
        text: text.to_owned(),
        emotes: Vec::new(),
        edited: false,
        deleted: false,
    };
    super::live::incoming(app, thread_id, m, Some(other));
}

/// Twitch turned the token itself away (integrity, sign-in), not the whisper.
fn refused(e: &cyex_twitch::Error) -> bool {
    e.api_message().is_some_and(|m| {
        m.contains("integrity") || m.contains("unauthenticated") || m.contains("Unauthorized")
    })
}

fn reason(code: &str, network: bool) -> String {
    if network {
        return "Couldn’t reach Twitch.".into();
    }
    match code {
        "TARGET_RESTRICTED" => "They don’t take whispers from you.".into(),
        "TARGET_BANNED" => "That account is suspended.".into(),
        "BODY_EMPTY" => "Type something first.".into(),
        "WHISPERS_OFF" => "Whispers are turned off on this account.".into(),
        "SENDER_NOT_VERIFIED" => "Twitch wants a verified phone number before you can whisper.".into(),
        "SENDER_HAS_CONSEQUENCE" => "Twitch has blocked your whispers for now.".into(),
        _ => {
            eprintln!("whispers: send: {code}");
            "Twitch didn’t deliver it.".into()
        }
    }
}

fn helix_reason(e: &cyex_twitch::Error) -> String {
    if e.is_network() {
        return "Couldn’t reach Twitch.".into();
    }
    let msg = e.api_message().unwrap_or_default().to_lowercase();
    match e.status() {
        Some(401) if msg.contains("verified phone") => {
            "Twitch wants a verified phone number before you can whisper.".into()
        }
        Some(400) | Some(403) if msg.contains("settings prevent") => {
            "They don’t take whispers from you.".into()
        }
        Some(403) => "Twitch has blocked your whispers for now.".into(),
        Some(404) => "No one by that name.".into(),
        Some(429) => "That’s a lot of whispers. Wait a bit, or whisper fewer new people today.".into(),
        _ => {
            eprintln!("whispers: helix send: {e}");
            "Twitch didn’t deliver it.".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whispers_are_paced_per_second_and_capped_per_minute() {
        // V107-N10d
        let t0 = Instant::now();
        let mut p = Pace::new();
        for _ in 0..3 {
            assert_eq!(p.take(t0), Ok(None));
        }
        assert_eq!(p.take(t0), Ok(Some(SHORT.1)));
        let mut t = t0;
        for _ in 1..LONG.0 / SHORT.0 {
            t += SHORT.1;
            for _ in 0..SHORT.0 {
                assert_eq!(p.take(t), Ok(None));
            }
        }
        assert_eq!(p.take(t + SHORT.1), Err(TOO_MANY));
        assert_eq!(p.take(t0 + Duration::from_secs(120)), Ok(None));
    }

    #[test]
    fn codes_read_as_text() {
        assert_eq!(
            reason("TARGET_RESTRICTED", false),
            "They don’t take whispers from you."
        );
        assert_eq!(reason("TARGET_RESTRICTED", true), "Couldn’t reach Twitch.");
        assert_eq!(reason("SOMETHING_NEW", false), "Twitch didn’t deliver it.");
    }
}
