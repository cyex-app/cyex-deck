//! S5 Asking plugins from async code: a line being sent, a command, Tab completion, a
//! menu entry, the console. Every ask has a deadline, so a stuck plugin can't hold the
//! chat box.

use std::time::Duration;

use cyex_plugins::{CALLBACK, Completion, CompletionRequest, Event, Line, NameUser, Plugin, QUICK, Reply, Sent};
use tauri::AppHandle;
use tokio::sync::oneshot;

/// Waits on one answer. `None`: not taken or too late.
async fn ask<T: Send + 'static>(wait: Duration, f: impl FnOnce(Reply<T>) -> bool) -> Option<T> {
    let (tx, rx) = oneshot::channel();
    if !f(Box::new(move |v| drop(tx.send(v)))) {
        return None;
    }
    tokio::time::timeout(wait, rx).await.ok()?.ok()
}

const SLACK: Duration = Duration::from_millis(150);
/// Commands, menu entries and the console may queue behind other work of the plugin.
const BUSY: Duration = Duration::from_secs(1);

/// A line the user sends, after plugins with a Sending callback. `Err`: a plugin stopped it.
pub async fn sending(app: &AppHandle, channel: &str, text: &str) -> Result<String, String> {
    let mut text = text.to_owned();
    for p in super::running(app).into_iter().filter(|p| p.has(Event::Sending)) {
        let t = text.clone();
        match ask(QUICK + SLACK, |r| p.sending(channel, &t, r)).await {
            Some(Sent::Text(new)) => text = new,
            Some(Sent::Stop) => return Err(format!("{} stopped this message.", super::name_of(app, &p.id))),
            Some(Sent::Unchanged) | None => {}
        }
    }
    Ok(text)
}

/// The plugin that registered `/name`, first by folder name.
pub fn command_owner(app: &AppHandle, name: &str) -> Option<std::sync::Arc<Plugin>> {
    super::running(app)
        .into_iter()
        .find(|p| p.exports().commands.iter().any(|c| c == name))
}

/// V1041-01 Every running plugin's commands, for Tab completion.
pub fn command_names(app: &AppHandle) -> Vec<String> {
    super::running(app)
        .into_iter()
        .flat_map(|p| p.exports().commands)
        .collect()
}

/// Runs a plugin's command. `Ok(Some(text))`: run that as if typed.
pub async fn command(p: &Plugin, name: &str, args: &str, channel: &str) -> Result<Option<String>, String> {
    let mut words = vec![format!("/{name}")];
    words.extend(args.split_whitespace().map(str::to_owned));
    ask(CALLBACK + BUSY, |r| p.command(name, words, channel, r))
        .await
        .ok_or_else(|| "The plugin didn't answer.".to_owned())?
}

/// Tab completion from every plugin that offers it.
pub async fn complete(app: &AppHandle, req: CompletionRequest) -> Completion {
    let mut out = Completion::default();
    for p in super::running(app)
        .into_iter()
        .filter(|p| p.has(Event::CompletionRequested))
    {
        let req = req.clone();
        if let Some(c) = ask(QUICK + SLACK, |r| p.complete(req, r)).await {
            out.hide_others |= c.hide_others;
            out.values.extend(c.values);
        }
    }
    out
}

/// A menu entry; name entries (S11) come with who was clicked.
pub async fn menu(p: &Plugin, index: usize, channel: &str, on: (Option<Line>, Option<NameUser>)) -> Result<(), String> {
    ask(CALLBACK + BUSY, |r| p.menu(index, channel, on, r))
        .await
        .ok_or_else(|| "The plugin didn't answer.".to_owned())?
}

pub async fn repl(p: &Plugin, code: &str) -> Result<String, String> {
    ask(Duration::from_secs(2) + BUSY, |r| p.repl(code, r))
        .await
        .ok_or_else(|| "The plugin didn't answer.".to_owned())?
}
