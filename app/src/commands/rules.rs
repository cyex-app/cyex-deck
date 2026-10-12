//! Checks for the settings editors (F3 filters, 1.8 commands) and the S1 menus.

use cyex_chat::commands::CommandList;
use cyex_chat::commands::menu::{MenuEntry, parse_menu};
use cyex_chat::filter::{self, Filter, Type};
use cyex_settings::Partition;
use cyex_settings::schema::Commands;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::settings::Settings;

/// F3 Why a filter doesn't read, or `None` when it's fine.
#[tauri::command]
pub fn filter_check(expr: String) -> Option<String> {
    Filter::parse(&expr).err()
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FilterName {
    pub name: String,
    /// `bool`, `number`, `text` or `list`.
    pub kind: String,
}

/// F3 What a filter can use, for hints in the editor.
#[tauri::command]
pub fn filter_names() -> Vec<FilterName> {
    filter::names()
        .map(|(name, ty)| FilterName {
            name: name.to_owned(),
            kind: match ty {
                Type::Bool => "bool",
                Type::Int => "number",
                Type::Str => "text",
                Type::List => "list",
                Type::Regex => "regex",
            }
            .to_owned(),
        })
        .collect()
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CommandLineError {
    /// From 0.
    pub line: u32,
    pub message: String,
}

/// 1.8 Lines of the custom command list that don't read.
#[tauri::command]
pub fn commands_check(lines: Vec<String>) -> Vec<CommandLineError> {
    CommandList::new(&lines)
        .errors
        .into_iter()
        .map(|e| CommandLineError {
            line: e.line as u32,
            message: e.message,
        })
        .collect()
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Menus {
    pub user: Vec<MenuEntry>,
    pub message: Vec<MenuEntry>,
    pub channel: Vec<MenuEntry>,
    pub usercard: Vec<MenuEntry>,
}

/// S1 The user's menu entries and usercard buttons.
#[tauri::command]
pub fn chat_menus(app: AppHandle) -> Menus {
    let c: Commands = app.state::<Settings>().read(Partition::Commands);
    Menus {
        user: parse_menu(&c.user_menu),
        message: parse_menu(&c.message_menu),
        channel: parse_menu(&c.channel_menu),
        usercard: parse_menu(&c.usercard_buttons),
    }
}
