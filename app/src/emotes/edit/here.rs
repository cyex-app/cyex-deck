//! I106-3 This chat's channel as an edit target, for the chat commands.

use cyex_emotes::seventv::account::sets::EmoteBrief;
use tauri::AppHandle;

use super::reach::Target;
use super::{lock, state};

/// The cached target whose `owner_id` is `channels[login].seventv_user` (Emotes state), cloned.
/// No network.
pub fn here_target(app: &AppHandle, account: &str, login: &str) -> Option<Target> {
    let (owner, _) = super::viewed(app, login, "", "");
    let owner = owner?;
    let reach = lock(&state(app).reach);
    reach
        .get(account)?
        .1
        .as_ref()?
        .targets
        .iter()
        .find(|t| t.owner_id == owner)
        .cloned()
}

/// Tab for `/remove` and `/rename`: that target's aliases, sorted. Empty when not editable or
/// not cached.
pub fn here_names(app: &AppHandle, account: &str, login: &str) -> Vec<String> {
    let mut names: Vec<String> = here_target(app, account, login)
        .map(|t| t.entries.into_iter().map(|(_, alias)| alias).collect())
        .unwrap_or_default();
    names.sort_unstable();
    names.dedup();
    names
}

/// Own 7TV name, private flag, owner: a loaded emote, else 7TV. `Ok(None)` = no such emote;
/// `Err(())` = 7TV down.
#[allow(clippy::result_unit_err)]
pub async fn emote_facts(app: &AppHandle, login: &str, id: &str) -> Result<Option<EmoteBrief>, ()> {
    if let Some(e) = super::find_emote(app, login, id) {
        return Ok(Some(EmoteBrief {
            id: e.id.clone(),
            name: e.original.unwrap_or(e.name),
            private: e.private,
            owner_id: e.owner_id,
        }));
    }
    crate::cosmetics::api(app).emote_brief(id).await.map_err(|e| {
        eprintln!("7tv edit: emote {id}: {e}");
    })
}
