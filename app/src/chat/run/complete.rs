//! V1041-01 What Tab completes after `/` in a chat: the app's and Twitch's commands this
//! chat can use (`builtin::usable`), your own for its channel (1.8) and plugins' (S5).

use tauri::AppHandle;

use super::{builtin, list, place_of};

/// Command names without `/`, sorted, once each.
pub fn names(app: &AppHandle, view: u32) -> Result<Vec<String>, String> {
    let place = place_of(app, view, false)?;
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
/// ranks them), and I106-3 `/remove` and `/rename`: this channel’s 7TV emote names (V111-03 see
/// `emote_args`). `sub` is `/combo`’s sub-command when the caret is past it.
pub async fn args(app: &AppHandle, view: u32, command: &str, sub: Option<&str>) -> Result<Vec<String>, String> {
    use std::time::Duration;

    use crate::cosmetics::{self, Kind, lookup};

    let place = place_of(app, view, false)?;
    // V111-03 The 7TV emote commands: `sub` "#" asks for channels, else it is the `#channel`
    // typed (or none), whose emote names come back.
    if let Some((service, verb)) = super::builtin::emote_service(command) {
        return Ok(emote_args(app, place.account(), place.channel(), service, verb, sub));
    }
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

/// V111-03 Tab for the emote commands. No network. V112-17 cyex.tv's bare ones take the
/// names of the set an open chat holds; the 7 ones what 7TV lets the account edit.
fn emote_args(
    app: &AppHandle,
    account: Option<&str>,
    here: &str,
    service: super::builtin::EmoteService,
    command: &str,
    sub: Option<&str>,
) -> Vec<String> {
    use super::builtin::EmoteService;
    use crate::emotes::edit;

    if service == EmoteService::Cyextv {
        if sub == Some("#") {
            return edit::channel_names(app, None, here);
        }
        let channel = sub.filter(|s| edit::is_login(s)).unwrap_or(here);
        return match command {
            "add" => Vec::new(),
            "steal" | "yoink" if channel == here => Vec::new(),
            _ => super::cyextv_edit::loaded_names(app, channel),
        };
    }

    if sub == Some("#") {
        return edit::channel_names(app, account, here);
    }
    let channel = sub.filter(|s| edit::is_login(s)).unwrap_or(here);
    match command {
        "remove" | "rename" => match account {
            Some(a) if channel == here => edit::here_names(app, a, here),
            Some(a) => edit::names_for(app, a, channel),
            None => Vec::new(),
        },
        "steal" | "yoink" if account.is_some() && channel != here => edit::loaded_names(app, channel),
        "whoadded" => edit::loaded_names(app, channel),
        _ => Vec::new(),
    }
}
