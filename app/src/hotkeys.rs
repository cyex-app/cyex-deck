//! S2 Global hotkeys: bindings in the hotkeys partition whose action starts with
//! `global.`, registered with Windows so they work while another app is in front. In-app
//! hotkeys stay in the UI (`ui/src/lib/hotkeys.svelte.ts`).

use std::collections::HashMap;
use std::sync::Mutex;

use cyex_settings::Partition;
use cyex_settings::schema::Hotkeys;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::settings::Settings;

/// Combos Windows wouldn't give us (another app holds them), for Settings → Hotkeys.
const FAILED_EVENT: &str = "hotkeys:failed";

#[derive(Default)]
pub struct Bound {
    /// Shortcut id → action.
    actions: Mutex<HashMap<u32, String>>,
    failed: Mutex<Vec<String>>,
}

pub fn plugin() -> TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let action = app
                .state::<Bound>()
                .actions
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .get(&shortcut.id())
                .cloned();
            if let Some(action) = action {
                run(app, &action);
            }
        })
        .build()
}

pub fn init(app: &AppHandle) {
    app.manage(Bound::default());
    apply(app);
}

/// Registers the global bindings again after Settings → Hotkeys changed.
pub fn apply(app: &AppHandle) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let hotkeys: Hotkeys = app.state::<Settings>().read(Partition::Hotkeys);
    let mut bound = HashMap::new();
    let mut failed = Vec::new();
    for (action, combo) in &hotkeys.bindings {
        if !action.starts_with("global.") || combo.is_empty() {
            continue;
        }
        match parse(combo) {
            Some(s) if gs.register(s).is_ok() => {
                bound.insert(s.id(), action.clone());
            }
            _ => failed.push(combo.clone()),
        }
    }
    let state = app.state::<Bound>();
    *state.actions.lock().unwrap_or_else(|p| p.into_inner()) = bound;
    *state.failed.lock().unwrap_or_else(|p| p.into_inner()) = failed.clone();
    let _ = app.emit(FAILED_EVENT, failed);
}

/// Combos that couldn't be registered last time.
pub fn failed(app: &AppHandle) -> Vec<String> {
    app.state::<Bound>()
        .failed
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

/// `Ctrl+Shift+K` (the UI's names) into a shortcut. `Win` is the Windows key.
fn parse(combo: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = combo
        .split('+')
        .map(|p| if p == "Win" { "Super" } else { p })
        .collect();
    // A lone key (or Shift plus one) would swallow typing everywhere.
    let (_, mods) = parts.split_last()?;
    if !mods.iter().any(|m| matches!(*m, "Ctrl" | "Alt" | "Super")) {
        return None;
    }
    parts.join("+").parse().ok()
}

fn run(app: &AppHandle, action: &str) {
    match action {
        "global.show" => crate::windows::toggle_main(app),
        "global.streamer" => crate::streamer::toggle(app),
        "global.clickThrough" => crate::windows::overlay::toggle_click_through(app),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ui_combos() {
        assert!(parse("Ctrl+Shift+K").is_some());
        assert!(parse("Ctrl+Alt+F5").is_some());
        assert!(parse("Win+Shift+O").is_some());
        assert_eq!(parse("Ctrl+K"), parse("Ctrl+KeyK"));
        assert!(parse("K").is_none());
        assert!(parse("Shift+K").is_none());
        assert!(parse("Ctrl+Nope").is_none());
    }
}
