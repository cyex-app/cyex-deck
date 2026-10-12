//! 2.2 Twitch profile and Twitch's own notification settings for an account: display name
//! (capitalization only), bio, email/push/onsite switches, the live-notification choice per
//! followed channel. GQL sign-in needed.

use cyex_twitch::gql::notifications::{self as notify, ChannelNotify, Switch};
use cyex_twitch::gql::profile::{self as api, BIO_MAX, Profile};
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use crate::auth;
use crate::whispers::GqlCall;

const NO_GQL: &str = "This needs a GQL sign-in for the account (Settings → Accounts).";

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TwitchNotify {
    pub switches: Vec<Switch>,
    pub channels: Vec<ChannelNotify>,
    pub cursor: Option<String>,
}

fn call(app: &AppHandle, account: &str) -> Result<GqlCall, String> {
    GqlCall::new(app, account).map_err(|_| NO_GQL.to_owned())
}

fn failed(e: cyex_twitch::Error) -> String {
    if e.is_network() {
        "Couldn't reach Twitch.".into()
    } else {
        eprintln!("profile: {e}");
        "Twitch didn't take that. Try again in a bit.".into()
    }
}

pub async fn get(app: &AppHandle, account: &str) -> Result<Profile, String> {
    let c = call(app, account)?;
    api::profile(&c.http, c.auth()).await.map_err(failed)
}

pub async fn save(app: &AppHandle, account: &str, display_name: &str, bio: &str) -> Result<(), String> {
    let c = call(app, account)?;
    let me = api::profile(&c.http, c.auth()).await.map_err(failed)?;
    let name = display_name.trim();
    if !api::name_allowed(&me.login, name, &me.language) {
        return Err(format!("Twitch only lets you change the capitals: {}.", me.login));
    }
    if bio.chars().count() > BIO_MAX {
        return Err(format!("The bio can be {BIO_MAX} characters at most."));
    }
    api::update(&c.http, c.auth(), &me.id, name, bio.trim())
        .await
        .map_err(|e| match e.api_message() {
            Some("DISPLAY_NAME_ONLY_CAP") => {
                format!("Twitch only lets you change the capitals: {}.", me.login)
            }
            Some("DISPLAY_NAME_CHANGE_AGAIN") => {
                "Twitch lets you change your name once every 60 days.".into()
            }
            Some("DISPLAY_NAME_NOT_AVAILABLE") => "That name isn't available.".into(),
            Some("INVALID_CHARS_IN_DISPLAY_NAME") => "That name has characters Twitch doesn't allow.".into(),
            Some("DISPLAY_NAME_TOO_LONG" | "DISPLAY_NAME_TOO_SHORT") => {
                "That name is the wrong length.".into()
            }
            Some("DESCRIPTION_TOO_LONG") => format!("The bio can be {BIO_MAX} characters at most."),
            Some("DESCRIPTION_FAILED_MODERATION") => "Twitch didn't accept that bio.".into(),
            _ => failed(e),
        })?;
    // The account list shows the display name too.
    let name = name.to_owned();
    auth::update_accounts(app, |a| match a.get_mut(account) {
        Some(acc) if acc.display_name != name => {
            acc.display_name = name;
            true
        }
        _ => false,
    })?;
    auth::notify(app);
    Ok(())
}

pub async fn notify_settings(
    app: &AppHandle,
    account: &str,
    cursor: Option<String>,
) -> Result<TwitchNotify, String> {
    let c = call(app, account)?;
    let s = notify::settings(&c.http, c.auth(), cursor.as_deref())
        .await
        .map_err(failed)?;
    Ok(TwitchNotify {
        switches: s.switches,
        channels: s.channels,
        cursor: s.cursor,
    })
}

pub async fn set_switch(
    app: &AppHandle,
    account: &str,
    category: &str,
    platform: &str,
    on: bool,
) -> Result<(), String> {
    let c = call(app, account)?;
    notify::set_switch(&c.http, c.auth(), category, platform, on)
        .await
        .map_err(failed)
}

pub async fn set_channel(
    app: &AppHandle,
    account: &str,
    channel_id: &str,
    state: &str,
) -> Result<(), String> {
    let c = call(app, account)?;
    notify::set_channel(&c.http, c.auth(), channel_id, state)
        .await
        .map_err(failed)
}
