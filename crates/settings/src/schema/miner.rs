use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Schema, TabChannel};

/// 1.31
const MAX_CHANNELS: usize = 500;
/// 1.44
pub const MAX_FIRST: usize = 2;

/// The points miner (08). Channels are kept by user id (1.15).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Miner {
    /// D24 Runs whenever the app runs, unless switched off here.
    pub enabled: bool,
    /// The account that mines; `None` is the default account.
    pub account: Option<String>,
    /// W3
    pub source: MineSource,
    /// 1.31 Your list, in priority order.
    pub channels: Vec<TabChannel>,
    /// W3 Follows the miner leaves out.
    pub skip: Vec<TabChannel>,
    /// 1.44 Up to two channels mined before all others.
    pub first: Vec<TabChannel>,
    /// What goes first after those two, in order. The rest follows the list.
    pub order: Vec<Rule>,
    pub claim_moments: bool,
    pub join_raids: bool,
    pub claim_drops: bool,
    /// C22 Bonus chests in open tabs too, with each tab's account.
    pub claim_in_tabs: bool,
    /// W1 Streams playing in the app count as watching, like on twitch.tv.
    pub watching_earns: bool,
    /// W2 Spends points on community goals.
    pub goals: bool,
    /// Join the chats of mined channels, e.g. for bots that reward chatters.
    pub chat: ChatPresence,
    pub bets: Bets,
}

impl Default for Miner {
    fn default() -> Self {
        Self {
            enabled: true,
            account: None,
            source: MineSource::List,
            channels: Vec::new(),
            skip: Vec::new(),
            first: Vec::new(),
            order: vec![Rule::Streak, Rule::Drops],
            claim_moments: true,
            join_raids: true,
            claim_drops: true,
            claim_in_tabs: true,
            watching_earns: true,
            goals: false,
            chat: ChatPresence::Never,
            bets: Bets::default(),
        }
    }
}

impl Schema for Miner {
    fn sanitize(&mut self) {
        let valid = |c: &TabChannel| !c.id.is_empty() && !c.login.is_empty();
        let mut seen = HashSet::new();
        self.channels.retain(|c| valid(c) && seen.insert(c.id.clone()));
        self.channels.truncate(MAX_CHANNELS);
        let mut seen = HashSet::new();
        self.skip.retain(|c| valid(c) && seen.insert(c.id.clone()));
        self.skip.truncate(MAX_CHANNELS);
        let mut seen = HashSet::new();
        self.first.retain(|c| valid(c) && seen.insert(c.id.clone()));
        self.first.truncate(MAX_FIRST);
        let mut seen = HashSet::new();
        self.order.retain(|r| seen.insert(*r));
        self.account = self.account.take().filter(|a| !a.is_empty());
        self.bets.sanitize();
    }
}

impl Miner {
    /// 1.44 Makes a channel one of the two mined first, pushing out the older one.
    /// Returns the channel that made room, if any.
    pub fn put_first(&mut self, c: TabChannel) -> Option<TabChannel> {
        if self.first.iter().any(|f| f.id == c.id) {
            return None;
        }
        let out = (self.first.len() >= MAX_FIRST).then(|| self.first.remove(0));
        self.first.push(c);
        out
    }

    /// 1.34 Adds a channel at the bottom of the list.
    pub fn add(&mut self, c: TabChannel) -> bool {
        if self.channels.iter().any(|x| x.id == c.id) {
            return false;
        }
        self.skip.retain(|x| x.id != c.id);
        self.channels.push(c);
        true
    }

    pub fn remove(&mut self, id: &str) {
        self.channels.retain(|c| c.id != id);
        self.first.retain(|c| c.id != id);
    }
}

/// W3
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MineSource {
    /// 1.31
    #[default]
    List,
    /// Every channel the account follows, oldest follow first, minus `skip`.
    Follows,
}

/// What takes a watch slot before the rest (the reference script's priorities).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "MinerRule")]
pub enum Rule {
    /// A watch streak not earned yet this stream.
    Streak,
    /// The channel has drops running.
    Drops,
    /// More points per minute, e.g. as a sub.
    Multiplier,
    /// Fewest points first.
    FewestPoints,
    /// Most points first.
    MostPoints,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ChatPresence {
    #[default]
    Never,
    /// While the channel is live.
    Live,
    Always,
}

/// Betting on predictions with points. Off by default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Bets {
    pub enabled: bool,
    pub strategy: Strategy,
    /// `Strategy::Outcome`: which outcome, from 1.
    pub outcome: u8,
    /// Share of the balance to bet, in percent.
    pub percent: u8,
    /// `Strategy::Smart`: below this gap between the two sides' voters (percent), bet on
    /// the underdog; above it, with the most voters.
    pub gap: u8,
    #[ts(type = "number")]
    pub max_points: u64,
    /// No bets while the balance is at or under this.
    #[ts(type = "number")]
    pub keep: u64,
    /// Never bet more than the biggest bet on that side (minus a few points).
    pub stealth: bool,
    pub delay_mode: DelayMode,
    /// Seconds, or percent of the window for `DelayMode::Percent`.
    pub delay: u32,
    pub filter: BetFilter,
}

impl Default for Bets {
    fn default() -> Self {
        Self {
            enabled: false,
            strategy: Strategy::Smart,
            outcome: 1,
            percent: 5,
            gap: 20,
            max_points: 50_000,
            keep: 0,
            stealth: false,
            delay_mode: DelayMode::BeforeEnd,
            delay: 6,
            filter: BetFilter::default(),
        }
    }
}

impl Bets {
    fn sanitize(&mut self) {
        self.outcome = self.outcome.clamp(1, 10);
        self.percent = self.percent.clamp(1, 100);
        self.gap = self.gap.min(100);
        self.max_points = self.max_points.max(10);
        self.delay = match self.delay_mode {
            DelayMode::Percent => self.delay.min(100),
            _ => self.delay.min(1800),
        };
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Strategy {
    /// Most voters, unless the two sides are close: then the underdog.
    #[default]
    Smart,
    MostVoted,
    /// The side with the best payout.
    HighOdds,
    /// The side with the most points on it.
    MostPoints,
    /// The side the biggest single bet is on.
    SmartMoney,
    /// Always the same outcome.
    Outcome,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DelayMode {
    /// `delay` seconds after the prediction opens.
    AfterStart,
    /// `delay` seconds before it closes.
    #[default]
    BeforeEnd,
    /// At `delay` percent of the window.
    Percent,
}

/// Bet only when this holds.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct BetFilter {
    pub by: FilterBy,
    pub cmp: FilterCmp,
    pub value: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FilterBy {
    #[default]
    None,
    /// Voters on both sides.
    TotalUsers,
    /// Points on both sides.
    TotalPoints,
    /// Voters on the chosen side.
    ChosenUsers,
    /// Points on the chosen side.
    ChosenPoints,
    /// The chosen side's payout, e.g. 2.5.
    Odds,
    /// Biggest single bet on the chosen side.
    TopPoints,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FilterCmp {
    #[default]
    Above,
    AtLeast,
    Below,
    AtMost,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(id: &str) -> TabChannel {
        TabChannel {
            id: id.into(),
            login: format!("c{id}"),
            name: format!("C{id}"),
        }
    }

    #[test]
    fn first_keeps_two_and_pushes_the_oldest_out() {
        let mut m = Miner::default();
        assert_eq!(m.put_first(ch("1")), None);
        assert_eq!(m.put_first(ch("2")), None);
        assert_eq!(m.put_first(ch("2")), None);
        assert_eq!(m.put_first(ch("3")).map(|c| c.id), Some("1".into()));
        assert_eq!(
            m.first.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            ["2", "3"]
        );
    }

    #[test]
    fn sanitize_dedupes_and_clamps() {
        let mut m = Miner {
            channels: vec![ch("1"), ch("1"), TabChannel::default(), ch("2")],
            first: vec![ch("1"), ch("2"), ch("3")],
            order: vec![Rule::Streak, Rule::Streak, Rule::Drops],
            account: Some(String::new()),
            ..Miner::default()
        };
        m.bets.percent = 0;
        m.bets.delay_mode = DelayMode::Percent;
        m.bets.delay = 400;
        m.sanitize();
        assert_eq!(m.channels.len(), 2);
        assert_eq!(m.first.len(), 2);
        assert_eq!(m.order, [Rule::Streak, Rule::Drops]);
        assert_eq!((m.account, m.bets.percent, m.bets.delay), (None, 1, 100));
    }

    #[test]
    fn adding_takes_it_off_the_skip_list() {
        let mut m = Miner {
            skip: vec![ch("1")],
            ..Miner::default()
        };
        assert!(m.add(ch("1")));
        assert!(!m.add(ch("1")));
        assert!(m.skip.is_empty());
        m.put_first(ch("1"));
        m.remove("1");
        assert!(m.channels.is_empty() && m.first.is_empty());
    }

    #[test]
    fn old_files_get_defaults() {
        let m: Miner = serde_json::from_str("{}").unwrap();
        assert!(m.enabled && m.watching_earns && !m.bets.enabled && !m.goals);
        assert_eq!(m.order, [Rule::Streak, Rule::Drops]);
    }
}
