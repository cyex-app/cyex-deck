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

/// V1051-04 Tab's candidates for an argument of `/combo`, `/paint` or `/badge` (raw names; the UI
/// ranks them). `sub` is `/combo`'s sub-command when the caret is past it.
pub async fn args(app: &AppHandle, view: u32, command: &str, sub: Option<&str>) -> Result<Vec<String>, String> {
    use std::time::Duration;

    use crate::cosmetics::{self, Kind, lookup};

    let place = place_of(app, view)?;
    let Some(account) = place.account() else {
        return Ok(Vec::new());
    };
    let kind = match command {
        "combo" => {
            let sub = sub.map(str::to_ascii_lowercase);
            return Ok(match sub.as_deref() {
                None => cyex_settings::schema::WORDS
                    .iter()
                    .map(|w| (*w).to_owned())
                    .chain(cosmetics::list_combos(app, account).into_iter().map(|c| c.name))
                    .collect(),
                Some("delete" | "rename") => cosmetics::list_combos(app, account).into_iter().map(|c| c.name).collect(),
                Some(_) => Vec::new(),
            });
        }
        "paint" => Kind::Paint,
        "badge" => Kind::Badge,
        _ => return Ok(Vec::new()),
    };
    let mut out = vec!["none".to_owned()];
    // Without consent `read` answers from settings alone, so 7TV isn't called.
    if let Ok(Ok(inv)) = tokio::time::timeout(Duration::from_secs(2), cosmetics::read(app, account)).await {
        out.extend(lookup::names(&inv, kind));
    }
    Ok(out)
}
