//! V108-C37 Cards in the app's top right corner: someone went live, a drop came in, a badge
//! was earned. They sit next to the Windows pop-ups (`toasts.rs`) and use the same click
//! targets. Which kinds show is Settings → Notifications → In-app cards. The main window draws
//! them (`NoticeCards.svelte`); streamer mode (U4) keeps them off with the pop-ups.

use cyex_settings::Partition;
use cyex_settings::schema::{InAppCards, Notifications};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::settings::Settings;
use crate::toasts::Open;
use crate::windows::MAIN;

const EVENT: &str = "notify:card";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "CardKind")]
pub enum Kind {
    Live,
    Drop,
    Badge,
}

impl Kind {
    fn wanted(self, c: &InAppCards) -> bool {
        match self {
            Kind::Live => c.live,
            Kind::Drop => c.drops,
            Kind::Badge => c.badges,
        }
    }
}

/// What the corner stack draws.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "NoticeCard")]
pub struct Card {
    pub kind: Kind,
    pub title: String,
    pub text: String,
    /// Picture URL (channel avatar, drop or badge art).
    pub image: Option<String>,
    pub open: Open,
}

/// Shows `card` if its kind is on in the settings and streamer mode lets pop-ups through.
pub fn show(app: &AppHandle, card: Card) {
    let n: Notifications = app.state::<Settings>().read(Partition::Notifications);
    if !card.kind.wanted(&n.cards) || crate::streamer::hides(app, |s| s.no_notifications) {
        return;
    }
    let _ = app.emit_to(MAIN, EVENT, card);
}

/// Test builds only: draws one card of `kind` (`live`, `drop`, `badge`; `badge-read` runs the real badge read) the way the real source
/// would, through the same settings check. Call from the page: `invoke('dev_card', { kind })`.
#[cfg(feature = "mocks")]
#[tauri::command]
pub fn dev_card(app: AppHandle, kind: String) -> Result<(), String> {
    if kind == "badge-read" {
        // The real source: read the account's badges again now (a mock identity server can add one).
        tauri::async_runtime::spawn(async move { crate::badges::check(&app).await });
        return Ok(());
    }
    // V108-C39 `live-tab` (the first tab's channel: the card offers "Go to tab"), `live-notab`
    // (a channel with no tab: "Open tab") or `live:<login>`.
    let login = match kind.as_str() {
        "live-tab" => Some(
            app.state::<Settings>()
                .read::<cyex_settings::schema::Tabs>(Partition::Tabs)
                .tiles()
                .map(|t| t.channel.login.clone())
                .find(|l| !l.is_empty())
                .ok_or("no tab with a channel")?,
        ),
        "live-notab" => Some("nobodyhasatabforme".to_owned()),
        other => other.strip_prefix("live:").map(str::to_owned),
    };
    if let Some(login) = login {
        let open = Open::Channel { login: login.clone() };
        show(&app, Card { kind: Kind::Live, title: format!("{login} is live"), text: "Just chatting\nA long title to see how it wraps".into(), image: None, open });
        return Ok(());
    }
    if let Some(rest) = kind.strip_prefix("live-message:") {
        // `live-message:<user id>:<login>`: a live card with that channel's real go-live message (read only).
        let (id, login) = rest.split_once(':').ok_or("live-message:<id>:<login>")?;
        let (id, login) = (id.to_owned(), login.to_owned());
        tauri::async_runtime::spawn(async move {
            let http = app.state::<crate::auth::Auth>().http.clone();
            let said = cyex_twitch::gql::live_message::live_messages(&http, std::slice::from_ref(&id)).await;
            let text = said.ok().and_then(|mut m| m.remove(&id)).unwrap_or_else(|| "Stream title (no message set)".into());
            let open = Open::Channel { login: login.clone() };
            show(&app, Card { kind: Kind::Live, title: format!("{login} is live"), text: format!("{text}\nJust Chatting"), image: None, open });
        });
        return Ok(());
    }
    let (kind, title, text, open) = match kind.as_str() {
        "live" => (Kind::Live, "Mock channel is live", "Just chatting\nA long title to see how it wraps", Open::Channel { login: "mockchannel".into() }),
        "drop" => (Kind::Drop, "Drop claimed", "Mock Helmet (Mock Game)", Open::Drops),
        "badge" => (Kind::Badge, "New badge", "Mock Cheer 1.000", Open::Settings { page: "emotes".into() }),
        other => return Err(format!("unknown kind {other}")),
    };
    show(
        &app,
        Card { kind, title: title.into(), text: text.into(), image: None, open },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_follows_its_switch() {
        let c = InAppCards { live: true, drops: false, badges: true };
        assert!(Kind::Live.wanted(&c) && !Kind::Drop.wanted(&c) && Kind::Badge.wanted(&c));
    }
}
