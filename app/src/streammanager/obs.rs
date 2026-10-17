//! X4 Stream status files for OBS and the marker log. When on (Settings, off by default),
//! each stats tick writes one small text file per value into the chosen folder, only when
//! it changed, so an OBS "Text (from file)" source can show it. Markers placed from the
//! stream manager are always appended to `markers.log` in the same folder.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use cyex_core::number::grouped;
use cyex_settings::Partition;
use cyex_settings::schema::StreamManager;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use ts_rs::TS;

use super::stats::Stats;
use crate::settings::Settings;

const MARKERS: &str = "markers.log";
/// V106-16 Every file in the status folder and what it holds, in table order.
pub const FILES: [(&str, &str); 8] = [
    ("live.txt", "Live or offline"),
    ("title.txt", "Your stream title"),
    ("category.txt", "Your category"),
    ("viewers.txt", "Viewers right now"),
    ("uptime.txt", "Time live, like 1:23"),
    ("followers.txt", "Your follower count"),
    ("subscribers.txt", "Your subscriber count"),
    (MARKERS, "Markers you add, with the time and how long you’d been live"),
];

/// V106-16 One row of the files table in Settings.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ObsFile {
    pub name: String,
    pub what: String,
    pub path: String,
    pub exists: bool,
}

/// V106-16 The files table: each name under the status folder and whether it is there.
pub fn table(app: &AppHandle) -> Vec<ObsFile> {
    let dir = folder(app);
    FILES
        .iter()
        .map(|(name, what)| {
            let path = dir.as_ref().map(|d| d.join(name));
            ObsFile {
                name: (*name).into(),
                what: (*what).into(),
                path: path.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
                exists: path.is_some_and(|p| p.is_file()),
            }
        })
        .collect()
}

/// How many marker lines the UI lists.
const MARKERS_SHOWN: usize = 50;

/// The folder last written to and what each file holds, so unchanged files aren't rewritten.
struct Written {
    dir: Option<PathBuf>,
    files: Vec<(&'static str, String)>,
}

static LAST: Mutex<Written> = Mutex::new(Written {
    dir: None,
    files: Vec::new(),
});

/// The status folder: the one set in Settings, else `stream-status/` next to settings.
pub fn folder(app: &AppHandle) -> Option<PathBuf> {
    let sm: StreamManager = app.state::<Settings>().read(Partition::StreamManager);
    let set = sm.obs.folder.trim();
    if !set.is_empty() {
        return Some(PathBuf::from(set));
    }
    crate::paths::config(app).ok().map(|d| d.join("stream-status"))
}

/// `H:MM` since `started_at`, or `offline`.
pub fn uptime(started_at: &str, now: i64) -> String {
    let Ok(start) = OffsetDateTime::parse(started_at, &Rfc3339) else {
        return "offline".into();
    };
    let secs = (now - start.unix_timestamp()).max(0);
    format!("{}:{:02}", secs / 3600, secs / 60 % 60)
}

/// Each file name and what it should hold.
pub fn files(s: &Stats, now: i64) -> Vec<(&'static str, String)> {
    let opt = |n: Option<u64>| n.map(grouped).unwrap_or_default();
    vec![
        ("live.txt", if s.live { "live" } else { "offline" }.into()),
        ("title.txt", s.title.clone()),
        ("category.txt", s.game.clone()),
        (
            "viewers.txt",
            if s.live { grouped(s.viewers) } else { "0".into() },
        ),
        (
            "uptime.txt",
            if s.live {
                uptime(&s.started_at, now)
            } else {
                "offline".into()
            },
        ),
        ("followers.txt", opt(s.followers)),
        ("subscribers.txt", opt(s.subs)),
    ]
}

/// Writes the status files when they're on and something changed.
pub fn write(app: &AppHandle, s: &Stats) {
    let sm: StreamManager = app.state::<Settings>().read(Partition::StreamManager);
    if !sm.obs.enabled {
        return;
    }
    let Some(dir) = folder(app) else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let mut last = LAST.lock().unwrap_or_else(|p| p.into_inner());
    if last.dir.as_ref() != Some(&dir) {
        last.dir = Some(dir.clone());
        last.files.clear();
    }
    for (name, text) in files(s, crate::auth::unix_now()) {
        if last.files.iter().any(|(n, t)| *n == name && *t == text) {
            continue;
        }
        if std::fs::write(dir.join(name), &text).is_ok() {
            last.files.retain(|(n, _)| *n != name);
            last.files.push((name, text));
        }
    }
}

/// One marker-log line: local date and time, uptime when live, the description.
pub fn marker_line(at: OffsetDateTime, uptime: Option<&str>, description: &str) -> String {
    let when = at
        .format(format_description!(
            "[year]/[month]/[day] [hour]:[minute]:[second]"
        ))
        .unwrap_or_default();
    let text = if description.trim().is_empty() {
        "(no description)"
    } else {
        description.trim()
    };
    match uptime {
        Some(u) => format!("{when}  {u}  {text}"),
        None => format!("{when}  {text}"),
    }
}

/// Appends a placed marker to `markers.log`.
pub fn log_marker(app: &AppHandle, description: &str) {
    let Some(dir) = folder(app) else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let up = super::stats::latest(app)
        .filter(|s| s.live)
        .map(|s| uptime(&s.started_at, now.unix_timestamp()));
    let line = marker_line(now, up.as_deref(), description);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(MARKERS))
    {
        let _ = writeln!(f, "{line}");
    }
}

/// The newest marker-log lines, newest first.
pub fn markers(app: &AppHandle) -> Vec<String> {
    let Some(dir) = folder(app) else { return Vec::new() };
    let text = std::fs::read_to_string(dir.join(MARKERS)).unwrap_or_default();
    text.lines()
        .rev()
        .filter(|l| !l.is_empty())
        .take(MARKERS_SHOWN)
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_match_the_table() {
        let names: Vec<&str> = files(&Stats::default(), 0).iter().map(|(n, _)| *n).collect();
        let table: Vec<&str> = FILES.iter().take(7).map(|(n, _)| *n).collect();
        assert_eq!(names, table);
        assert_eq!(FILES[7].0, MARKERS);
    }

    #[test]
    fn uptime_counts_hours_and_minutes() {
        let start = "2024-01-01T10:00:00Z";
        let t0 = OffsetDateTime::parse(start, &Rfc3339).unwrap().unix_timestamp();
        assert_eq!(uptime(start, t0 + 3 * 3600 + 7 * 60 + 59), "3:07");
        assert_eq!(uptime(start, t0 - 5), "0:00");
        assert_eq!(uptime("", t0), "offline");
    }

    #[test]
    fn files_offline_and_live() {
        let mut s = Stats {
            title: "hi".into(),
            game: "Chess".into(),
            followers: Some(12_345),
            ..Stats::default()
        };
        let off = files(&s, 0);
        let get = |v: &[(&str, String)], k: &str| v.iter().find(|(n, _)| *n == k).unwrap().1.clone();
        assert_eq!(get(&off, "live.txt"), "offline");
        assert_eq!(get(&off, "viewers.txt"), "0");
        assert_eq!(get(&off, "followers.txt"), "12.345");
        assert_eq!(get(&off, "subscribers.txt"), "");
        s.live = true;
        s.viewers = 1500;
        s.started_at = "2024-01-01T10:00:00Z".into();
        let t0 = OffsetDateTime::parse(&s.started_at, &Rfc3339)
            .unwrap()
            .unix_timestamp();
        let on = files(&s, t0 + 90 * 60);
        assert_eq!(get(&on, "viewers.txt"), "1.500");
        assert_eq!(get(&on, "uptime.txt"), "1:30");
    }

    #[test]
    fn marker_lines() {
        let at = OffsetDateTime::parse("2026-10-10T21:04:33Z", &Rfc3339).unwrap();
        assert_eq!(
            marker_line(at, Some("1:23"), " big play "),
            "2026/10/10 21:04:33  1:23  big play"
        );
        assert_eq!(marker_line(at, None, ""), "2026/10/10 21:04:33  (no description)");
    }
}
