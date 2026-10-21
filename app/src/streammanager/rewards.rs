//! V107-R37 Channel points rewards on the manager's own channel: list, create, edit,
//! pause, turn off, delete, and the redemption queue (complete or reject). GQL only,
//! like Twitch's dashboard: Helix can only touch rewards its own client made.

use cyex_twitch::gql::dashboard::rewards::{
    self as api, COOLDOWN_MAX, PROMPT_MAX, Queue, Reward, RewardEdit, Rewards, Status, Switch, TITLE_MAX,
};
use std::time::Duration;
use tauri::AppHandle;

use super::{Call, words};

const GQL_ONLY: &str = "Rewards need the sign-in with a code (Settings → Accounts).";
/// Twitch's queue sends at most 50 ids per call.
const BATCH: usize = 50;
const PACE: Duration = Duration::from_millis(400);
/// Twitch's cap on a reward's cost.
pub const COST_MAX: u64 = 1_000_000_000;

#[cfg(feature = "mocks")]
const MOCK_VAR: &str = "CYEX_GQL_REWARDS_MOCK";

/// A local GQL mock in mocks builds (`http://127.0.0.1:<port>`), Twitch otherwise.
#[cfg(feature = "mocks")]
fn base() -> Option<String> {
    crate::mock_url::loopback_base(&std::env::var(MOCK_VAR).ok()?)
}

#[cfg(not(feature = "mocks"))]
fn base() -> Option<String> {
    None
}

fn call(app: &AppHandle) -> Result<Call, String> {
    let c = Call::new(app)?;
    if c.has_gql() { Ok(c) } else { Err(GQL_ONLY.into()) }
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
    let c = call(app)?;
    api::list(&c.http, c.gql(), base().as_deref(), &c.channel_id)
        .await
        .map_err(|e| said(&e))
}

pub async fn create(app: &AppHandle, edit: RewardEdit) -> Result<Reward, String> {
    checked(&edit)?;
    let c = call(app)?;
    let b = base();
    let made = api::create(&c.http, c.gql(), b.as_deref(), &c.channel_id, &edit)
        .await
        .map_err(|e| said(&e))?;
    if edit.enabled {
        return Ok(made);
    }
    // New rewards start on; off is a second call.
    api::switch(
        &c.http,
        c.gql(),
        b.as_deref(),
        &c.channel_id,
        &made.id,
        Switch::Enabled(false),
    )
    .await
    .map_err(|e| said(&e))
}

pub async fn update(app: &AppHandle, id: String, edit: RewardEdit) -> Result<Reward, String> {
    checked(&edit)?;
    let c = call(app)?;
    api::update(&c.http, c.gql(), base().as_deref(), &c.channel_id, &id, &edit)
        .await
        .map_err(|e| said(&e))
}

pub async fn switch(app: &AppHandle, id: String, s: Switch) -> Result<Reward, String> {
    let c = call(app)?;
    api::switch(&c.http, c.gql(), base().as_deref(), &c.channel_id, &id, s)
        .await
        .map_err(|e| said(&e))
}

pub async fn delete(app: &AppHandle, id: String, waiting: Option<Status>) -> Result<(), String> {
    let c = call(app)?;
    api::delete(&c.http, c.gql(), base().as_deref(), &c.channel_id, &id, waiting)
        .await
        .map_err(|e| said(&e))
}

pub async fn queue(app: &AppHandle, reward: Option<String>, cursor: Option<String>) -> Result<Queue, String> {
    let c = call(app)?;
    api::queue(
        &c.http,
        c.gql(),
        base().as_deref(),
        &c.channel_id,
        reward.as_deref(),
        cursor.as_deref(),
    )
    .await
    .map_err(|e| said(&e))
}

/// Completes or rejects (refunds) redemptions, 50 a call, paced. Returns how many went
/// through; a refusal midway stops there.
pub async fn set_status(app: &AppHandle, ids: Vec<String>, s: Status) -> Result<usize, String> {
    let c = call(app)?;
    let b = base();
    let mut done = 0;
    for (i, part) in batches(&ids).enumerate() {
        if i > 0 {
            tokio::time::sleep(PACE).await;
        }
        if let Err(e) = api::set_status(&c.http, c.gql(), b.as_deref(), &c.channel_id, part, s).await {
            return if done == 0 { Err(said(&e)) } else { Ok(done) };
        }
        done += part.len();
    }
    Ok(done)
}

fn batches(ids: &[String]) -> std::slice::Chunks<'_, String> {
    ids.chunks(BATCH)
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
    fn statuses_go_fifty_at_a_time() {
        let ids: Vec<String> = (0..121).map(|i| i.to_string()).collect();
        assert_eq!(
            batches(&ids).map(<[String]>::len).collect::<Vec<_>>(),
            [50, 50, 21]
        );
    }

    #[test]
    fn reward_codes_read_as_lines() {
        let r = |c: &str| said(&cyex_twitch::Error::Refused(c.into()));
        assert_eq!(r("DUPLICATE_REWARD"), "A reward with that name exists already.");
        assert_eq!(r("RATE_LIMITED"), "Too fast. Try again in a moment.");
    }
}
