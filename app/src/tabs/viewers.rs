//! V106-18 Viewer counts over time for live tab channels, from the live check (`live`, once a
//! minute; no extra requests). Memory only (Q13). Old points thin out past `POINTS`; the peak
//! is kept apart so thinning never loses it.
//!
//! Size: a point is 16 bytes (i64 + u32, padded), so 240 points are about 4 KB; 100 channels
//! stay near 0,4 MB.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use cyex_twitch::gql::ChannelInfo;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

pub const POINTS: usize = 240;

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ViewerHistory {
    /// RFC 3339, the stream's start.
    pub started_at: String,
    /// `(unix seconds, viewers)`, oldest first.
    #[ts(type = "Array<[number, number]>")]
    pub points: Vec<(i64, u32)>,
    /// `(unix seconds, viewers)` of the highest count seen.
    #[ts(type = "[number, number]")]
    pub peak: (i64, u32),
}

#[derive(Default)]
pub struct Viewers(pub Mutex<HashMap<String, ViewerHistory>>);

/// One live check's answer. `tab_ids` are the channels in tabs; others are dropped.
pub fn record(
    map: &mut HashMap<String, ViewerHistory>,
    channels: &[ChannelInfo],
    tab_ids: &HashSet<String>,
    now: i64,
) {
    for c in channels {
        let Some(stream) = c.stream.as_ref().filter(|_| tab_ids.contains(&c.id)) else {
            continue;
        };
        let h = map
            .entry(c.id.clone())
            .or_insert_with(|| fresh(&stream.started_at, now, stream.viewers));
        if h.started_at != stream.started_at {
            *h = fresh(&stream.started_at, now, stream.viewers);
            continue;
        }
        if h.points.last().is_some_and(|p| p.0 == now) {
            continue;
        }
        h.points.push((now, stream.viewers));
        if stream.viewers >= h.peak.1 {
            h.peak = (now, stream.viewers);
        }
        if h.points.len() > POINTS {
            thin(&mut h.points);
        }
    }
    keep(map, tab_ids);
    map.retain(|id, _| channels.iter().any(|c| &c.id == id && c.stream.is_some()));
}

/// K5 Drops every channel no longer in a tab.
pub fn keep(map: &mut HashMap<String, ViewerHistory>, tab_ids: &HashSet<String>) {
    map.retain(|id, _| tab_ids.contains(id));
}

fn fresh(started_at: &str, now: i64, viewers: u32) -> ViewerHistory {
    ViewerHistory {
        started_at: started_at.to_owned(),
        points: vec![(now, viewers)],
        peak: (now, viewers),
    }
}

/// The older half becomes pair averages (the very first point keeps its time), the newer half stays.
fn thin(points: &mut Vec<(i64, u32)>) {
    let half = points.len() / 2;
    let newer = points.split_off(half);
    let older = std::mem::take(points);
    for (i, pair) in older.chunks(2).enumerate() {
        if let [a, b] = pair {
            let t = if i == 0 { a.0 } else { (a.0 + b.0).div_euclid(2) };
            points.push((t, (u64::from(a.1) + u64::from(b.1)).div_ceil(2) as u32));
        } else {
            points.push(pair[0]);
        }
    }
    points.extend(newer);
}

pub fn get(app: &AppHandle, channel_id: &str) -> Option<ViewerHistory> {
    let v = app.try_state::<Viewers>()?;
    let map = v.0.lock().unwrap_or_else(|p| p.into_inner());
    map.get(channel_id).cloned()
}

#[cfg(test)]
mod tests {
    use cyex_twitch::gql::StreamInfo;

    use super::*;

    fn ch(id: &str, live: Option<(&str, u32)>) -> ChannelInfo {
        ChannelInfo {
            id: id.into(),
            login: id.into(),
            name: id.into(),
            avatar: None,
            offline_image: None,
            stream: live.map(|(s, v)| StreamInfo {
                rerun: false,
                title: String::new(),
                game: None,
                viewers: v,
                started_at: s.into(),
            }),
        }
    }

    fn tabs(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn adds_points_and_tracks_the_peak() {
        let mut m = HashMap::new();
        let t = tabs(&["1"]);
        record(&mut m, &[ch("1", Some(("a", 10)))], &t, 100);
        record(&mut m, &[ch("1", Some(("a", 30)))], &t, 160);
        record(&mut m, &[ch("1", Some(("a", 20)))], &t, 220);
        let h = &m["1"];
        assert_eq!(h.points, vec![(100, 10), (160, 30), (220, 20)]);
        assert_eq!(h.peak, (160, 30));
    }

    #[test]
    fn new_stream_resets() {
        let mut m = HashMap::new();
        let t = tabs(&["1"]);
        record(&mut m, &[ch("1", Some(("a", 50)))], &t, 100);
        record(&mut m, &[ch("1", Some(("b", 5)))], &t, 160);
        assert_eq!(m["1"].points, vec![(160, 5)]);
        assert_eq!(m["1"].peak, (160, 5));
    }

    #[test]
    fn offline_and_non_tab_channels_drop() {
        let mut m = HashMap::new();
        record(
            &mut m,
            &[ch("1", Some(("a", 5))), ch("2", Some(("a", 5)))],
            &tabs(&["1", "2"]),
            100,
        );
        assert_eq!(m.len(), 2);
        record(
            &mut m,
            &[ch("1", None), ch("2", Some(("a", 6)))],
            &tabs(&["1", "2"]),
            160,
        );
        assert!(!m.contains_key("1"));
        record(
            &mut m,
            &[ch("2", Some(("a", 7))), ch("3", Some(("a", 1)))],
            &tabs(&["3"]),
            220,
        );
        assert!(!m.contains_key("2"));
        assert!(m.contains_key("3"));
        // Own channel (not a tab) is never kept.
        record(&mut m, &[ch("9", Some(("a", 1)))], &tabs(&["3"]), 280);
        assert!(!m.contains_key("9"));
    }

    #[test]
    fn closed_tab_drops_at_once() {
        let mut m = HashMap::new();
        let t = tabs(&["1", "2"]);
        record(&mut m, &[ch("1", Some(("a", 5))), ch("2", Some(("a", 7)))], &t, 100);
        record(&mut m, &[ch("1", Some(("a", 6))), ch("2", Some(("a", 8)))], &t, 160);
        keep(&mut m, &tabs(&["1"]));
        assert!(!m.contains_key("2"));
        assert_eq!(m["1"].points, vec![(100, 5), (160, 6)]);
    }

    #[test]
    fn same_second_is_skipped() {
        let mut m = HashMap::new();
        let t = tabs(&["1"]);
        record(&mut m, &[ch("1", Some(("a", 5)))], &t, 100);
        record(&mut m, &[ch("1", Some(("a", 9)))], &t, 100);
        assert_eq!(m["1"].points, vec![(100, 5)]);
    }

    #[test]
    fn cap_thins_but_keeps_ends_and_peak() {
        let mut m = HashMap::new();
        let t = tabs(&["1"]);
        for i in 0..1000_i64 {
            let v = if i == 3 { 9999 } else { 100 + (i % 7) as u32 };
            record(&mut m, &[ch("1", Some(("a", v)))], &t, i * 60);
        }
        let h = &m["1"];
        assert!(h.points.len() <= POINTS);
        assert_eq!(h.points.first().unwrap().0, 0);
        assert_eq!(h.points.last().unwrap().0, 999 * 60);
        assert_eq!(h.peak, (180, 9999));
        assert!(h.points.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn thin_shrinks_to_three_quarters() {
        let mut p: Vec<(i64, u32)> = (0..=POINTS as i64).map(|i| (i, 10)).collect();
        thin(&mut p);
        assert_eq!(p.len(), 60 + 121);
    }
}
