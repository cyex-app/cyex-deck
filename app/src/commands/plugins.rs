//! Lua plugins (S5): the list in Settings, turning them on, their settings, the console,
//! Tab completion and menu entries on lines, chats and names.

use cyex_plugins::{CompletionRequest, MenuPlace, NameUser};
use serde::Serialize;
use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;

use crate::chat::peek;
use crate::plugins::logs::{self, PluginLog};
use crate::plugins::{self, PluginsView, calls, hook_line};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn plugins_view(app: AppHandle) -> PluginsView {
    plugins::view(&app)
}

/// On agrees to the permissions its info file asks for now.
#[tauri::command]
pub fn plugins_set_on(app: AppHandle, id: String, on: bool) -> CmdResult<()> {
    plugins::set_on(&app, &id, on)
}

#[tauri::command]
pub fn plugins_set_value(app: AppHandle, id: String, key: String, value: Value) -> CmdResult<()> {
    plugins::set_value(&app, &id, &key, value)
}

/// Reads the folder again and restarts the plugin; without an id, every plugin.
#[tauri::command]
pub fn plugins_reload(app: AppHandle, id: Option<String>) {
    plugins::reload(&app, id.as_deref());
}

#[tauri::command]
pub fn plugins_open_folder(app: AppHandle, id: Option<String>) -> CmdResult<()> {
    let dir = plugins::folder(&app, id.as_deref());
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plugins_logs(app: AppHandle, id: String) -> Vec<PluginLog> {
    logs::lines(&app, &id)
}

#[tauri::command]
pub fn plugins_clear_log(app: AppHandle, id: String) {
    logs::clear(&app, &id);
}

/// Runs a line of Lua in the plugin, for its console.
#[tauri::command]
pub async fn plugins_repl(app: AppHandle, id: String, code: String) -> CmdResult<String> {
    let p = plugins::get(&app, &id).ok_or("Turn the plugin on first.")?;
    calls::repl(&p, &code).await
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginCompletion {
    pub values: Vec<String>,
    pub hide_others: bool,
}

/// Tab in a chat's message box.
#[tauri::command]
pub async fn plugins_complete(
    app: AppHandle,
    view: u32,
    query: String,
    full_text: String,
    cursor: usize,
    first_word: bool,
) -> PluginCompletion {
    let channel = peek::channel_of(&app, view).unwrap_or_default();
    let c = calls::complete(
        &app,
        CompletionRequest {
            query,
            full_text_content: full_text,
            cursor_position: cursor,
            is_first_word: first_word,
            channel,
        },
    )
    .await;
    PluginCompletion {
        values: c.values,
        hide_others: c.hide_others,
    }
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginMenuEntry {
    pub plugin: String,
    pub index: usize,
    pub label: String,
    /// S9 A line's menu or the chat's ⋯ menu.
    pub place: MenuPlace,
    pub group: Option<String>,
    /// Kinds of line that show it; empty for all.
    pub lines: Vec<String>,
}

/// Every running plugin's message menu entries.
#[tauri::command]
pub fn plugins_menus(app: AppHandle) -> Vec<PluginMenuEntry> {
    plugins::running(&app)
        .iter()
        .flat_map(|p| {
            p.exports()
                .menu
                .into_iter()
                .enumerate()
                .map(|(index, e)| PluginMenuEntry {
                    plugin: p.id.clone(),
                    index,
                    label: e.label,
                    place: e.place,
                    group: e.group,
                    lines: e.lines,
                })
        })
        .collect()
}

#[tauri::command]
pub async fn plugins_menu_run(
    app: AppHandle,
    view: u32,
    plugin: String,
    index: usize,
    msg_id: Option<String>,
) -> CmdResult<()> {
    let p = plugins::get(&app, &plugin).ok_or("That plugin isn't running.")?;
    let (channel, line) = match msg_id.and_then(|id| peek::message(&app, view, &id)) {
        Some((channel, m)) => {
            let line = hook_line(&channel, &m);
            (channel, Some(line))
        }
        None => (peek::channel_of(&app, view).ok_or("This chat is closed.")?, None),
    };
    calls::menu(&p, index, &channel, (line, None)).await
}

/// S11 A plugin's entry on a name: who, in which chat, and the line the name was on. Their
/// id and display name come from that line or their latest one in the chat.
#[tauri::command]
pub async fn plugins_name_run(
    app: AppHandle,
    plugin: String,
    index: usize,
    channel: String,
    login: String,
    view: Option<u32>,
    msg_id: Option<String>,
) -> CmdResult<()> {
    let p = plugins::get(&app, &plugin).ok_or("That plugin isn't running.")?;
    let line = view
        .zip(msg_id)
        .and_then(|(v, id)| peek::message(&app, v, &id))
        .map(|(c, m)| hook_line(&c, &m));
    let (id, name) = line
        .as_ref()
        .and_then(|l| l.sender.as_ref())
        .filter(|s| s.login == login)
        .map(|s| (s.id.clone(), s.name.clone()))
        .or_else(|| peek::sender(&app, &channel, &login))
        .unzip();
    let user = NameUser { login, id, name };
    calls::menu(&p, index, &channel, (line, Some(user))).await
}
