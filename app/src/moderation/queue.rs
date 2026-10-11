//! The mod view's reads and queue actions (1.48, 1.56, M2, M3): AutoMod allow/deny, the
//! unban request queue, a user's mod history and comments, and the chatter list. Each
//! picks Helix when the account has the scope, else GQL (D25).

use std::collections::HashSet;

use cyex_moderation::actions::ModLine;
use cyex_twitch::gql;
use cyex_twitch::gql::modlogs::{Chatters, ModHistory, UnbanRequest};
use cyex_twitch::helix::moderation as hm;
use tauri::AppHandle;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{codes, http};

fn err(e: cyex_twitch::Error) -> String {
    match &e {
        cyex_twitch::Error::Api { status, message } => codes::helix(*status, message),
        _ => e.to_string(),
    }
}

/// 1.56 Lets a held message through, or drops it.
pub async fn resolve_automod(app: &AppHandle, account: &str, login: &str, msg_id: &str, allow: bool) -> Result<(), String> {
    let actor = super::actor(app, account, login)?;
    if let Some(h) = actor.helix_scoped(app, "moderator:manage:automod") {
        return hm::automod(&http(app), h, account, msg_id, allow).await.map_err(err);
    }
    if let Some(g) = actor.gql() {
        return gql::moderation::resolve_automod(&http(app), g, msg_id, allow).await.map_err(err);
    }
    Err("This account can't moderate AutoMod.".into())
}

/// M2 The pending unban requests for a channel.
pub async fn unban_requests(app: &AppHandle, account: &str, login: &str) -> Result<Vec<UnbanRequest>, String> {
    let actor = super::actor(app, account, login)?;
    if let Some(g) = actor.gql() {
        return gql::modlogs::unban_requests(&http(app), g, login).await.map_err(err);
    }
    // Helix has them too, in a different shape; map to the GQL shape the UI uses.
    if let Some(h) = actor.helix_scoped(app, "moderator:read:unban_requests") {
        let list = hm::unban_requests(&http(app), h, actor.room()).await.map_err(err)?;
        return Ok(list
            .into_iter()
            .map(|r| UnbanRequest {
                id: r.id,
                user_id: r.user_id,
                login: r.user_login,
                display: r.user_name,
                text: r.text,
                created_at: r.created_at,
            })
            .collect());
    }
    Err("This account can't read unban requests.".into())
}

/// M2 Approves or denies a request, with a note the viewer sees.
pub async fn resolve_unban(app: &AppHandle, account: &str, login: &str, id: &str, approve: bool, note: &str) -> Result<(), String> {
    let actor = super::actor(app, account, login)?;
    if let Some(g) = actor.gql() {
        return gql::modlogs::resolve_unban(&http(app), g, id, approve, note).await.map_err(err);
    }
    if let Some(h) = actor.helix_scoped(app, "moderator:manage:unban_requests") {
        return hm::resolve_unban(&http(app), h, actor.room(), id, approve, note).await.map_err(err);
    }
    Err("This account can't resolve unban requests.".into())
}

/// M3 A user's mod history and comments (GQL only).
pub async fn history(app: &AppHandle, account: &str, login: &str, target_id: &str) -> Result<ModHistory, String> {
    let actor = super::actor(app, account, login)?;
    let g = actor.gql().ok_or("Mod history needs a GQL sign-in.")?;
    let channel_id = actor.channel_id.clone();
    gql::modlogs::history(&http(app), g, &channel_id, target_id).await.map_err(err)
}

/// V103-15 The next page of the channel's mod-action history into its board (GQL only):
/// the first page once, or with `older` the page after the ones shown. Mods only (V102-21).
pub async fn channel_history(app: &AppHandle, account: &str, login: &str, older: bool) -> Result<(), String> {
    // Your own channel needs no open chat to say you're its broadcaster (the stream manager).
    let own = crate::auth::accounts(app)
        .list
        .iter()
        .any(|a| a.user_id == account && a.login.eq_ignore_ascii_case(login));
    let cid = if own { account.to_owned() } else { super::actor(app, account, login)?.channel_id };
    let (token, client_id) = crate::auth::gql_token(app, account).ok_or("Earlier mod actions need a GQL sign-in.")?;
    let g = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    let Some(after) = super::view::history_next(app, &cid, older) else { return Ok(()) };
    match gql::modlogs::channel_actions(&http(app), g, &cid, after.as_deref()).await {
        Ok(page) => {
            let lines = page.actions.iter().map(|a| ModLine::past(a, millis(&a.at))).collect();
            super::view::add_history(app, &cid, Some((lines, page.cursor)), after);
            Ok(())
        }
        Err(e) => {
            super::view::add_history(app, &cid, None, after);
            Err(err(e))
        }
    }
}

/// RFC 3339 as unix milliseconds; 0 when unreadable.
fn millis(at: &str) -> i64 {
    OffsetDateTime::parse(at, &Rfc3339).map_or(0, |t| (t.unix_timestamp_nanos() / 1_000_000) as i64)
}

/// M8 A page of a user's messages from the channel's mod logs (GQL only).
pub async fn mod_log_messages(
    app: &AppHandle,
    account: &str,
    login: &str,
    target_id: &str,
    cursor: Option<&str>,
) -> Result<gql::modlogs::ModLogPage, String> {
    let actor = super::actor(app, account, login)?;
    let g = actor.gql().ok_or("Mod logs need a GQL sign-in.")?;
    gql::modlogs::messages(&http(app), g, &actor.channel_id, target_id, cursor)
        .await
        .map_err(err)
}

/// M3 Writes a mod comment on a user.
pub async fn add_comment(app: &AppHandle, account: &str, login: &str, target_id: &str, text: &str, shareable: bool) -> Result<(), String> {
    let actor = super::actor(app, account, login)?;
    let g = actor.gql().ok_or("Mod comments need a GQL sign-in.")?;
    gql::modlogs::add_comment(&http(app), g, &actor.channel_id, target_id, text, shareable).await.map_err(err)
}

pub async fn delete_comment(app: &AppHandle, account: &str, login: &str, id: &str) -> Result<(), String> {
    let actor = super::actor(app, account, login)?;
    let g = actor.gql().ok_or("Mod comments need a GQL sign-in.")?;
    gql::modlogs::delete_comment(&http(app), g, &actor.channel_id, id).await.map_err(err)
}

/// 1.48, V101-35 Who's in a chat. Twitch's grouped list over GQL works for anyone; a
/// moderator with a Helix sign-in gets every viewer instead of Twitch's sample of 100.
pub async fn chatters(app: &AppHandle, account: Option<&str>, login: &str) -> Result<Chatters, String> {
    // The grouped read works signed out with the GQL client.
    let auth = gql::Auth {
        client_id: cyex_twitch::clients::GQL,
        token: None,
    };
    let grouped = gql::modlogs::chatters(&http(app), auth, login).await;
    let actor = account.and_then(|a| super::actor(app, a, login).ok());
    let helix = actor.as_ref().and_then(|a| Some((a.helix_scoped(app, "moderator:read:chatters")?, a.room())));
    let Some((h, room)) = helix else {
        return grouped.map_err(err)?.ok_or_else(|| "This channel doesn't exist or is suspended.".into());
    };
    let (list, count) = cyex_twitch::helix::roles::chatters(&http(app), h, room).await.map_err(err)?;
    let mut c = grouped.ok().flatten().unwrap_or_default();
    let named: HashSet<String> = [&c.broadcasters, &c.moderators, &c.vips, &c.staff].into_iter().flatten().cloned().collect();
    c.viewers = list.into_iter().map(|u| u.user_login).filter(|l| !named.contains(l)).collect();
    c.viewers.sort_unstable();
    c.count = count;
    c.complete = c.listed() as u64 >= count;
    Ok(c)
}
