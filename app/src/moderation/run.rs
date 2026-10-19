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

/// V107-P4 The local line after a delete, like "Pinned." and "Unpinned.". V107-P7 With the
/// message's text when it's known: `Deleted message from cyexbot: “hi there”`.
fn deleted_line(who: Option<&str>, text: Option<&str>) -> String {
    match (who, text.and_then(cyex_chat::model::quote)) {
        (Some(n), Some(q)) => format!("Deleted message from {n}: “{q}”"),
        (Some(n), None) => format!("Deleted a message from {n}."),
        (None, _) => "Deleted a message.".into(),
    }
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
        C::Delete { msg_id } => {
            let (who, text) = crate::chat::live::deleted_of(app, &msg_id);
            // V107-P7 Marked first: the echo can beat Twitch's answer.
            super::feeds::own_delete(app, &msg_id, true);
            if let Err(e) = delete(app, &actor, Some(&msg_id)).await {
                super::feeds::own_delete(app, &msg_id, false);
                return Err(e);
            }
            // V107 Twitch takes a deleted message's pin down with it (seen 2026/10/19).
            crate::chat::live::message_gone(app, &actor.channel_id, &msg_id);
            // V107-P4 Twitch's CLEARMSG only strikes the line through, so this is the one notice.
            Ok(Some(deleted_line(who.as_deref(), text.as_deref())))
        }
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
        C::PinSend { text, secs } => {
            pin_send(app, &actor, &text, secs.unwrap_or(PIN_SEND_SECS)).await?;
            crate::chat::live::pin_changed(app, &actor.channel_id);
            Ok(None)
        }
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

/// V107-M7 The last ban or timeout counts (`restack`).
async fn ban(app: &AppHandle, actor: &Actor, who: &str, secs: Option<u32>, reason: &str) -> Result<(), String> {
    super::restack::ban(app, actor, who, None, secs, reason).await
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
/// V107 A new pin replaces the old one, and every open chat of the channel shows it at once.
async fn pin(app: &AppHandle, actor: &Actor, msg_id: &str, secs: Option<u32>) -> Result<(), String> {
    let res = pin_once(app, actor, msg_id, secs).await;
    if res.is_ok() {
        crate::chat::live::pin_changed(app, &actor.channel_id);
    }
    res
}

async fn pin_once(app: &AppHandle, actor: &Actor, msg_id: &str, secs: Option<u32>) -> Result<(), String> {
    if let Some(h) = actor.helix_scoped(app, "moderator:manage:chat_messages") {
        return helix_pin(&http(app), h, actor, msg_id, secs).await;
    }
    if let Some(g) = actor.gql() {
        return gql_pin(&http(app), g, actor, msg_id, secs).await;
    }
    Err(no_token())
}

/// V107 GQL replaces another message's pin, but pinning the message that is pinned already
/// (a new length, say) fails with "service error" (seen 2026/10/19). Then: take that pin
/// down and pin again. Any other failure stands, so the old pin is never lost to it.
async fn gql_pin(http: &cyex_twitch::Http, g: gql::Auth<'_>, actor: &Actor, msg_id: &str, secs: Option<u32>) -> Result<(), String> {
    let first = gql::moderation::pin(http, g, &actor.channel_id, msg_id, secs).await;
    if matches!(first, Ok(None)) {
        return Ok(());
    }
    let held = gql::pinned(http, &actor.channel_id).await.ok().flatten();
    let Some(pin_id) = repin_over(held.as_ref().map(|p| (p.id.as_str(), p.message_id.as_str())), msg_id) else {
        return refuse(first, "that message");
    };
    refuse(gql::moderation::unpin(http, g, pin_id).await, "that pin")?;
    refuse(gql::moderation::pin(http, g, &actor.channel_id, msg_id, secs).await, "that message")
}

/// V107 After a failed pin: the pin to take down first, when `held` (pin id, message id)
/// is the very message being pinned again.
fn repin_over<'a>(held: Option<(&'a str, &str)>, msg_id: &str) -> Option<&'a str> {
    held.filter(|(pin, msg)| !pin.is_empty() && *msg == msg_id).map(|(pin, _)| pin)
}

/// M11 Twitch's web chat pins for 20 minutes.
const PIN_SEND_SECS: u32 = 1200;

/// M11 Sends a line and pins it: Twitch's own send-and-pin over GQL, else Helix sends it
/// and pins it by the id it gets back. The line itself comes in over IRC like any other.
async fn pin_send(app: &AppHandle, actor: &Actor, text: &str, secs: u32) -> Result<(), String> {
    if text.chars().count() > cyex_chat::irc::write::MAX_MESSAGE_CHARS {
        return Err("That’s over 500 characters.".into());
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
        return helix_pin(&http, h, actor, &id, Some(secs)).await;
    }
    Err(no_token())
}

/// V101-34 Takes down the channel's current pin. Helix wants the message's id, GQL the pin's.
async fn unpin(app: &AppHandle, actor: &Actor) -> Result<(), String> {
    let res = unpin_once(app, actor).await;
    if res.is_ok() {
        crate::chat::live::pin_changed(app, &actor.channel_id);
    }
    res
}

/// V107 Helix says 409 when the message is pinned already (Twitch's docs, 2026/10/19; a new
/// length for the same pin, say): take the pin down, then pin again.
async fn helix_pin(http: &cyex_twitch::Http, h: cyex_twitch::helix::Auth<'_>, actor: &Actor, msg_id: &str, secs: Option<u32>) -> Result<(), String> {
    match hm::pin(http, h, actor.room(), msg_id, secs).await {
        Err(e) if pin_taken(&e) => {
            if let Some(old) = gql::pinned(http, &actor.channel_id).await.map_err(|e| e.to_string())? {
                hm::unpin(http, h, actor.room(), &old.message_id).await.map_err(helix_err)?;
            }
            hm::pin(http, h, actor.room(), msg_id, secs).await.map_err(helix_err)
        }
        res => res.map_err(helix_err),
    }
}

/// V107 Helix's "already pinned".
fn pin_taken(e: &cyex_twitch::Error) -> bool {
    status_code(e) == 409
}

async fn unpin_once(app: &AppHandle, actor: &Actor) -> Result<(), String> {
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

/// 1.50 One mod or VIP change. V107-M8 Granting one to someone who holds the other takes
/// that away first (Twitch refuses to stack them) and leaves one chat line for both; a
/// grant Twitch refuses for that reason (stale badges) swaps too.
async fn set_role(app: &AppHandle, actor: &Actor, who: &str, mod_role: bool, on: bool) -> Result<(), String> {
    use super::standing;
    let clash = on && standing::holds(app, &actor.channel, who).await.is_some_and(|h| h.clash(mod_role));
    if !clash {
        match role_step(app, actor, who, mod_role, on).await {
            Ok(()) => {
                standing::changed(app, &actor.channel, who, mod_role, on);
                return Ok(());
            }
            Err(r) if !r.clash => return Err(r.text),
            Err(_) => {}
        }
    }
    swap_role(app, actor, who, mod_role).await
}

/// V107-M8 Takes the other role away, then grants `mod_role`.
async fn swap_role(app: &AppHandle, actor: &Actor, who: &str, mod_role: bool) -> Result<(), String> {
    use super::{feeds, standing};
    use cyex_moderation::role_change as rc;
    let (gone, got) = (rc::action(!mod_role, false), rc::action(mod_role, true));
    let mark = |m: bool| {
        feeds::own_role(app, &actor.channel_id, gone, who, m);
        feeds::own_role(app, &actor.channel_id, got, who, m);
    };
    // What can be checked before anything goes: a token that may grant. Slots can't be
    // (Helix has no count), so a refused grant puts the old role back below.
    can_step(app, actor, mod_role)?;
    mark(true);
    if let Err(r) = role_step(app, actor, who, !mod_role, false).await {
        mark(false);
        return Err(r.text);
    }
    standing::changed(app, &actor.channel, who, !mod_role, false);
    if let Err(r) = role_step(app, actor, who, mod_role, true).await {
        // Nobody should end up with neither: the old role goes back, its echo adds no line.
        let back = rc::action(!mod_role, true);
        feeds::own_role(app, &actor.channel_id, back, who, true);
        let restored = role_step(app, actor, who, !mod_role, true).await.is_ok();
        feeds::own_role(app, &actor.channel_id, got, who, false);
        if restored {
            standing::changed(app, &actor.channel, who, !mod_role, true);
        } else {
            feeds::own_role(app, &actor.channel_id, gone, who, false);
            feeds::own_role(app, &actor.channel_id, back, who, false);
        }
        // The echoes may be gone already: chat says what happened too.
        let line = rc::grant_failed(who, mod_role, &r.text, r.slots, restored);
        crate::chat::notice_line(app, &actor.channel, cyex_chat::model::NoticeKind::Moderation, &line);
        return Err(line);
    }
    standing::changed(app, &actor.channel, who, mod_role, true);
    crate::chat::notice_line(app, &actor.channel, cyex_chat::model::NoticeKind::Moderation, &rc::swapped(who, mod_role));
    Ok(())
}

/// A refused role change: the line to show, and whether Twitch said they hold the other role.
struct RoleRefused {
    text: String,
    clash: bool,
    /// V107-M8 A VIP grant with no slots left.
    slots: bool,
}

impl From<&str> for RoleRefused {
    fn from(text: &str) -> Self {
        text.to_owned().into()
    }
}

impl From<String> for RoleRefused {
    fn from(text: String) -> Self {
        RoleRefused { text, clash: false, slots: false }
    }
}

/// 1.50 Helix takes only the broadcaster's token; lead mods (and mods, Twitch decides) use GQL.
fn role_helix<'a>(app: &AppHandle, actor: &'a Actor, mod_role: bool) -> Option<cyex_twitch::helix::Auth<'a>> {
    let scope = if mod_role { "channel:manage:moderators" } else { "channel:manage:vips" };
    if actor.role == ModRole::Broadcaster { actor.helix_scoped(app, scope) } else { None }
}

/// V107-M8 Whether this account has a way to change `mod_role` at all, before a swap takes
/// anything away.
fn can_step(app: &AppHandle, actor: &Actor, mod_role: bool) -> Result<(), String> {
    let helix = role_helix(app, actor, mod_role).is_some();
    cyex_moderation::role_change::path(actor.role, helix, actor.gql().is_some()).map(|_| ()).map_err(Into::into)
}

async fn role_step(app: &AppHandle, actor: &Actor, who: &str, mod_role: bool, on: bool) -> Result<(), RoleRefused> {
    use cyex_moderation::role_change::{self as rc, Path};
    let helix = role_helix(app, actor, mod_role);
    let gql = actor.gql();
    match rc::path(actor.role, helix.is_some(), gql.is_some())? {
        Path::Helix => {
            let hrole = if mod_role { HRole::Mod } else { HRole::Vip };
            let h = helix.ok_or(rc::NO_TOKEN)?;
            hr::set_role(&http(app), h, &actor.channel_id, &target_id(app, who).await?, hrole, on)
                .await
                .map_err(|e| RoleRefused {
                    // V107-M8 422: "the user is a VIP/moderator, you must first remove them".
                    clash: on && status(&e) == 422,
                    slots: on && rc::no_slots(mod_role, "", status(&e)),
                    text: codes::helix(status(&e), e.api_message().unwrap_or("")),
                })
        }
        Path::Gql => {
            let g = gql.ok_or(rc::NO_TOKEN)?;
            match gql::moderation::set_role(&http(app), g, &actor.channel_id, who, mod_role, on).await {
                Ok(None) => Ok(()),
                Ok(Some(code)) => Err(RoleRefused {
                    clash: on && rc::clash_code(mod_role, &code),
                    slots: on && rc::no_slots(mod_role, &code, 0),
                    text: rc::refused(actor.role, &code, codes::gql(&code, who)),
                }),
                Err(e) => Err(rc::refused(actor.role, &e.to_string(), e.to_string()).into()),
            }
        }
    }
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
    "This account can’t moderate. Sign in again in Settings → Accounts.".into()
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

#[cfg(test)]
mod tests {
    #[test]
    fn v107_p7_the_delete_line_quotes_the_text() {
        let line = super::deleted_line;
        assert_eq!(line(Some("cyexbot"), Some("p7  1")), "Deleted message from cyexbot: “p7 1”");
        let long = format!("{}b", "a".repeat(50));
        assert_eq!(line(Some("cyexbot"), Some(&long)), format!("Deleted message from cyexbot: “{}…”", "a".repeat(50)));
        // Text unknown or empty, then sender unknown.
        assert_eq!(line(Some("cyexbot"), None), "Deleted a message from cyexbot.");
        assert_eq!(line(Some("cyexbot"), Some(" 
 ")), "Deleted a message from cyexbot.");
        assert_eq!(line(None, Some("hi")), "Deleted a message.");
        assert_eq!(line(None, None), "Deleted a message.");
    }

    use super::*;

    #[test]
    fn only_helix_409_means_another_pin_holds_the_spot() {
        let api = |status| cyex_twitch::Error::Api { status, message: String::new() };
        assert!(pin_taken(&api(409)));
        assert!(!pin_taken(&api(400)));
        assert!(!pin_taken(&api(403)));
        assert!(!pin_taken(&cyex_twitch::Error::Refused("CONFLICT".into())));
    }

    #[test]
    fn a_failed_pin_of_the_pinned_message_unpins_it_first() {
        // V107 Same message pinned again (a new length): take its pin down, then pin.
        assert_eq!(repin_over(Some(("pin-1", "msg-a")), "msg-a"), Some("pin-1"));
        // Another message pinned: GQL replaces it itself, so a failure is a real one and the
        // old pin stays up.
        assert_eq!(repin_over(Some(("pin-1", "msg-b")), "msg-a"), None);
        assert_eq!(repin_over(None, "msg-a"), None);
        assert_eq!(repin_over(Some(("", "msg-a")), "msg-a"), None);
    }
}
