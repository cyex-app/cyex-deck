//! C17 Translating chat (D23): one message on request, or every message of a chat that
//! has "Translate messages" on. Auto lines are gathered for a moment and sent in one
//! request; ones already in your language, or that come back the same, are left alone.
//! Emotes, mentions and links stay out of the text that's sent.

use std::sync::Mutex;
use std::time::Duration;

use cyex_chat::model::{ChatMessage, Flag, Fragment, Translation};
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use tauri::{AppHandle, Manager};

use super::{Chat, RoomKey, queue};
use crate::settings::Settings;

/// Auto lines wait this long to be sent together.
const GATHER: Duration = Duration::from_millis(400);
/// More waiting than this and the rest are dropped, not queued (moltorino's cap).
const MAX_WAITING: usize = 40;

#[derive(Default)]
pub struct Translator {
    /// The window's language, from the UI (`navigator.language`).
    ui_lang: Mutex<String>,
    waiting: Mutex<Vec<(RoomKey, String, String)>>,
    /// V107-A9 Chats that translate everything, kept from the settings so a chat line
    /// doesn't read them (a read parses the whole chat partition under the settings lock).
    chats: Mutex<Chats>,
}

/// V107-A9 The chats with "Translate messages" on.
#[derive(Default)]
struct Chats(std::collections::HashSet<String>);

impl Chats {
    fn set(&mut self, logins: &[String]) {
        self.0 = logins.iter().cloned().collect();
    }

    fn on(&self, login: &str) -> bool {
        self.0.contains(login)
    }
}

pub fn init(app: &AppHandle) {
    app.manage(Translator::default());
    on_settings(app, &app.state::<Settings>().read::<ChatSettings>(Partition::Chat).translate_chats);
}

/// The chat settings changed (`chat::on_settings`).
pub fn on_settings(app: &AppHandle, translate_chats: &[String]) {
    if let Some(tr) = app.try_state::<Translator>() {
        tr.chats.lock().unwrap_or_else(|p| p.into_inner()).set(translate_chats);
    }
}

pub fn set_ui_language(app: &AppHandle, lang: &str) {
    let base = lang.split(['-', '_']).next().unwrap_or("en").to_ascii_lowercase();
    *app.state::<Translator>()
        .ui_lang
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = base;
}

/// The language to translate into: the setting, else the app's.
pub fn target(app: &AppHandle) -> String {
    let set = app
        .state::<Settings>()
        .read::<ChatSettings>(Partition::Chat)
        .translate_to;
    if !set.is_empty() {
        return set;
    }
    let ui = app
        .state::<Translator>()
        .ui_lang
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    if ui.is_empty() { "en".into() } else { ui }
}

/// The text worth translating: words only.
fn words(m: &ChatMessage) -> Option<String> {
    let text: Vec<&str> = m
        .body
        .iter()
        .filter_map(|f| match f {
            Fragment::Text { text } => Some(text.trim()),
            _ => None,
        })
        .filter(|t| !t.is_empty())
        .collect();
    let text = text.join(" ");
    (text.chars().any(char::is_alphabetic)).then_some(text)
}

/// Right-click → Translate.
pub async fn one(app: &AppHandle, view: u32, id: &str) -> Result<(), String> {
    let (key, text) = {
        let chat = app.state::<Chat>();
        let inner = chat.lock();
        let key = inner
            .views
            .get(&view)
            .map(|v| v.0.clone())
            .ok_or("This chat is closed.")?;
        let text = inner
            .rooms
            .get(&key)
            .and_then(|e| e.room.text_of(id))
            .ok_or("Nothing to translate there.")?;
        (key, text)
    };
    let to = target(app);
    let http = app.state::<crate::auth::Auth>().http.clone();
    let out = cyex_twitch::translate::translate(&http, std::slice::from_ref(&text), &to)
        .await
        .map_err(|_| "Translation isn’t working right now.".to_owned())?;
    let t = out.into_iter().next().ok_or("Nothing came back.")?;
    if same(&t, &text, &to) {
        return Err("That’s already in your language.".into());
    }
    put(app, &key, id, t.text, t.from);
    Ok(())
}

/// The text came back unchanged, or was in the target language already.
fn same(t: &cyex_twitch::translate::Translated, original: &str, to: &str) -> bool {
    t.text.is_empty()
        || t.from.eq_ignore_ascii_case(to)
        || t.text.trim().eq_ignore_ascii_case(original.trim())
}

fn put(app: &AppHandle, key: &RoomKey, id: &str, text: String, from: String) {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    if let Some(entry) = inner.rooms.get_mut(key)
        && let Some(ev) = entry.room.set_translation(id, Translation { text, from })
    {
        queue(entry, [ev], &chat.wake);
    }
}

/// New live messages in a room: queued when that chat translates everything.
pub fn auto(app: &AppHandle, key: &RoomKey, messages: &[std::sync::Arc<ChatMessage>]) {
    let on = app
        .try_state::<Translator>()
        .is_some_and(|tr| tr.chats.lock().unwrap_or_else(|p| p.into_inner()).on(&key.1));
    if !on {
        return;
    }
    let picked: Vec<(RoomKey, String, String)> = messages
        .iter()
        .filter(|m| m.sender.is_some() && m.notice.is_none() && !m.has(Flag::Own) && !m.has(Flag::History))
        .filter_map(|m| Some((key.clone(), m.id.clone(), words(m)?)))
        .collect();
    if picked.is_empty() {
        return;
    }
    let first = {
        let tr = app.state::<Translator>();
        let mut waiting = tr.waiting.lock().unwrap_or_else(|p| p.into_inner());
        let first = waiting.is_empty();
        let room = MAX_WAITING.saturating_sub(waiting.len());
        waiting.extend(picked.into_iter().take(room));
        first
    };
    if first {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(GATHER).await;
            flush(&app).await;
        });
    }
}

async fn flush(app: &AppHandle) {
    let batch = std::mem::take(
        &mut *app
            .state::<Translator>()
            .waiting
            .lock()
            .unwrap_or_else(|p| p.into_inner()),
    );
    if batch.is_empty() {
        return;
    }
    let to = target(app);
    let texts: Vec<String> = batch.iter().map(|(_, _, t)| t.clone()).collect();
    let http = app.state::<crate::auth::Auth>().http.clone();
    let Ok(out) = cyex_twitch::translate::translate(&http, &texts, &to).await else {
        return;
    };
    for ((key, id, original), t) in batch.into_iter().zip(out) {
        if !same(&t, &original, &to) {
            put(app, &key, &id, t.text, t.from);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_a9_the_kept_chats_follow_the_settings() {
        let mut c = Chats::default();
        c.set(&["a".into()]);
        assert!(c.on("a") && !c.on("b"));
        c.set(&["b".into()]);
        assert!(!c.on("a") && c.on("b"));
    }
}
