//! Betting on predictions with points (off by default), the reference script's
//! strategies: which side, how much, when, and whether to bet at all.

use cyex_settings::schema::{BetFilter, Bets, DelayMode, FilterBy, FilterCmp, Strategy};

/// Bets under this many points aren't placed.
const MIN_BET: u64 = 10;

/// One outcome as Twitch reports it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Side {
    pub users: u64,
    pub points: u64,
    /// The biggest single bet on it.
    pub top: u64,
}

fn total_points(sides: &[Side]) -> u64 {
    sides.iter().map(|s| s.points).sum()
}

/// Payout per point: all points over this side's.
fn odds(sides: &[Side], i: usize) -> f64 {
    match sides[i].points {
        0 => 0.0,
        p => total_points(sides) as f64 / p as f64,
    }
}

fn argmax(sides: &[Side], key: impl Fn(usize) -> f64) -> Option<usize> {
    (0..sides.len()).max_by(|&a, &b| key(a).total_cmp(&key(b)).then(b.cmp(&a)))
}

/// The side to bet on, by index.
pub fn choose(b: &Bets, sides: &[Side]) -> Option<usize> {
    if sides.is_empty() {
        return None;
    }
    let most_voted = || argmax(sides, |i| sides[i].users as f64);
    let high_odds = || argmax(sides, |i| odds(sides, i));
    match b.strategy {
        Strategy::MostVoted => most_voted(),
        Strategy::HighOdds => high_odds(),
        Strategy::MostPoints => argmax(sides, |i| sides[i].points as f64),
        Strategy::SmartMoney => argmax(sides, |i| sides[i].top as f64),
        Strategy::Outcome => Some(usize::from(b.outcome.saturating_sub(1)).min(sides.len() - 1)),
        Strategy::Smart => {
            let users: u64 = sides.iter().map(|s| s.users).sum();
            if sides.len() < 2 || users == 0 {
                return most_voted();
            }
            let share = |i: usize| 100.0 * sides[i].users as f64 / users as f64;
            if (share(0) - share(1)).abs() < f64::from(b.gap) {
                high_odds()
            } else {
                most_voted()
            }
        }
    }
}

/// How much to bet; 0 means don't. `jitter` (1-5) keeps stealth bets from looking
/// exact.
pub fn amount(b: &Bets, balance: u64, side: Side, jitter: u64) -> u64 {
    if balance <= b.keep {
        return 0;
    }
    let mut a = (balance * u64::from(b.percent) / 100).min(b.max_points);
    if b.stealth && side.top > 0 && a >= side.top {
        a = side.top.saturating_sub(jitter.clamp(1, 5));
    }
    if a < MIN_BET { 0 } else { a }
}

/// Seconds from now until the bet goes in. `elapsed` is how long the prediction has
/// been open.
pub fn wait_secs(b: &Bets, window_secs: u32, elapsed: f64) -> f64 {
    let w = f64::from(window_secs);
    let d = f64::from(b.delay);
    let at = match b.delay_mode {
        DelayMode::AfterStart => d.min(w),
        DelayMode::BeforeEnd => (w - d).max(0.0),
        DelayMode::Percent => w * d / 100.0,
    };
    (at - elapsed).max(0.0)
}

/// Whether the filter lets this bet through.
pub fn passes(f: &BetFilter, sides: &[Side], chosen: usize) -> bool {
    let Some(side) = sides.get(chosen) else {
        return false;
    };
    let value = match f.by {
        FilterBy::None => return true,
        FilterBy::TotalUsers => sides.iter().map(|s| s.users).sum::<u64>() as f64,
        FilterBy::TotalPoints => total_points(sides) as f64,
        FilterBy::ChosenUsers => side.users as f64,
        FilterBy::ChosenPoints => side.points as f64,
        FilterBy::Odds => odds(sides, chosen),
        FilterBy::TopPoints => side.top as f64,
    };
    match f.cmp {
        FilterCmp::Above => value > f.value,
        FilterCmp::AtLeast => value >= f.value,
        FilterCmp::Below => value < f.value,
        FilterCmp::AtMost => value <= f.value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(users: u64, points: u64, top: u64) -> Side {
        Side { users, points, top }
    }

    fn with(strategy: Strategy) -> Bets {
        Bets {
            strategy,
            ..Bets::default()
        }
    }

    #[test]
    fn strategies_pick_their_side() {
        let sides = [side(70, 1000, 500), side(30, 4000, 100)];
        assert_eq!(choose(&with(Strategy::MostVoted), &sides), Some(0));
        assert_eq!(choose(&with(Strategy::HighOdds), &sides), Some(0));
        assert_eq!(choose(&with(Strategy::MostPoints), &sides), Some(1));
        assert_eq!(choose(&with(Strategy::SmartMoney), &sides), Some(0));
        let third = Bets {
            strategy: Strategy::Outcome,
            outcome: 9,
            ..Bets::default()
        };
        assert_eq!(choose(&third, &sides), Some(1));
        assert_eq!(choose(&with(Strategy::Smart), &[]), None);
    }

    #[test]
    fn smart_takes_the_underdog_when_close() {
        // 55/45 is inside the 20 % gap: best payout wins.
        let close = [side(55, 5000, 0), side(45, 1000, 0)];
        assert_eq!(choose(&with(Strategy::Smart), &close), Some(1));
        let clear = [side(80, 5000, 0), side(20, 1000, 0)];
        assert_eq!(choose(&with(Strategy::Smart), &clear), Some(0));
    }

    #[test]
    fn amounts() {
        let b = Bets::default();
        assert_eq!(amount(&b, 10_000, side(1, 1, 0), 3), 500);
        assert_eq!(amount(&b, 100, side(1, 1, 0), 3), 0);
        let capped = Bets {
            max_points: 200,
            ..Bets::default()
        };
        assert_eq!(amount(&capped, 10_000, side(1, 1, 0), 3), 200);
        let stealth = Bets {
            stealth: true,
            ..Bets::default()
        };
        assert_eq!(amount(&stealth, 10_000, side(1, 1, 400), 3), 397);
        let keep = Bets {
            keep: 10_000,
            ..Bets::default()
        };
        assert_eq!(amount(&keep, 10_000, side(1, 1, 0), 3), 0);
    }

    #[test]
    fn delays() {
        let mut b = Bets::default();
        assert_eq!(wait_secs(&b, 120, 10.0), 104.0);
        assert_eq!(wait_secs(&b, 120, 119.0), 0.0);
        b.delay_mode = DelayMode::AfterStart;
        assert_eq!(wait_secs(&b, 120, 2.0), 4.0);
        b.delay_mode = DelayMode::Percent;
        b.delay = 50;
        assert_eq!(wait_secs(&b, 120, 0.0), 60.0);
    }

    #[test]
    fn filters() {
        let sides = [side(10, 1000, 300), side(5, 3000, 900)];
        let f = |by, cmp, value| BetFilter { by, cmp, value };
        assert!(passes(&BetFilter::default(), &sides, 0));
        assert!(passes(
            &f(FilterBy::TotalUsers, FilterCmp::AtLeast, 15.0),
            &sides,
            0
        ));
        assert!(!passes(
            &f(FilterBy::TotalUsers, FilterCmp::Above, 15.0),
            &sides,
            0
        ));
        assert!(passes(&f(FilterBy::Odds, FilterCmp::Above, 3.9), &sides, 0));
        assert!(passes(
            &f(FilterBy::TopPoints, FilterCmp::Below, 1000.0),
            &sides,
            1
        ));
        assert!(!passes(
            &f(FilterBy::ChosenPoints, FilterCmp::AtMost, 10.0),
            &sides,
            5
        ));
    }
}
