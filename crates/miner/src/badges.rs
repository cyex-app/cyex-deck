//! V110-U68 Badge drops: drop campaigns that give a chat badge for watching. Which badges
//! can be earned, how far along they are, which live channels can earn one, and which
//! helper channels (found by the miner, never saved to the list) are still needed. No I/O
//! here; `app/src/miner` loads the campaigns and acts on this. Times are unix seconds.

use std::collections::HashSet;

use cyex_settings::schema::Miner;
use cyex_twitch::gql::drops::{Allowed, Campaign, Inventory};
use serde::Serialize;
use ts_rs::TS;

/// Helper channels kept per campaign.
pub const HELPERS_PER_CAMPAIGN: usize = 5;
/// Helper channels added per campaign in one session, however many leave.
pub const ADDS_PER_CAMPAIGN: u32 = 10;
/// How long to wait before looking for helpers for a campaign again.
pub const RETRY_SECS: i64 = 30 * 60;

/// One badge, earned by watching alone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Badge {
    pub campaign_id: String,
    pub drop_id: String,
    pub benefit_id: String,
    pub name: String,
    pub image: String,
    pub game_id: String,
    pub game: String,
    pub game_slug: String,
    /// Only these channels count; empty means any channel playing the game.
    pub allowed: Vec<Allowed>,
    pub minutes_required: u32,
    pub minutes_watched: u32,
    /// When the badge can no longer be earned; 0 if unknown.
    pub ends_at: i64,
    pub earned: bool,
    /// Seen not earned in the inventory, so earning it now happened in this session.
    pub seen_unearned: bool,
    /// Found in the inventory but not in the campaign list; kept until it ends.
    pub from_inventory: bool,
}

impl Badge {
    pub fn minutes_left(&self) -> u32 {
        self.minutes_required.saturating_sub(self.minutes_watched)
    }

    /// Still earnable at `now`.
    pub fn open(&self, now: i64) -> bool {
        !self.earned && (self.ends_at == 0 || self.ends_at > now)
    }
}

/// Every badge of a campaign that is earned by watching alone (no subs needed), fewest
/// minutes first. Campaigns can have several tiers, e.g. for 30, 60 and 90 minutes.
pub fn watch_badges(c: &Campaign) -> Vec<Badge> {
    let mut out: Vec<Badge> = c
        .drops
        .iter()
        .filter(|d| d.badge && d.required_subs == 0 && d.required_minutes > 0)
        .map(|d| Badge {
            campaign_id: c.id.clone(),
            drop_id: d.id.clone(),
            benefit_id: d.benefit_id.clone().unwrap_or_default(),
            name: d.name.clone(),
            image: d.image.clone().unwrap_or_default(),
            game_id: c.game_id.clone().unwrap_or_default(),
            game: c.game.clone().unwrap_or_default(),
            game_slug: c.game_slug.clone().unwrap_or_default(),
            allowed: c.allowed.clone(),
            minutes_required: d.required_minutes,
            minutes_watched: d.watched_minutes,
            ends_at: earliest_end(d.ends_at, c.ends_at),
            earned: d.claimed,
            seen_unearned: false,
            from_inventory: false,
        })
        .collect();
    out.sort_by_key(|b| b.minutes_required);
    out
}

fn earliest_end(a: i64, b: i64) -> i64 {
    match (a, b) {
        (0, x) | (x, 0) => x,
        (a, b) => a.min(b),
    }
}

/// The badge of a campaign to work on: the quickest one not earned yet, else the longest.
pub fn current<'a>(badges: &'a [Badge], campaign_id: &str) -> Option<&'a Badge> {
    let of_campaign = || badges.iter().filter(|b| b.campaign_id == campaign_id);
    of_campaign()
        .filter(|b| !b.earned)
        .min_by_key(|b| b.minutes_required)
        .or_else(|| of_campaign().max_by_key(|b| b.minutes_required))
}

/// Campaigns with a badge left to earn at `now`.
pub fn open_campaigns(badges: &[Badge], now: i64) -> HashSet<String> {
    badges
        .iter()
        .filter(|b| b.open(now))
        .map(|b| b.campaign_id.clone())
        .collect()
}

/// The badges to work on now, one per open campaign (its `current`), that the settings
/// want.
pub fn wanted<'a>(badges: &'a [Badge], miner: &Miner, now: i64) -> Vec<&'a Badge> {
    let mut seen = HashSet::new();
    badges
        .iter()
        .filter(|b| b.open(now) && miner.badges.wants(&b.campaign_id) && seen.insert(&b.campaign_id))
        .filter_map(|b| current(badges, &b.campaign_id))
        .filter(|b| b.open(now))
        .collect()
}

/// A freshly loaded campaign list becomes the badge list, keeping what's known about
/// progress (by drop). Badges only the inventory knows stay until they end.
pub fn merge(old: &[Badge], campaigns: &[Campaign], now: i64) -> Vec<Badge> {
    let mut fresh: Vec<Badge> = campaigns
        .iter()
        .flat_map(watch_badges)
        .map(|mut b| {
            if let Some(o) = old.iter().find(|o| o.drop_id == b.drop_id) {
                b.minutes_watched = b.minutes_watched.max(o.minutes_watched);
                b.earned |= o.earned;
                b.seen_unearned = o.seen_unearned;
            }
            b
        })
        .collect();
    let kept: Vec<Badge> = old
        .iter()
        .filter(|o| o.from_inventory && o.ends_at > now && !fresh.iter().any(|f| f.drop_id == o.drop_id))
        .cloned()
        .collect();
    fresh.extend(kept);
    fresh
}

/// Adds badge campaigns the inventory has in progress but the campaign list lacks.
/// Returns the names added.
pub fn add_from_inventory(badges: &mut Vec<Badge>, inv: &Inventory, now: i64) -> Vec<String> {
    let mut added = Vec::new();
    for b in inv.campaigns.iter().flat_map(watch_badges) {
        if (b.ends_at != 0 && b.ends_at <= now) || badges.iter().any(|o| o.drop_id == b.drop_id) {
            continue;
        }
        added.push(b.name.clone());
        badges.push(Badge {
            from_inventory: true,
            ..b
        });
    }
    added
}

/// Applies the inventory's progress and earned rewards. Returns the badges that were seen
/// unearned before and are earned now (so earned in this session); `channel` names the
/// channel (id, login) that was being watched for a campaign.
pub fn apply_inventory(
    badges: &mut [Badge],
    inv: &Inventory,
    now: i64,
    channel: impl Fn(&str) -> Option<(String, String)>,
) -> Vec<EarnedBadge> {
    let mut newly = Vec::new();
    for b in badges.iter_mut() {
        let drop = inv
            .campaigns
            .iter()
            .filter(|c| c.id == b.campaign_id)
            .flat_map(|c| &c.drops)
            .find(|d| d.id == b.drop_id);
        let mut done = !b.benefit_id.is_empty() && inv.earned.contains(&b.benefit_id);
        if let Some(d) = drop {
            b.minutes_watched = d.watched_minutes;
            done |= d.claimed;
        }
        if done && !b.earned && b.seen_unearned {
            let ch = channel(&b.campaign_id);
            newly.push(EarnedBadge {
                campaign_id: b.campaign_id.clone(),
                name: b.name.clone(),
                image: b.image.clone(),
                game: b.game.clone(),
                minutes_required: b.minutes_required,
                earned_at: now,
                channel_id: ch.as_ref().map(|c| c.0.clone()),
                channel: ch.map(|c| c.1),
            });
        }
        b.earned |= done;
        b.seen_unearned |= !done;
    }
    newly
}

/// A live stream, as far as badges care.
#[derive(Clone, Copy, Debug)]
pub struct StreamInfo<'a> {
    pub channel_id: &'a str,
    pub login: &'a str,
    pub live: bool,
    pub game_id: Option<&'a str>,
    /// Campaign ids running on the channel (`channel_campaigns`).
    pub campaigns: &'a [String],
}

/// Whether watching this stream earns the badge: it's live, plays the campaign's game, has
/// the campaign running, and is on the allow list when there is one.
pub fn is_target(s: &StreamInfo<'_>, b: &Badge) -> bool {
    s.live
        && (b.game_id.is_empty() || s.game_id == Some(b.game_id.as_str()))
        && s.campaigns.contains(&b.campaign_id)
        && (b.allowed.is_empty()
            || b.allowed
                .iter()
                .any(|a| a.id == s.channel_id || a.login.eq_ignore_ascii_case(s.login)))
}

/// Of the badges worked on (`wanted`), the one this stream earns that is closest to done.
pub fn target_badge<'a>(s: &StreamInfo<'_>, wanted: &[&'a Badge]) -> Option<&'a Badge> {
    wanted
        .iter()
        .copied()
        .filter(|b| is_target(s, b))
        .min_by_key(|b| b.minutes_left())
}

/// Ids of the streams that earn one of the `wanted` badges, in stream order.
pub fn targets(streams: &[StreamInfo<'_>], wanted: &[&Badge]) -> Vec<String> {
    streams
        .iter()
        .filter(|s| target_badge(s, wanted).is_some())
        .map(|s| s.channel_id.to_owned())
        .collect()
}

/// A channel the miner found for a badge. Kept in memory only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Helper {
    pub channel_id: String,
    pub login: String,
    pub campaign_id: String,
    pub added_at: i64,
}

/// Helpers to let go: their campaign's badge is earned, over, gone, or not wanted anymore.
pub fn helpers_to_drop(helpers: &[Helper], badges: &[Badge], miner: &Miner, now: i64) -> Vec<String> {
    let open = open_campaigns(badges, now);
    helpers
        .iter()
        .filter(|h| !open.contains(&h.campaign_id) || !miner.badges.wants(&h.campaign_id))
        .map(|h| h.channel_id.clone())
        .collect()
}

/// How many more helpers to look for on a campaign: room under the cap, and adds left in
/// this session. 0 when the finder is switched off.
pub fn helpers_wanted(helpers: &[Helper], campaign_id: &str, adds_so_far: u32, miner: &Miner) -> usize {
    if !miner.badges.find_channels {
        return 0;
    }
    let have = helpers.iter().filter(|h| h.campaign_id == campaign_id).count();
    let room = HELPERS_PER_CAMPAIGN.saturating_sub(have);
    let adds = ADDS_PER_CAMPAIGN.saturating_sub(adds_so_far) as usize;
    room.min(adds)
}

/// Time to look for helpers again: never tried (`None`), or the last try is old enough.
pub fn retry_due(tried_at: Option<i64>, now: i64) -> bool {
    tried_at.is_none_or(|t| now - t >= RETRY_SECS)
}

/// A badge as the Miner view shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BadgeView {
    pub campaign_id: String,
    pub name: String,
    pub image: String,
    pub game: String,
    pub minutes_required: u32,
    pub minutes_watched: u32,
    /// Unix seconds; 0 if unknown.
    #[ts(type = "number")]
    pub ends_at: i64,
    /// Wanted: picked, or all with auto select.
    pub picked: bool,
    /// Channels the campaign is limited to; 0 means any channel playing the game.
    pub allowed_channels: u32,
    /// Helper channels found for it right now.
    pub helper_channels: u32,
}

/// A badge earned while the app ran.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EarnedBadge {
    pub campaign_id: String,
    pub name: String,
    pub image: String,
    pub game: String,
    pub minutes_required: u32,
    /// Unix seconds.
    #[ts(type = "number")]
    pub earned_at: i64,
    /// The channel watched for it, if known.
    pub channel_id: Option<String>,
    pub channel: Option<String>,
}

/// The badges still to earn, one per open campaign, for the view.
pub fn views(badges: &[Badge], helpers: &[Helper], miner: &Miner, now: i64) -> Vec<BadgeView> {
    let mut seen = HashSet::new();
    badges
        .iter()
        .filter(|b| b.open(now) && seen.insert(&b.campaign_id))
        .filter_map(|b| current(badges, &b.campaign_id).filter(|c| c.open(now)))
        .map(|b| BadgeView {
            campaign_id: b.campaign_id.clone(),
            name: b.name.clone(),
            image: b.image.clone(),
            game: b.game.clone(),
            minutes_required: b.minutes_required,
            minutes_watched: b.minutes_watched,
            ends_at: b.ends_at,
            picked: miner.badges.wants(&b.campaign_id),
            allowed_channels: b.allowed.len() as u32,
            helper_channels: helpers.iter().filter(|h| h.campaign_id == b.campaign_id).count() as u32,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_twitch::gql::drops::{Drop, Inventory};

    const NOW: i64 = 1_000_000;

    fn drop(id: &str, minutes: u32, badge: bool, subs: u32) -> Drop {
        Drop {
            id: id.into(),
            name: format!("name {id}"),
            image: Some(format!("{id}.png")),
            benefit_id: Some(format!("b-{id}")),
            badge,
            required_subs: subs,
            starts_at: 0,
            ends_at: 0,
            required_minutes: minutes,
            watched_minutes: 0,
            instance_id: None,
            claimed: false,
        }
    }

    fn campaign(id: &str, drops: Vec<Drop>) -> Campaign {
        Campaign {
            id: id.into(),
            name: format!("camp {id}"),
            game: Some("Chess".into()),
            game_id: Some("7".into()),
            game_slug: Some("chess".into()),
            allowed: Vec::new(),
            starts_at: 0,
            ends_at: NOW + 10_000,
            drops,
        }
    }

    fn badges_of(c: &Campaign) -> Vec<Badge> {
        watch_badges(c)
    }

    #[test]
    fn only_badges_for_watching_alone_count() {
        let c = campaign(
            "c",
            vec![
                drop("90", 90, true, 0),
                drop("sub", 30, true, 1),
                drop("item", 30, false, 0),
                drop("zero", 0, true, 0),
                drop("30", 30, true, 0),
            ],
        );
        let b = badges_of(&c);
        assert_eq!(
            b.iter().map(|b| b.drop_id.as_str()).collect::<Vec<_>>(),
            ["30", "90"]
        );
        assert_eq!(
            (b[0].benefit_id.as_str(), b[0].game_id.as_str(), b[0].ends_at),
            ("b-30", "7", NOW + 10_000)
        );
    }

    #[test]
    fn the_earlier_end_wins() {
        let mut d = drop("d", 30, true, 0);
        d.ends_at = NOW + 5;
        assert_eq!(badges_of(&campaign("c", vec![d]))[0].ends_at, NOW + 5);
        let mut c = campaign("c", vec![drop("d", 30, true, 0)]);
        c.ends_at = 0;
        assert_eq!(badges_of(&c)[0].ends_at, 0);
        assert!(badges_of(&c)[0].open(NOW));
    }

    #[test]
    fn current_is_the_next_tier_then_the_last() {
        let c = campaign(
            "c",
            vec![
                drop("30", 30, true, 0),
                drop("60", 60, true, 0),
                drop("90", 90, true, 0),
            ],
        );
        let mut b = badges_of(&c);
        assert_eq!(current(&b, "c").unwrap().drop_id, "30");
        b[0].earned = true;
        assert_eq!(current(&b, "c").unwrap().drop_id, "60");
        for x in &mut b {
            x.earned = true;
        }
        assert_eq!(current(&b, "c").unwrap().drop_id, "90");
        assert!(current(&b, "other").is_none());
        assert!(open_campaigns(&b, NOW).is_empty());
        b[2].earned = false;
        assert_eq!(open_campaigns(&b, NOW).len(), 1);
        // Over is not open.
        assert!(open_campaigns(&b, NOW + 20_000).is_empty());
    }

    #[test]
    fn wanted_follows_the_settings() {
        let b: Vec<Badge> = ["a", "b"]
            .iter()
            .flat_map(|id| {
                badges_of(&campaign(
                    id,
                    vec![
                        drop(&format!("{id}1"), 30, true, 0),
                        drop(&format!("{id}2"), 60, true, 0),
                    ],
                ))
            })
            .collect();
        let mut m = Miner::default();
        let names = |w: Vec<&Badge>| w.iter().map(|b| b.drop_id.clone()).collect::<Vec<_>>();
        assert_eq!(names(wanted(&b, &m, NOW)), ["a1", "b1"]);
        m.badges.auto = false;
        assert!(wanted(&b, &m, NOW).is_empty());
        m.badges.picked = vec!["b".into()];
        assert_eq!(names(wanted(&b, &m, NOW)), ["b1"]);
    }

    #[test]
    fn merge_keeps_progress_and_inventory_only_badges() {
        let c = campaign("c", vec![drop("30", 30, true, 0)]);
        let mut old = badges_of(&c);
        old[0].minutes_watched = 12;
        old[0].seen_unearned = true;
        let mut inv_only = badges_of(&campaign("x", vec![drop("x1", 30, true, 0)]));
        inv_only[0].from_inventory = true;
        old.extend(inv_only.clone());
        let merged = merge(&old, std::slice::from_ref(&c), NOW);
        assert_eq!(merged.len(), 2);
        assert_eq!((merged[0].minutes_watched, merged[0].seen_unearned), (12, true));
        assert!(merged[1].from_inventory);
        // The inventory-only one goes once it ends.
        assert_eq!(merge(&old, &[c], NOW + 20_000).len(), 1);
    }

    #[test]
    fn inventory_adds_missing_badges_once() {
        let mut badges = Vec::new();
        let inv = Inventory {
            campaigns: vec![campaign("c", vec![drop("30", 30, true, 0)])],
            earned: Vec::new(),
        };
        assert_eq!(add_from_inventory(&mut badges, &inv, NOW), ["name 30"]);
        assert!(badges[0].from_inventory);
        assert!(add_from_inventory(&mut badges, &inv, NOW).is_empty());
        let mut over = inv.clone();
        over.campaigns[0].ends_at = NOW - 1;
        let mut none = Vec::new();
        assert!(add_from_inventory(&mut none, &over, NOW).is_empty());
    }

    #[test]
    fn earning_is_noticed_only_after_it_was_seen_unearned() {
        let c = campaign("c", vec![drop("30", 30, true, 0), drop("60", 60, true, 0)]);
        let mut b = badges_of(&c);
        let who = |id: &str| (id == "c").then(|| ("11".to_owned(), "alpha".to_owned()));
        // Progress arrives; the first look marks both as seen unearned.
        let mut p = campaign("c", vec![drop("30", 30, true, 0), drop("60", 60, true, 0)]);
        p.drops[0].watched_minutes = 29;
        let inv = Inventory {
            campaigns: vec![p],
            earned: Vec::new(),
        };
        assert!(apply_inventory(&mut b, &inv, NOW, who).is_empty());
        assert_eq!(
            (b[0].minutes_watched, b[0].seen_unearned, b[1].seen_unearned),
            (29, true, true)
        );
        // The badge shows up under gameEventDrops.
        let inv = Inventory {
            campaigns: Vec::new(),
            earned: vec!["b-30".into()],
        };
        let e = apply_inventory(&mut b, &inv, NOW + 60, who);
        assert_eq!(e.len(), 1);
        assert_eq!(
            (
                e[0].name.as_str(),
                e[0].channel.as_deref(),
                e[0].channel_id.as_deref(),
                e[0].earned_at
            ),
            ("name 30", Some("alpha"), Some("11"), NOW + 60)
        );
        assert!(b[0].earned && !b[1].earned);
        // Not announced twice.
        assert!(apply_inventory(&mut b, &inv, NOW + 120, who).is_empty());
    }

    #[test]
    fn a_badge_earned_before_the_session_is_not_announced() {
        let mut b = badges_of(&campaign("c", vec![drop("30", 30, true, 0)]));
        let inv = Inventory {
            campaigns: Vec::new(),
            earned: vec!["b-30".into()],
        };
        assert!(apply_inventory(&mut b, &inv, NOW, |_| None).is_empty());
        assert!(b[0].earned);
    }

    fn stream<'a>(id: &'a str, login: &'a str, game: Option<&'a str>, camps: &'a [String]) -> StreamInfo<'a> {
        StreamInfo {
            channel_id: id,
            login,
            live: true,
            game_id: game,
            campaigns: camps,
        }
    }

    #[test]
    fn targets_need_game_campaign_and_allow_list() {
        let camps = vec!["c".to_owned()];
        let none: Vec<String> = Vec::new();
        let mut c = campaign("c", vec![drop("30", 30, true, 0)]);
        let any = badges_of(&c);
        assert!(is_target(&stream("1", "a", Some("7"), &camps), &any[0]));
        assert!(!is_target(&stream("1", "a", Some("8"), &camps), &any[0]));
        assert!(!is_target(&stream("1", "a", None, &camps), &any[0]));
        assert!(!is_target(&stream("1", "a", Some("7"), &none), &any[0]));
        let mut off = stream("1", "a", Some("7"), &camps);
        off.live = false;
        assert!(!is_target(&off, &any[0]));
        c.allowed = vec![Allowed {
            id: "2".into(),
            login: "beta".into(),
        }];
        let listed = badges_of(&c);
        assert!(!is_target(&stream("1", "a", Some("7"), &camps), &listed[0]));
        assert!(is_target(&stream("2", "x", Some("7"), &camps), &listed[0]));
        assert!(is_target(&stream("9", "Beta", Some("7"), &camps), &listed[0]));
        // A campaign without a game id (not known) matches any game.
        c.game_id = None;
        assert!(is_target(
            &stream("2", "x", Some("99"), &camps),
            &badges_of(&c)[0]
        ));
    }

    #[test]
    fn targets_pick_streams_and_the_closest_badge() {
        let c1 = vec!["c1".to_owned()];
        let c12 = vec!["c1".to_owned(), "c2".to_owned()];
        let mut b1 = badges_of(&campaign("c1", vec![drop("a", 60, true, 0)]));
        let b2 = badges_of(&campaign("c2", vec![drop("b", 60, true, 0)]));
        b1[0].minutes_watched = 10;
        let all: Vec<Badge> = b1.into_iter().chain(b2).collect();
        let wanted_refs: Vec<&Badge> = all.iter().collect();
        let streams = [
            stream("1", "a", Some("7"), &c1),
            stream("2", "b", Some("7"), &c12),
            stream("3", "c", Some("5"), &c12),
        ];
        assert_eq!(targets(&streams, &wanted_refs), ["1", "2"]);
        // Stream 2 earns both; c1's is closer to done.
        assert_eq!(target_badge(&streams[1], &wanted_refs).unwrap().campaign_id, "c1");
        assert!(targets(&streams, &[]).is_empty());
    }

    fn helper(id: &str, campaign: &str) -> Helper {
        Helper {
            channel_id: id.into(),
            login: format!("l{id}"),
            campaign_id: campaign.into(),
            added_at: NOW,
        }
    }

    #[test]
    fn helpers_go_when_the_badge_is_earned_over_or_unwanted() {
        let mut b = badges_of(&campaign("c", vec![drop("30", 30, true, 0)]));
        let mut b2 = badges_of(&campaign("d", vec![drop("d1", 30, true, 0)]));
        b.append(&mut b2);
        let helpers = [helper("1", "c"), helper("2", "d"), helper("3", "gone")];
        let mut m = Miner::default();
        assert_eq!(helpers_to_drop(&helpers, &b, &m, NOW), ["3"]);
        b[0].earned = true;
        assert_eq!(helpers_to_drop(&helpers, &b, &m, NOW), ["1", "3"]);
        assert_eq!(helpers_to_drop(&helpers, &b, &m, NOW + 20_000), ["1", "2", "3"]);
        b[0].earned = false;
        m.badges.auto = false;
        m.badges.picked = vec!["c".into()];
        assert_eq!(helpers_to_drop(&helpers, &b, &m, NOW), ["2", "3"]);
    }

    #[test]
    fn helper_room_and_retry() {
        let m = Miner::default();
        let hs: Vec<Helper> = (0..3).map(|i| helper(&i.to_string(), "c")).collect();
        assert_eq!(helpers_wanted(&hs, "c", 3, &m), 2);
        assert_eq!(helpers_wanted(&hs, "other", 3, &m), 5);
        assert_eq!(helpers_wanted(&hs, "other", 8, &m), 2);
        assert_eq!(helpers_wanted(&hs, "other", 10, &m), 0);
        assert_eq!(helpers_wanted(&[], "c", 0, &m), 5);
        let mut off = Miner::default();
        off.badges.find_channels = false;
        assert_eq!(helpers_wanted(&hs, "other", 0, &off), 0);
        assert!(retry_due(None, NOW));
        assert!(!retry_due(Some(NOW), NOW + RETRY_SECS - 1));
        assert!(retry_due(Some(NOW), NOW + RETRY_SECS));
    }

    #[test]
    fn views_show_one_open_badge_per_campaign() {
        let mut b = badges_of(&campaign(
            "c",
            vec![drop("30", 30, true, 0), drop("60", 60, true, 0)],
        ));
        b.extend(badges_of(&campaign("d", vec![drop("d1", 30, true, 0)])));
        b[0].earned = true;
        b[2].earned = true;
        let hs = [helper("1", "c"), helper("2", "c")];
        let mut m = Miner::default();
        m.badges.auto = false;
        m.badges.picked = vec!["c".into()];
        let v = views(&b, &hs, &m, NOW);
        assert_eq!(v.len(), 1);
        assert_eq!(
            (
                v[0].name.as_str(),
                v[0].minutes_required,
                v[0].picked,
                v[0].helper_channels
            ),
            ("name 60", 60, true, 2)
        );
        assert_eq!(v[0].allowed_channels, 0);
    }
}
