//! IPC for moderation (spec 09): role check for the gate (1.57), running a command, the
//! mod view's snapshot and queue actions, mass actions and `/nuke`.

use cyex_moderation::command;
use cyex_twitch::gql::modlogs::{Chatters, ModHistory, UnbanRequest};
use tauri::AppHandle;

use crate::moderation::nuke::NukePreview;
use crate::moderation::view::Board0;
use crate::moderation::{self, Role};

/// 1.57 "broadcaster", "moderator", or null. The UI shows mod controls only when set.
#[tauri::command]
pub fn mod_role(app: AppHandle, account: String, channel: String) -> Option<String> {
    moderation::role(&app, &account, &channel).map(|r| {
        match r {
            Role::Broadcaster => "broadcaster",
            Role::Moderator => "moderator",
        }
        .to_owned()
    })
}

/// Which open account moderates a channel, for the tab's mod-view shield.
#[tauri::command]
pub fn mod_moderating(app: AppHandle, channel: String) -> Option<String> {
    crate::chat::moderated_open(&app)
        .into_iter()
        .find(|(_, l)| *l == channel.to_ascii_lowercase())
        .map(|(acc, _)| acc)
}

/// Runs a mod command typed or clicked, e.g. `ban spammer bots`. Returns a line to show.
#[tauri::command]
pub async fn mod_run(app: AppHandle, account: String, channel: String, name: String, args: String) -> Result<Option<String>, String> {
    if moderation::nuke::is_nuke(&name) {
        moderation::nuke::run(&app, &account, &channel, &args).await?;
        return Ok(None);
    }
    let parsed = command::parse(&name, &args).ok_or_else(|| format!("/{name} isn't a mod command."))?;
    moderation::run::command(&app, &account, &channel, parsed?).await
}

/// The channel id for a login, so the UI can read its board and subscribe to updates.
#[tauri::command]
pub fn mod_channel_id(app: AppHandle, channel: String) -> Option<String> {
    moderation::channel_id(&app, &channel)
}

#[tauri::command]
pub fn mod_view(app: AppHandle, channel_id: String) -> Board0 {
    moderation::view::snapshot(&app, &channel_id)
}

#[tauri::command]
pub async fn mod_automod(app: AppHandle, account: String, channel: String, msg_id: String, allow: bool) -> Result<(), String> {
    moderation::queue::resolve_automod(&app, &account, &channel, &msg_id, allow).await
}

#[tauri::command]
pub async fn mod_unban_requests(app: AppHandle, account: String, channel: String) -> Result<Vec<UnbanRequest>, String> {
    moderation::queue::unban_requests(&app, &account, &channel).await
}

#[tauri::command]
pub async fn mod_resolve_unban(app: AppHandle, account: String, channel: String, id: String, approve: bool, note: String) -> Result<(), String> {
    moderation::queue::resolve_unban(&app, &account, &channel, &id, approve, &note).await
}

#[tauri::command]
pub async fn mod_history(app: AppHandle, account: String, channel: String, target_id: String) -> Result<ModHistory, String> {
    moderation::queue::history(&app, &account, &channel, &target_id).await
}

#[tauri::command]
pub async fn mod_comment_add(app: AppHandle, account: String, channel: String, target_id: String, text: String, shareable: bool) -> Result<(), String> {
    moderation::queue::add_comment(&app, &account, &channel, &target_id, &text, shareable).await
}

#[tauri::command]
pub async fn mod_comment_delete(app: AppHandle, account: String, channel: String, id: String) -> Result<(), String> {
    moderation::queue::delete_comment(&app, &account, &channel, &id).await
}

#[tauri::command]
pub async fn mod_chatters(app: AppHandle, account: String, channel: String) -> Result<Chatters, String> {
    moderation::queue::chatters(&app, &account, &channel).await
}

/// M1 A `/nuke` preview: who it would hit, before running it.
#[tauri::command]
pub fn mod_nuke_preview(app: AppHandle, account: String, channel: String, args: String) -> Result<NukePreview, String> {
    moderation::nuke::preview(&app, &account, &channel, &args)
}

/// 1.48, 1.55 Mass ban (or timeout when `secs` is set) a list of logins.
#[tauri::command]
pub async fn mod_massban(app: AppHandle, account: String, channel: String, logins: Vec<String>, secs: Option<u32>, reason: String) -> Result<u32, String> {
    let actor = moderation::actor(&app, &account, &channel)?;
    let mut pacer = cyex_moderation::pacer::Pacer::default();
    moderation::massban::run(&app, &actor, &logins, secs, &reason, &mut pacer).await
}

#[tauri::command]
pub fn mod_massban_stop(app: AppHandle, channel_id: String) {
    moderation::massban::stop(&app, &channel_id);
}
