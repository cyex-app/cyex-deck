//! V1041-01 What Tab completes after `/` in a chat: the app's and Twitch's commands this
//! chat can use (`builtin::usable`), your own for its channel (1.8) and plugins' (S5).

use tauri::AppHandle;

use super::{builtin, list, place_of};

/// Command names without `/`, sorted, once each.
pub fn names(app: &AppHandle, view: u32) -> Result<Vec<String>, String> {
    let place = place_of(app, view)?;
    let account = place.account();
    let role = account.and_then(|a| crate::chat::roles::role(app, a, place.channel()));
    let mut names: Vec<String> = builtin::usable(role, account.is_some())
        .into_iter()
        .map(str::to_owned)
        .chain(list(app).names(place.channel()))
        .chain(crate::plugins::calls::command_names(app))
        .collect();
    names.sort_unstable();
    names.dedup();
    Ok(names)
}
