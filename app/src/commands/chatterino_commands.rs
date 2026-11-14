//! V111-06 Commands from Chatterino: pick its commands.json, preview, import. Only the
//! command list is read.

use std::sync::Mutex;

use cyex_settings::Partition;
use cyex_settings::chatterino::{CommandImport, CommandPreview, FoundCommands, apply_commands, parse_commands, plan_commands};
use cyex_settings::schema::Commands;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::settings::Settings;
use crate::tabs::import as chatterino;

use super::CmdResult;

/// A commands.json this big isn't one.
const MAX_BYTES: u64 = 16 * 1024 * 1024;
const NOT_COMMANDS: &str = "That file isn’t Chatterino’s commands.json.";

/// What the last pick found, until the import or the next pick.
static PICKED: Mutex<Option<FoundCommands>> = Mutex::new(None);

fn picked() -> std::sync::MutexGuard<'static, Option<FoundCommands>> {
    PICKED.lock().unwrap_or_else(|p| p.into_inner())
}

/// The app's and Twitch's own command names (lower case).
fn reserved(name: &str) -> bool {
    crate::chat::run::reserved(name)
}

fn read(path: &std::path::Path) -> CmdResult<FoundCommands> {
    let size = std::fs::metadata(path)
        .map_err(|_| "Couldn’t open that file.")?
        .len();
    if size > MAX_BYTES {
        return Err(NOT_COMMANDS.into());
    }
    let bytes = std::fs::read(path).map_err(|_| "Couldn’t open that file.")?;
    let text = String::from_utf8(bytes).map_err(|_| NOT_COMMANDS)?;
    parse_commands(&text, &reserved).map_err(|_| NOT_COMMANDS.into())
}

/// Picks Chatterino's commands.json (starting in its Settings folder) and says what importing
/// would do. `None` = cancelled.
#[tauri::command]
pub async fn commands_chatterino_preview(
    app: AppHandle,
    settings: State<'_, Settings>,
) -> CmdResult<Option<CommandPreview>> {
    // V110-F85 The picker blocks: off the runtime's workers.
    let (asked, start) = (app.clone(), chatterino::folder(&app));
    let chosen = tokio::task::spawn_blocking(move || {
        let mut dialog = asked
            .dialog()
            .file()
            .set_title("Import commands from Chatterino")
            .add_filter("Chatterino commands", &["json"])
            .add_filter("All files", &["*"]);
        if let Some(dir) = start {
            dialog = dialog.set_directory(dir);
        }
        dialog.blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(source) = chosen else {
        return Ok(None);
    };
    let path = source.into_path().map_err(|e| e.to_string())?;
    let found = read(&path)?;
    let current: Commands = settings.read(Partition::Commands);
    let preview = plan_commands(&found, &current, &reserved);
    *picked() = Some(found);
    Ok(Some(preview))
}

/// Adds the picked commands. `take`: ids of conflicts that take Chatterino's; `menu`: also
/// add their message-menu entries.
#[tauri::command]
pub fn commands_chatterino_import(
    app: AppHandle,
    settings: State<Settings>,
    take: Vec<u32>,
    menu: bool,
) -> CmdResult<CommandImport> {
    let found = picked().take().ok_or("Pick the file again.")?;
    let mut result = CommandImport::default();
    settings.update::<Commands>(&app, Partition::Commands, |c| {
        result = apply_commands(found, c, &reserved, &take, menu);
        result.added + result.replaced + result.menu > 0
    })?;
    Ok(result)
}

/// Drops what the last pick found (the dialog was closed).
#[tauri::command]
pub fn commands_chatterino_cancel() {
    *picked() = None;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cyex_chat::commands::{self, CommandList, Host, Items, Params};

    use super::*;

    struct NoHost;

    impl Host for NoHost {
        fn custom(&self, _: &str) -> Option<Arc<Items>> {
            None
        }
        fn get(&self, _: &str, _: Option<&str>) -> Option<String> {
            None
        }
        fn request(&mut self, _: &str, _: &[(String, String)]) -> Result<String, String> {
            Err("no".into())
        }
        fn random(&mut self, _: u64) -> u64 {
            0
        }
        fn now_ms(&self) -> i64 {
            0
        }
    }

    const FILE: &str = r#"{"commands": [
        {"name": "/hi", "func": "/me waves at {1} and {2+} in {channel.name}", "showInMsgContextMenu": false},
        {"name": "/game", "func": "Playing {stream.game;nothing} as {my.name;nobody}", "showInMsgContextMenu": false},
        {"name": "/odd", "func": "costs $5 \\o/ {paint} a,b) {{x}} {1}-{2}", "showInMsgContextMenu": false},
        {"name": "/quote", "func": "\"{msg.text}\" - {user.name;someone}", "showInMsgContextMenu": true}
    ]}"#;

    fn run(row: &str, args: &str, values: &[(&str, &str)]) -> String {
        let body = row.split_once(' ').map_or("", |(_, b)| b);
        let items = commands::parse(body).unwrap_or_else(|e| panic!("{row}: {e}"));
        let mut p = Params::new(args);
        for (k, v) in values {
            p.set(k, *v);
        }
        commands::run(&items, &p, &mut NoHost).unwrap()
    }

    #[test]
    fn imported_rows_read_and_run_like_in_chatterino() {
        let found = parse_commands(FILE, &reserved).unwrap();
        let rows: Vec<String> = found.commands.iter().map(|c| c.row.clone()).collect();
        // The command list takes every row.
        assert!(CommandList::new(&rows).errors.is_empty());
        assert_eq!(run(&rows[0], "bob a b", &[("chan", "forsen")]), "/me waves at bob and a b in forsen");
        assert_eq!(run(&rows[1], "", &[("my-nick", "me")]), "Playing nothing as me");
        assert_eq!(run(&rows[1], "", &[("streamgame", "Celeste")]), "Playing Celeste as nobody");
        assert_eq!(run(&rows[2], "x y", &[]), "costs $5 \\o/ {paint} a,b) {x}} x-y");
        assert_eq!(run(&rows[3], "", &[]), "\"\" - someone");
        let menu = found.commands[3].menu.clone().unwrap().unwrap();
        let entry = menu.strip_prefix(".quote=").unwrap();
        let items = commands::parse(entry).unwrap();
        let mut p = Params::new("bob");
        p.set("nick", "bob");
        p.set("msg", "hello there");
        assert_eq!(commands::run(&items, &p, &mut NoHost).unwrap(), "\"hello there\" - bob");
        let mut empty = Params::new("");
        empty.set("msg", "hi");
        assert_eq!(commands::run(&items, &empty, &mut NoHost).unwrap(), "\"hi\" - someone");
    }

    #[test]
    fn built_ins_are_conflicts() {
        let found = parse_commands(r#"{"commands":[{"name":"/ban","func":"/ban {1} spam"}]}"#, &reserved).unwrap();
        let p = plan_commands(&found, &Commands::default(), &reserved);
        assert_eq!(p.items[0].conflict, cyex_settings::chatterino::Conflict::BuiltIn);
    }
}
