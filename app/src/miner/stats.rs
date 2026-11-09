//! 1.32 Miner stats on disk: `miner\<account id>\<channel id>.jsonl` next to the
//! settings folder, one line per event (`cyex_miner::stats`), plus `names.json` so
//! channels show by name. Summed up when the stats menu asks.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use cyex_miner::stats::{self, Record, Stats};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StatsChannel {
    pub id: String,
    pub login: String,
    pub name: String,
}

/// Ids are digits; anything else never becomes a path.
fn digits(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_digit())
}

fn dir(app: &AppHandle, account: &str) -> Option<PathBuf> {
    if !digits(account) {
        return None;
    }
    crate::paths::config(app).ok().map(|d| d.join("miner").join(account))
}

fn file(app: &AppHandle, account: &str, channel_id: &str) -> Option<PathBuf> {
    if !digits(channel_id) {
        return None;
    }
    dir(app, account).map(|d| d.join(format!("{channel_id}.jsonl")))
}

type Names = BTreeMap<String, (String, String)>;

fn read_names(app: &AppHandle, account: &str) -> Names {
    dir(app, account)
        .and_then(|d| fs::read(d.join("names.json")).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// V110-F82 Temp file and rename: a crash mid-write used to cut `names.json` short, and then
/// every channel showed as a raw id.
fn write_names(dir: &Path, names: &Names) {
    if let Ok(json) = serde_json::to_vec(names) {
        let _ = cyex_settings::write_atomic(&dir.join("names.json"), &json);
    }
}

/// Appends one event to a channel's log.
pub fn record(app: &AppHandle, account: &str, channel_id: &str, rec: &Record) {
    let Some(path) = file(app, account, channel_id) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let ok = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(rec.line().as_bytes()));
    if let Err(e) = ok {
        eprintln!("miner: couldn't write stats: {e}");
        return;
    }
    if let Some((login, name)) = super::name_of(app, channel_id) {
        let mut names = read_names(app, account);
        let entry = (login, name);
        if names.get(channel_id) != Some(&entry) {
            names.insert(channel_id.to_owned(), entry);
            if let Some(d) = dir(app, account) {
                write_names(&d, &names);
            }
        }
    }
}

/// Channels with stats, A-Z (1.32).
pub fn channels(app: &AppHandle, account: &str) -> Vec<StatsChannel> {
    let names = read_names(app, account);
    let Some(d) = dir(app, account) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(d) else {
        return Vec::new();
    };
    let mut out: Vec<StatsChannel> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let id = name.strip_suffix(".jsonl")?.to_owned();
            let (login, display) = names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| (id.clone(), id.clone()));
            Some(StatsChannel {
                id,
                login,
                name: display,
            })
        })
        .collect();
    out.sort_by_key(|c| c.name.to_lowercase());
    out
}

/// One channel's stats, or every channel's added up.
pub fn summary(app: &AppHandle, account: &str, channel_id: Option<&str>) -> Stats {
    let start = app.state::<super::Miner>().session_start;
    let read = |id: &str| -> Stats {
        let text = file(app, account, id)
            .and_then(|p| fs::read_to_string(p).ok())
            .unwrap_or_default();
        stats::summarize(&stats::read(&text), start)
    };
    match channel_id {
        Some(id) => read(id),
        None => {
            let all: Vec<Stats> = channels(app, account).iter().map(|c| read(&c.id)).collect();
            stats::merge(&all)
        }
    }
}

/// Deletes one channel's stats, or all of them.
pub fn clear(app: &AppHandle, account: &str, channel_id: Option<&str>) -> Result<(), String> {
    let result = match channel_id {
        Some(id) => file(app, account, id).map_or(Ok(()), fs::remove_file),
        None => dir(app, account).map_or(Ok(()), fs::remove_dir_all),
    };
    match result {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("Couldn’t delete the stats: {e}")),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// V110-F82 A write that fails halfway leaves the old names.json whole.
    #[test]
    fn names_are_written_through_a_temp_file() {
        let dir = std::env::temp_dir().join(format!("cyex-stats-names-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let mut names = Names::new();
        names.insert("1".into(), ("a".into(), "A".into()));
        write_names(&dir, &names);
        let before = fs::read(dir.join("names.json")).unwrap();
        // The temp file can't be made: a plain write would have replaced names.json anyway.
        fs::create_dir(dir.join("names.tmp")).unwrap();
        names.insert("2".into(), ("b".into(), "B".into()));
        write_names(&dir, &names);
        assert_eq!(fs::read(dir.join("names.json")).unwrap(), before);
        let _ = fs::remove_dir_all(&dir);
    }
}
