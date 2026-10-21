//! V107-R37 Channel points rewards on the manager's own channel: list, create, edit,
//! pause, turn off, delete, and the redemption queue (complete or reject). GQL only,
//! like Twitch's dashboard: Helix can only touch rewards its own client made. Calls go
//! through `gql_call` (V107-R41: the mock never gets the real token), bulk status through
//! `bulk`.

use cyex_twitch::gql::dashboard::rewards::{
    self as api, COOLDOWN_MAX, COST_MAX, PROMPT_MAX, Queue, Reward, RewardEdit, Rewards, STATUS_MAX, Status, Switch,
    TITLE_MAX,
};
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use super::bulk::{self, Bulk};
use super::gql_call::gql_call;
use super::words;

/// A saved reward, and what didn't go with it (CORR-03).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmRewardSaved")]
pub struct Saved {
    pub reward: Reward,
    /// Made, but a second step failed: shown as is.
    pub warning: Option<String>,
}

/// Twitch's refusal as a short line.
fn said(e: &cyex_twitch::Error) -> String {
    let line = match e {
        cyex_twitch::Error::Refused(code) => match code.as_str() {
            "DUPLICATE_REWARD" => "A reward with that name exists already.",
            "TOO_MANY_REWARDS" => "That's as many rewards as Twitch allows.",
            "NOT_FOUND" | "REWARD_NOT_FOUND" => "That reward is gone. Refresh the list.",
            "CHANNEL_POINTS_NOT_ENABLED" | "CHANNEL_POINTS_NOT_AVAILABLE" => {
                "Channel points are off for your channel."
            }
            "AUTOMOD_FAILED" | "TITLE_USES_BANNED_WORDS" | "PROMPT_USES_BANNED_WORDS" => {
                "Twitch blocked some words in that."
            }
            _ => return words::gql(e),
        },
        _ => return words::gql(e),
    };
    line.into()
}

/// What's wrong with a reward before it's sent, one line each.
pub fn check(e: &RewardEdit) -> Vec<String> {
    let mut out = Vec::new();
    let title = e.title.trim().chars().count();
    if title == 0 {
        out.push("Give it a name.".into());
    } else if title > TITLE_MAX {
        out.push(format!("Names can be {TITLE_MAX} characters."));
    }
    if e.prompt.trim().chars().count() > PROMPT_MAX {
        out.push(format!("Descriptions can be {PROMPT_MAX} characters."));
    }
    if !(1..=COST_MAX).contains(&e.cost) {
        out.push("Cost is 1 or more.".into());
    }
    if !is_color(&e.color) {
        out.push("Pick a color.".into());
    }
    if e.cooldown.is_some_and(|s| s == 0 || s > COOLDOWN_MAX) {
        out.push("Cooldowns run 1 second to 7 days.".into());
    }
    if e.max_per_stream == Some(0) || e.max_per_user == Some(0) {
        out.push("Limits are 1 or more.".into());
    }
    out
}

fn is_color(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn checked(e: &RewardEdit) -> Result<(), String> {
    match check(e).into_iter().next() {
        Some(line) => Err(line),
        None => Ok(()),
    }
}

pub async fn list(app: &AppHandle) -> Result<Rewards, String> {
    let c = gql_call(app)?;
    api::list(&c.channel()).await.map_err(|e| said(&e))
}

/// New rewards start on; off is a second call. When that fails the reward is still made:
/// it comes back with a warning, not as an error (CORR-03).
pub async fn create(app: &AppHandle, edit: RewardEdit) -> Result<Saved, String> {
    checked(&edit)?;
    let c = gql_call(app)?;
    let ch = c.channel();
    let made = api::create(&ch, &edit).await.map_err(|e| said(&e))?;
    if edit.enabled {
        return Ok(Saved { reward: made, warning: None });
    }
    Ok(match api::switch(&ch, &made.id, Switch::Enabled(false)).await {
        Ok(reward) => Saved { reward, warning: None },
        Err(e) => Saved { reward: made, warning: Some(off_failed(&e)) },
    })
}

fn off_failed(e: &cyex_twitch::Error) -> String {
    format!("Made it, but it’s still on. {}", said(e))
}

pub async fn update(app: &AppHandle, id: String, edit: RewardEdit) -> Result<Reward, String> {
    checked(&edit)?;
    let c = gql_call(app)?;
    api::update(&c.channel(), &id, &edit).await.map_err(|e| said(&e))
}

pub async fn switch(app: &AppHandle, id: String, s: Switch) -> Result<Reward, String> {
    let c = gql_call(app)?;
    api::switch(&c.channel(), &id, s).await.map_err(|e| said(&e))
}

pub async fn delete(app: &AppHandle, id: String, waiting: Option<Status>) -> Result<(), String> {
    let c = gql_call(app)?;
    api::delete(&c.channel(), &id, waiting).await.map_err(|e| said(&e))
}

pub async fn queue(app: &AppHandle, reward: Option<String>, cursor: Option<String>) -> Result<Queue, String> {
    let c = gql_call(app)?;
    api::queue(&c.channel(), reward.as_deref(), cursor.as_deref())
        .await
        .map_err(|e| said(&e))
}

/// Completes or rejects (refunds) redemptions, 50 a call, paced (`bulk`). `done` lists the
/// ids that went; a refusal midway stops there and says why (CORR-02).
pub async fn set_status(app: &AppHandle, ids: Vec<String>, s: Status) -> Result<Bulk, String> {
    let c = &gql_call(app)?;
    bulk::run(app, ids, STATUS_MAX, said, |part| async move {
        api::set_status(&c.channel(), &part, s).await.map(|()| part)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit() -> RewardEdit {
        RewardEdit {
            title: "Hydrate".into(),
            prompt: String::new(),
            cost: 100,
            color: "#145AFF".into(),
            enabled: true,
            input_required: false,
            skip_queue: false,
            cooldown: None,
            max_per_stream: None,
            max_per_user: None,
        }
    }

    #[test]
    fn a_reward_needs_a_name_a_cost_and_sane_limits() {
        assert!(check(&edit()).is_empty());
        let bad = RewardEdit {
            title: " ".into(),
            cost: 0,
            color: "blue".into(),
            cooldown: Some(COOLDOWN_MAX + 1),
            max_per_user: Some(0),
            ..edit()
        };
        assert_eq!(
            check(&bad),
            [
                "Give it a name.",
                "Cost is 1 or more.",
                "Pick a color.",
                "Cooldowns run 1 second to 7 days.",
                "Limits are 1 or more."
            ]
        );
        let long = RewardEdit {
            title: "x".repeat(46),
            prompt: "y".repeat(201),
            ..edit()
        };
        assert_eq!(check(&long).len(), 2);
    }

    #[test]
    fn v107_r41_a_failed_off_is_a_warning() {
        let w = off_failed(&cyex_twitch::Error::Refused("FORBIDDEN".into()));
        assert_eq!(w, "Made it, but it’s still on. Twitch didn’t allow that.");
    }

    #[test]
    fn reward_codes_read_as_lines() {
        let r = |c: &str| said(&cyex_twitch::Error::Refused(c.into()));
        assert_eq!(r("DUPLICATE_REWARD"), "A reward with that name exists already.");
        assert_eq!(r("RATE_LIMITED"), "Too fast. Try again in a moment.");
    }
}
