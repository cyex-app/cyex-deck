//! The mod view's reads and queue actions (1.48, 1.56, M2, M3): AutoMod allow/deny, the
//! unban request queue, a user's mod history and comments, and the chatter list. Each
//! picks Helix when the account has the scope, else GQL (D25).

use cyex_twitch::gql;
use cyex_twitch::gql::modlogs::{Chatters, ModHistory, UnbanRequest};
use cyex_twitch::helix::moderation as hm;
use tauri::AppHandle;

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

/// 1.48 The chatter list, grouped. Reads with the Android TV client (signed out works).
pub async fn chatters(app: &AppHandle, account: &str, login: &str) -> Result<Chatters, String> {
    let actor = super::actor(app, account, login)?;
    if let Some(h) = actor.helix_scoped(app, "moderator:read:chatters") {
        let (list, count) = cyex_twitch::helix::roles::chatters(&http(app), h, actor.room()).await.map_err(err)?;
        return Ok(Chatters {
            count,
            broadcasters: Vec::new(),
            moderators: Vec::new(),
            vips: Vec::new(),
            staff: Vec::new(),
            viewers: list.into_iter().map(|u| u.user_login).collect(),
        });
    }
    // GQL's grouped chatters read works with the Android TV client.
    let auth = gql::Auth {
        client_id: cyex_twitch::clients::ANDROID_TV,
        token: None,
    };
    gql::modlogs::chatters(&http(app), auth, login).await.map_err(err)
}
