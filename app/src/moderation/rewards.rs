//! V107-R43 Reward requests in the mod view: the unfulfilled channel points redemptions of a
//! channel the account moderates, completed or rejected (refunded) one or many. The same GQL
//! operations as the stream manager's queue (V107-R37): Twitch's own mod view runs them
//! with a mod's token on the channel (docs/research/twitch-api.md, 2026/10/21). Bits
//! rewards (power-ups) never show: they can't be refunded.

use cyex_twitch::gql::dashboard::rewards::{Queue, Rewards, Status};
use tauri::AppHandle;

use crate::streammanager::bulk::Bulk;
use crate::streammanager::gql_call::{GqlCall, gql_call_as};
use crate::streammanager::rewards as sm;

/// Twitch's refusal as a line, about their channel, not yours.
fn said(e: &cyex_twitch::Error) -> String {
    match e {
        cyex_twitch::Error::Refused(c) if c.starts_with("CHANNEL_POINTS_NOT_") => {
            "Channel points are off on this channel.".into()
        }
        _ => sm::said(e),
    }
}

/// The account's call on the channel: a mod of it (or its broadcaster), signed in to GQL.
fn call(app: &AppHandle, account: &str, login: &str) -> Result<GqlCall, String> {
    let actor = super::actor(app, account, login)?;
    gql_call_as(app, account, actor.channel_id, actor.channel)
}

/// The channel's rewards, for the filter and the waiting counts.
pub async fn list(app: &AppHandle, account: &str, login: &str) -> Result<Rewards, String> {
    sm::list_on(&call(app, account, login)?, said).await
}

/// Newest first; `reward` narrows it to one reward.
pub async fn queue(
    app: &AppHandle,
    account: &str,
    login: &str,
    reward: Option<String>,
    cursor: Option<String>,
) -> Result<Queue, String> {
    sm::queue_on(&call(app, account, login)?, reward, cursor, said).await
}

/// Paced and stoppable, through the stream manager's one bulk gate.
pub async fn set_status(app: &AppHandle, account: &str, login: &str, ids: Vec<String>, s: Status) -> Result<Bulk, String> {
    let c = call(app, account, login)?;
    sm::set_status_on(app, &c, ids, s, said).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_r43_lines_speak_of_their_channel() {
        let r = |c: &str| said(&cyex_twitch::Error::Refused(c.into()));
        assert_eq!(r("CHANNEL_POINTS_NOT_ENABLED"), "Channel points are off on this channel.");
        assert_eq!(r("CHANNEL_POINTS_NOT_AVAILABLE"), "Channel points are off on this channel.");
        assert_eq!(r("RATE_LIMITED"), "Too fast. Try again in a moment.");
    }
}
