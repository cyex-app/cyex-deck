//! Miner stats (1.32): an append-only log per channel, one JSON line per event, summed
//! up when the stats menu asks. No database.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The balance chart keeps at most this many points.
const SERIES_MAX: usize = 240;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "StatKind")]
pub enum Kind {
    /// Points for watching.
    Watch,
    /// Bonus chests (C22).
    Chest,
    Streak,
    Raid,
    Moment,
    /// Points put on a prediction (negative).
    Bet,
    Won,
    Lost,
    Refund,
    Drop,
    /// W2 Points put toward a goal (negative).
    Goal,
    /// Minutes watched (`minutes`), no points.
    Watched,
    Other,
}

impl Kind {
    /// From a `points-earned` reason code.
    pub fn from_reason(code: &str) -> Self {
        match code {
            "WATCH" => Kind::Watch,
            "CLAIM" => Kind::Chest,
            "WATCH_STREAK" => Kind::Streak,
            "RAID" => Kind::Raid,
            "MOMENT" | "COMMUNITY_MOMENT" => Kind::Moment,
            _ => Kind::Other,
        }
    }
}

/// One line of a channel's log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// Unix ms.
    pub t: i64,
    pub k: Kind,
    /// Points gained (or spent, negative).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub p: i64,
    /// The balance after it, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b: Option<u64>,
    /// Minutes watched (`Kind::Watched`).
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub m: u32,
    /// A drop's name, a bet's title, an unknown reason code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,
}

fn is_zero(v: &i64) -> bool {
    *v == 0
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

impl Record {
    pub fn new(t: i64, k: Kind, p: i64) -> Self {
        Self {
            t,
            k,
            p,
            b: None,
            m: 0,
            n: None,
        }
    }

    /// The line to append, with its newline.
    pub fn line(&self) -> String {
        let mut s = serde_json::to_string(self).unwrap_or_default();
        s.push('\n');
        s
    }
}

/// A log file's records; broken lines (e.g. cut off by a crash) are skipped.
pub fn read(text: &str) -> Vec<Record> {
    text.lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Stats {
    /// Net points over the whole log.
    #[ts(type = "number")]
    pub total: i64,
    /// Net points since this run of the app started.
    #[ts(type = "number")]
    pub session: i64,
    #[ts(type = "number | null")]
    pub balance: Option<u64>,
    #[ts(type = "number")]
    pub minutes: u64,
    /// Points and counts by kind, biggest first.
    pub kinds: Vec<KindTotal>,
    pub bets_won: u32,
    pub bets_lost: u32,
    /// Drops claimed, newest first: (time, name).
    #[ts(type = "Array<[number, string]>")]
    pub drops: Vec<(i64, String)>,
    /// (time, balance), oldest first.
    #[ts(type = "Array<[number, number]>")]
    pub series: Vec<(i64, u64)>,
    /// First record's time.
    #[ts(type = "number | null")]
    pub since: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KindTotal {
    pub kind: Kind,
    #[ts(type = "number")]
    pub points: i64,
    pub count: u32,
}

/// Sums up records (oldest first). `session_start` is when this run began, in ms.
pub fn summarize(records: &[Record], session_start: i64) -> Stats {
    let mut s = Stats {
        since: records.first().map(|r| r.t),
        ..Stats::default()
    };
    let mut kinds: Vec<KindTotal> = Vec::new();
    for r in records {
        s.total += r.p;
        if r.t >= session_start {
            s.session += r.p;
        }
        s.minutes += u64::from(r.m);
        match r.k {
            Kind::Won => s.bets_won += 1,
            Kind::Lost => s.bets_lost += 1,
            Kind::Drop => s.drops.push((r.t, r.n.clone().unwrap_or_default())),
            _ => {}
        }
        if let Some(b) = r.b {
            s.balance = Some(b);
            s.series.push((r.t, b));
        }
        if r.k == Kind::Watched {
            continue;
        }
        match kinds.iter_mut().find(|k| k.kind == r.k) {
            Some(k) => {
                k.points += r.p;
                k.count += 1;
            }
            None => kinds.push(KindTotal {
                kind: r.k,
                points: r.p,
                count: 1,
            }),
        }
    }
    kinds.sort_by_key(|k| std::cmp::Reverse(k.points.abs()));
    s.kinds = kinds;
    s.drops.reverse();
    s.series = thin(s.series, SERIES_MAX);
    s
}

/// Every n-th point, keeping the last.
fn thin(series: Vec<(i64, u64)>, max: usize) -> Vec<(i64, u64)> {
    if series.len() <= max {
        return series;
    }
    let step = series.len().div_ceil(max);
    let last = *series.last().expect("not empty");
    let mut out: Vec<_> = series.into_iter().step_by(step).collect();
    if out.last() != Some(&last) {
        out.push(last);
    }
    out
}

/// Adds several channels' stats together (the "All channels" view).
pub fn merge(all: &[Stats]) -> Stats {
    let mut out = Stats::default();
    for s in all {
        out.total += s.total;
        out.session += s.session;
        out.minutes += s.minutes;
        out.bets_won += s.bets_won;
        out.bets_lost += s.bets_lost;
        out.drops.extend(s.drops.iter().cloned());
        out.since = match (out.since, s.since) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        for k in &s.kinds {
            match out.kinds.iter_mut().find(|x| x.kind == k.kind) {
                Some(x) => {
                    x.points += k.points;
                    x.count += k.count;
                }
                None => out.kinds.push(k.clone()),
            }
        }
    }
    out.kinds.sort_by_key(|k| std::cmp::Reverse(k.points.abs()));
    out.drops.sort_by_key(|d| std::cmp::Reverse(d.0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(t: i64, k: Kind, p: i64, b: Option<u64>) -> Record {
        Record {
            b,
            ..Record::new(t, k, p)
        }
    }

    #[test]
    fn lines_round_trip_and_stay_small() {
        let r = rec(5, Kind::Chest, 50, Some(1050));
        assert_eq!(r.line(), "{\"t\":5,\"k\":\"chest\",\"p\":50,\"b\":1050}\n");
        let text = format!("{}garbage\n{{\"t\":6,\"k\":\"watched\",\"m\":5}}\n", r.line());
        let back = read(&text);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0], r);
        assert_eq!(back[1].m, 5);
    }

    #[test]
    fn summary_sums_by_kind_and_session() {
        let mut drop = Record::new(40, Kind::Drop, 0);
        drop.n = Some("Hat".into());
        let records = vec![
            rec(10, Kind::Watch, 10, Some(10)),
            rec(20, Kind::Chest, 50, Some(60)),
            rec(30, Kind::Bet, -20, Some(40)),
            rec(31, Kind::Lost, 0, None),
            drop,
            Record {
                m: 5,
                ..Record::new(50, Kind::Watched, 0)
            },
            rec(60, Kind::Watch, 10, Some(50)),
        ];
        let s = summarize(&records, 25);
        assert_eq!((s.total, s.session, s.balance, s.minutes), (50, -10, Some(50), 5));
        assert_eq!(
            s.kinds[0],
            KindTotal {
                kind: Kind::Chest,
                points: 50,
                count: 1
            }
        );
        assert!(s.kinds.iter().all(|k| k.kind != Kind::Watched));
        assert_eq!((s.bets_won, s.bets_lost), (0, 1));
        assert_eq!(s.drops, [(40, "Hat".to_owned())]);
        assert_eq!(s.series.len(), 4);
        assert_eq!(s.since, Some(10));
        let both = merge(&[s.clone(), s]);
        assert_eq!((both.total, both.drops.len()), (100, 2));
        assert_eq!(
            both.kinds.iter().find(|k| k.kind == Kind::Watch).map(|k| k.count),
            Some(4)
        );
    }

    #[test]
    fn long_series_thin_out() {
        let series: Vec<(i64, u64)> = (0..1000).map(|i| (i, i as u64)).collect();
        let out = thin(series, 240);
        assert!(out.len() <= 241);
        assert_eq!(out.last(), Some(&(999, 999)));
        assert_eq!(out[0], (0, 0));
    }

    #[test]
    fn reasons() {
        assert_eq!(Kind::from_reason("WATCH_STREAK"), Kind::Streak);
        assert_eq!(Kind::from_reason("SOMETHING_NEW"), Kind::Other);
    }
}
