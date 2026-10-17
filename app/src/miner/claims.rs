//! Bonus chests (C22), moments and raids. The points they bring arrive as Hermes
//! `points-earned` events and go to the stats there; the log says what was claimed.

use cyex_settings::Partition;
use cyex_settings::schema::Tabs;
use cyex_twitch::gql::miner::{self as gm, Claimed};
use tauri::{AppHandle, Manager};

use super::log::LogKind;
use super::{gql_auth, http, lock, log};
use crate::settings::Settings;

/// A channel's name for the log: from the miner, else from a tab, else its id.
pub(super) fn label(app: &AppHandle, channel_id: &str) -> String {
    if let Some((_, name)) = super::name_of(app, channel_id) {
        return name;
    }
    let tabs: Tabs = app.state::<Settings>().read(Partition::Tabs);
    tabs.tiles()
        .find(|t| t.channel.id == channel_id)
        .map_or_else(|| channel_id.to_owned(), |t| t.channel.name.clone())
}

/// Whose account it was, when it isn't the miner's.
fn by(app: &AppHandle, account: &str) -> String {
    if lock(app).account.as_deref() == Some(account) {
        return String::new();
    }
    crate::auth::accounts(app)
        .list
        .iter()
        .find(|a| a.user_id == account)
        .map(|a| format!(" as {}", a.display_name))
        .unwrap_or_default()
}

pub async fn bonus(app: &AppHandle, account: &str, channel_id: &str, claim_id: &str) {
    let Some((token, client_id)) = crate::auth::gql_token(app, account) else {
        return;
    };
    let name = label(app, channel_id);
    match gm::claim_bonus(&http(app), gql_auth(&token, &client_id), channel_id, claim_id).await {
        // The miner's own account logs it with the points, from Hermes.
        Ok(Claimed::Balance(_)) => {
            let by = by(app, account);
            if !by.is_empty() {
                log::add(app, LogKind::Claim, Some(&name), format!("Claimed a bonus chest{by}."));
            }
        }
        Ok(Claimed::Refused(code)) => {
            eprintln!("miner: chest in {name} refused: {code}");
            log::add(app, LogKind::Error, Some(&name), "Couldn’t claim the chest.");
        }
        Err(e) => {
            eprintln!("miner: chest in {name}: {e}");
            log::add(app, LogKind::Error, Some(&name), "Couldn’t claim the chest.");
        }
    }
}

pub async fn moment(app: &AppHandle, account: &str, channel_id: &str, moment_id: &str) {
    let Some((token, client_id)) = crate::auth::gql_token(app, account) else {
        return;
    };
    let name = label(app, channel_id);
    match gm::claim_moment(&http(app), gql_auth(&token, &client_id), moment_id).await {
        Ok(None) => log::add(app, LogKind::Claim, Some(&name), "Claimed a moment."),
        Ok(Some(code)) => {
            eprintln!("miner: moment in {name} refused: {code}");
            log::add(app, LogKind::Error, Some(&name), "Couldn’t claim the moment.");
        }
        Err(e) => {
            eprintln!("miner: moment in {name}: {e}");
            log::add(app, LogKind::Error, Some(&name), "Couldn’t claim the moment.");
        }
    }
}

/// Joins a raid once, so its points come along.
pub async fn raid(app: &AppHandle, account: &str, channel_id: &str, raid_id: &str, target: &str) {
    if !lock(app).raids.insert(raid_id.to_owned()) {
        return;
    }
    let Some((token, client_id)) = crate::auth::gql_token(app, account) else {
        return;
    };
    let name = label(app, channel_id);
    match gm::join_raid(&http(app), gql_auth(&token, &client_id), raid_id).await {
        Ok(()) => log::add(app, LogKind::Raid, Some(&name), format!("Joined the raid to {target}.")),
        Err(e) => {
            eprintln!("miner: raid from {name}: {e}");
            log::add(app, LogKind::Error, Some(&name), "Couldn’t join the raid.");
        }
    }
}
