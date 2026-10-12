//! Runs one mod command (1.49, 1.50) for the acting account: Helix when it has the
//! scopes, else the raw GQL mutation (D25). Bans, timeouts, deletes and role changes stay
//! quiet since EventSub and IRC echo them; the rest get a short local confirmation.

use cyex_chat::model::ModRole;
use cyex_moderation::command::{ModCommand, Treatment};
use cyex_twitch::gql;
use cyex_twitch::helix::modes::ModeChange;
use cyex_twitch::helix::moderation as hm;
use cyex_twitch::helix::roles::{self as hr, Role as HRole};
use tauri::AppHandle;

use super::{Actor, codes, http};

/// Looks up a login's user id (for actions that need an id).
async fn user_id(app: &AppHandle, login: &str) -> Result<String, String> {
    gql::profile(&http(app), Some(login), None)
        .await
        .map_err(|e| e.to_string())?
        .map(|p| p.id)
        .ok_or_else(|| format!("No user called {login}."))
}

/// Twitch's GQL `expiresIn` form for a timeout: seconds as `"<n>s"`.
fn expires_in(secs: u32) -> String {
    format!("{secs}s")
}

/// Runs a parsed command. `Ok(Some(line))` shows a local confirmation; `Ok(None)` stays
/// quiet (the action echoes over EventSub/IRC).
pub async fn command(app: &AppHandle, account: &str, login: &str, cmd: ModCommand) -> Result<Option<String>, String> {
    let actor = super::actor_for(app, account, login, cmd.needs())?;
    use ModCommand as C;
    match cmd {
        C::Ban { login: who, reason } => ban(app, &actor, &who, None, &reason).await.map(|()| None),
        C::Timeout { login: who, secs, reason } => ban(app, &actor, &who, Some(secs), &reason).await.map(|()| None),
        C::Unban { login: who } => unban(app, &actor, &who).await.map(|()| None),
        C::Delete { msg_id } => delete(app, &actor, Some(&msg_id)).await.map(|()| None),
        C::Clear => delete(app, &actor, None).await.map(|()| Some("Chat cleared.".into())),
        C::Warn { login: who, reason } => warn(app, &actor, &who, &reason).await.map(|()| None),
        C::Mode(change) => mode(app, &actor, change).await.map(Some),
        C::Treat { login: who, how } => treat(app, &actor, &who, how).await.map(|()| None),
        C::SetRole { login: who, role: r, on } => set_role(app, &actor, &who, r == HRole::Mod, on).await.map(|()| None),
        C::ListRole(r) => list_role(app, &actor, r == HRole::Mod).await.map(Some),
        C::Announce { color, text } => {
            let (auth, room) = helix_for(&actor, "announce")?;
            let http = http(app);
            cyex_twitch::helix::modes::announce(&http, auth, room, &text, color).await.map_err(helix_err)?;
            Ok(None)
        }
        C::Shoutout { login: who } => {
            let to = user_id(app, &who).await?;
            let (auth, room) = helix_for(&actor, "shout out")?;
            let http = http(app);
            cyex_twitch::helix::modes::shoutout(&http, auth, room, &to).await.map_err(helix_err)?;
            Ok(Some(format!("Shouted out {who}.")))
        }
        C::Shield(on) => {
            let (auth, room) = helix_for(&actor, "use shield mode")?;
            let http = http(app);
            hm::set_shield(&http, auth, room, on).await.map_err(helix_err)?;
            Ok(Some(if on { "Shield mode on.".into() } else { "Shield mode off.".into() }))
        }
        C::Pin { msg_id, secs } => pin(app, &actor, &msg_id, secs).await.map(|()| Some("Pinned.".into())),
        C::PinSend { text, secs } => pin_send(app, &actor, &text, secs.unwrap_or(PIN_SEND_SECS)).await.map(|()| None),
        C::Unpin => unpin(app, &actor).await.map(|()| Some("Unpinned.".into())),
        C::Raid { login: who } => {
            let to = user_id(app, &who).await?;
            let (auth, _) = helix_for(&actor, "raid")?;
            let http = http(app);
            hr::raid(&http, auth, &actor.channel_id, &to).await.map_err(helix_err)?;
            Ok(Some(format!("Raiding {who}…")))
        }
        C::Unraid => {
            let (auth, _) = helix_for(&actor, "cancel a raid")?;
            let http = http(app);
            hr::unraid(&http, auth, &actor.channel_id).await.map_err(helix_err)?;
            Ok(Some("Raid canceled.".into()))
        }
        C::Commercial { secs } => {
            let (auth, _) = helix_for(&actor, "run an ad")?;
            let http = http(app);
            let got = hr::commercial(&http, auth, &actor.channel_id, secs).await.map_err(helix_err)?;
            Ok(Some(format!("Ad started ({got}s).")))
        }
        C::Marker { text } => {
            let (auth, _) = helix_for(&actor, "add a marker")?;
            let http = http(app);
            hr::marker(&http, auth, &actor.channel_id, &text).await.map_err(helix_err)?;
            Ok(Some("Marker added.".into()))
        }
    }
}

/// A Helix mod auth and room, or a clear message when the account has no Helix sign-in.
fn helix_for<'a>(actor: &'a Actor, what: &str) -> Result<(cyex_twitch::helix::Auth<'a>, cyex_twitch::helix::Room<'a>), String> {
    actor
        .helix_any()
        .ok_or_else(|| format!("You need a Helix sign-in with mod scopes to {what}."))
}

fn helix_err(e: cyex_twitch::Error) -> String {
    codes::helix(status(&e), e.api_message().unwrap_or(""))
}

async fn ban(app: &AppHandle, actor: &Actor, who: &str, secs: Option<u32>, reason: &str) -> Result<(), String> {
    if let Some(h) = actor.helix_scoped(app, "moderator:manage:banned_users") {
        let (res, _) = hm::ban(&http(app), h, actor.room(), &target_id(app, who).await?, secs, reason).await;
        return res.map_err(|e| codes::helix(status(&e), e.api_message().unwrap_or("")));
    }
    if let Some(g) = actor.gql() {
        let expires = secs.map(expires_in);
        return refuse(gql::moderation::ban(&http(app), g, &actor.channel_id, who, expires.as_deref(), reason).await, who);
    }
    Err(no_token())
}

async fn unban(app: &AppHandle, actor: &Actor, who: &str) -> Result<(), String> {
    if let Some(h) = actor.helix_scoped(app, "moderator:manage:banned_users") {
        let (res, _) = hm::unban(&http(app), h, actor.room(), &target_id(app, who).await?).await;
        return res.map_err(|e| codes::helix(status(&e), e.api_message().unwrap_or("")));
    }
    if let Some(g) = actor.gql() {
        return refuse(gql::moderation::unban(&http(app), g, &actor.channel_id, who).await, who);
    }
    Err(no_token())
}

async fn delete(app: &AppHandle, actor: &Actor, msg_id: Option<&str>) -> Result<(), String> {
    if let Some(h) = actor.helix_scoped(app, "moderator:manage:chat_messages") {
        return hm::delete(&http(app), h, actor.room(), msg_id)
            .await
            .map_err(|e| codes::helix(status(&e), e.api_message().unwrap_or("")));
    }
    match (actor.gql(), msg_id) {
        (Some(g), Some(id)) => refuse(gql::moderation::delete_message(&http(app), g, &actor.channel_id, id).await, "that message"),
        (Some(_), None) => Err("Clearing the whole chat needs a Helix sign-in with mod scopes.".into()),
        _ => Err(no_token()),
    }
}

async fn warn(app: &AppHandle, actor: &Actor, who: &str, reason: &str) -> Result<(), String> {
    if let Some(h) = actor.helix_scoped(app, "moderator:manage:warnings") {
        return hm::warn(&http(app), h, actor.room(), &target_id(app, who).await?, reason)
            .await
            .map_err(|e| codes::helix(status(&e), e.api_message().unwrap_or("")));
    }
    if let Some(g) = actor.gql() {
        return refuse(gql::moderation::warn(&http(app), g, &actor.channel_id, who, reason, &[]).await, who);
    }
    Err(no_token())
}

/// V101-34 Helix pins when the account has the scope, GQL otherwise (both GA since 2026/05).
async fn pin(app: &AppHandle, actor: &Actor, msg_id: &str, secs: Option<u32>) -> Result<(), String> {
    if let Some(h) = actor.helix_scoped(app, "moderator:manage:chat_messages") {
        return hm::pin(&http(app), h, actor.room(), msg_id, secs).await.map_err(helix_err);
    }
    if let Some(g) = actor.gql() {
        return refuse(gql::moderation::pin(&http(app), g, &actor.channel_id, msg_id, secs).await, "that message");
    }
    Err(no_token())
}

/// M11 Twitch's web chat pins for 20 minutes.
const PIN_SEND_SECS: u32 = 1200;

/// M11 Sends a line and pins it: Twitch's own send-and-pin over GQL, else Helix sends it
/// and pins it by the id it gets back. The line itself comes in over IRC like any other.
async fn pin_send(app: &AppHandle, actor: &Actor, text: &str, secs: u32) -> Result<(), String> {
    if text.chars().count() > cyex_chat::irc::write::MAX_MESSAGE_CHARS {
        return Err("That's over 500 characters.".into());
    }
    // F20 Helix cuts an unknown command word off and sends the rest.
    let text = &crate::chat::typed::escape(text.to_owned());
    let http = http(app);
    if let Some(g) = actor.gql() {
        let pin_id = match gql::moderation::send_pinned(&http, g, &actor.channel_id, text).await {
            Ok(id) => id,
            Err(cyex_twitch::Error::Refused(c)) => return Err(codes::gql(&c, "that message")),
            Err(e) => return Err(e.to_string()),
        };
        return refuse(gql::moderation::update_pin(&http, g, &pin_id, secs).await, "that pin");
    }
    let pins = actor.helix_scoped(app, "moderator:manage:chat_messages").is_some();
    if let Some(h) = actor.helix_scoped(app, "user:write:chat").filter(|_| pins) {
        let room = actor.room();
        let id = match cyex_twitch::helix::send_message(&http, h, room.broadcaster_id, room.moderator_id, text).await {
            Ok(id) => id,
            Err(cyex_twitch::Error::Refused(why)) => return Err(why),
            Err(e) => return Err(helix_err(e)),
        };
        return hm::pin(&http, h, room, &id, Some(secs)).await.map_err(helix_err);
    }
    Err(no_token())
}

/// V101-34 Takes down the channel's current pin. Helix wants the message's id, GQL the pin's.
async fn unpin(app: &AppHandle, actor: &Actor) -> Result<(), String> {
    let http = http(app);
    let pin = gql::pinned(&http, &actor.channel_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Nothing is pinned.")?;
    if let Some(h) = actor.helix_scoped(app, "moderator:manage:chat_messages") {
        return hm::unpin(&http, h, actor.room(), &pin.message_id).await.map_err(helix_err);
    }
    if let Some(g) = actor.gql() {
        return refuse(gql::moderation::unpin(&http, g, &pin.id).await, "that pin");
    }
    Err(no_token())
}

async fn mode(app: &AppHandle, actor: &Actor, change: ModeChange) -> Result<String, String> {
    let name = mode_name(change);
    let (auth, room) = helix_for(actor, "change chat modes")?;
    let http = http(app);
    cyex_twitch::helix::modes::set_mode(&http, auth, room, change).await.map_err(helix_err)?;
    Ok(name)
}

async fn treat(app: &AppHandle, actor: &Actor, who: &str, how: Treatment) -> Result<(), String> {
    let target = target_id(app, who).await?;
    let status = match how {
        Treatment::Monitor => Some("ACTIVE_MONITORING"),
        Treatment::Restrict => Some("RESTRICTED"),
        Treatment::None => None,
    };
    if let Some(h) = actor.helix_scoped(app, "moderator:manage:suspicious_users") {
        return hm::treat(&http(app), h, actor.room(), &target, status)
            .await
            .map_err(|e| codes::helix(status_code(&e), e.api_message().unwrap_or("")));
    }
    if let Some(g) = actor.gql() {
        return gql::moderation::treat_low_trust(&http(app), g, &actor.channel_id, &target, status.unwrap_or("NONE"))
            .await
            .map_err(|e| e.to_string());
    }
    Err(no_token())
}

async fn set_role(app: &AppHandle, actor: &Actor, who: &str, mod_role: bool, on: bool) -> Result<(), String> {
    // Only the broadcaster may change roles over Helix; lead mods go through GQL.
    let scope = if mod_role { "channel:manage:moderators" } else { "channel:manage:vips" };
    if actor.role == ModRole::Broadcaster
        && let Some(h) = actor.helix_scoped(app, scope)
    {
        let hrole = if mod_role { HRole::Mod } else { HRole::Vip };
        return hr::set_role(&http(app), h, &actor.channel_id, &target_id(app, who).await?, hrole, on)
            .await
            .map_err(|e| codes::helix(status(&e), e.api_message().unwrap_or("")));
    }
    if let Some(g) = actor.gql() {
        return refuse(gql::moderation::set_role(&http(app), g, &actor.channel_id, who, mod_role, on).await, who);
    }
    Err(no_token())
}

async fn list_role(app: &AppHandle, actor: &Actor, mods: bool) -> Result<String, String> {
    let h = actor
        .helix_scoped(app, if mods { "moderation:read" } else { "channel:read:vips" })
        .ok_or("Listing needs a Helix sign-in.")?;
    let list = if mods {
        hr::moderators(&http(app), h, &actor.channel_id).await
    } else {
        hr::vips(&http(app), h, &actor.channel_id).await
    }
    .map_err(|e| codes::helix(status(&e), e.api_message().unwrap_or("")))?;
    let names: Vec<&str> = list.iter().map(|u| u.user_name.as_str()).collect();
    let what = if mods { "Mods" } else { "VIPs" };
    Ok(if names.is_empty() {
        format!("No {}.", what.to_lowercase())
    } else {
        format!("{what}: {}", names.join(", "))
    })
}

/// Resolves the target's user id; bans over Helix need the id, not the login.
async fn target_id(app: &AppHandle, who: &str) -> Result<String, String> {
    user_id(app, who).await
}

/// Maps a GQL mutation's `Ok(Some(code))` into a friendly error.
fn refuse(res: cyex_twitch::Result<Option<String>>, who: &str) -> Result<(), String> {
    match res {
        Ok(None) => Ok(()),
        Ok(Some(code)) => Err(codes::gql(&code, who)),
        Err(e) => Err(e.to_string()),
    }
}

fn no_token() -> String {
    "This account can't moderate. Sign in again in Settings → Accounts.".into()
}

fn status(e: &cyex_twitch::Error) -> u16 {
    status_code(e)
}

fn status_code(e: &cyex_twitch::Error) -> u16 {
    match e {
        cyex_twitch::Error::Api { status, .. } => *status,
        _ => 0,
    }
}

fn mode_name(change: ModeChange) -> String {
    match change {
        ModeChange::Emote(true) => "Emote-only on.",
        ModeChange::Emote(false) => "Emote-only off.",
        ModeChange::Followers(Some(_)) => "Followers-only on.",
        ModeChange::Followers(None) => "Followers-only off.",
        ModeChange::Slow(Some(_)) => "Slow mode on.",
        ModeChange::Slow(None) => "Slow mode off.",
        ModeChange::Subs(true) => "Subscribers-only on.",
        ModeChange::Subs(false) => "Subscribers-only off.",
        ModeChange::Unique(true) => "Unique-chat on.",
        ModeChange::Unique(false) => "Unique-chat off.",
        ModeChange::Delay(_) => "Chat delay changed.",
    }
    .to_owned()
}
