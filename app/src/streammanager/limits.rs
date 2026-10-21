//! V107-R41 Every limit the dashboard panels check, from the Rust that checks it: the UI
//! reads them once instead of keeping its own copies.

use cyex_twitch::gql::dashboard::content::videos;
use cyex_twitch::gql::dashboard::{modset, rewards};
use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DashboardLimits {
    /// Chat rules: how many, characters each.
    pub rules_max: usize,
    pub rule_len: usize,
    /// Chat delays Twitch offers, seconds.
    pub delays: Vec<u32>,
    /// Blocked and allowed terms, characters.
    pub term_min: usize,
    pub term_max: usize,
    /// Channel points rewards.
    pub reward_title_max: usize,
    pub reward_prompt_max: usize,
    #[ts(type = "number")]
    pub reward_cost_max: u64,
    /// Seconds.
    pub reward_cooldown_max: u32,
    /// Video titles, characters.
    pub video_title_max: usize,
}

pub fn limits() -> DashboardLimits {
    DashboardLimits {
        rules_max: modset::MAX_RULES,
        rule_len: modset::MAX_RULE_LEN,
        delays: modset::DELAYS.to_vec(),
        term_min: modset::TERM_MIN,
        term_max: modset::TERM_MAX,
        reward_title_max: rewards::TITLE_MAX,
        reward_prompt_max: rewards::PROMPT_MAX,
        reward_cost_max: rewards::COST_MAX,
        reward_cooldown_max: rewards::COOLDOWN_MAX,
        video_title_max: videos::TITLE_MAX,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn v107_r41_limits_are_twitchs() {
        let l = super::limits();
        assert_eq!(
            (l.rules_max, l.rule_len, l.term_min, l.term_max),
            (15, 150, 2, 500)
        );
        assert_eq!(
            (l.reward_title_max, l.reward_prompt_max, l.video_title_max),
            (45, 200, 140)
        );
        assert_eq!(l.delays, [0, 2, 4, 6]);
    }
}
