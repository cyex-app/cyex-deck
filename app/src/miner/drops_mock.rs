//! V110-U68 Test builds only: `CYEX_DROPS_MOCK=<folder>` on a test data folder serves the
//! miner's drop data from JSON files there, read again on every call so a test can edit
//! them while the app runs (see tools/mock/drops/). Each file holds what Twitch's `data`
//! would, so the real parsers read it:
//! - `dashboard.json`: `{"currentUser":{"dropCampaigns":[{"id","status"}]}}`
//! - `details.json`: a list of `{"user":{"dropCampaign":{…}}}` answers
//! - `inventory.json`: `{"currentUser":{"inventory":{…}}}`
//! - `channels.json`: `{"<channel id>": ["<campaign id>"], "*": [...]}` (`*` = any channel)
//! - `helpers.json` (optional): `[{"id","login"}]`, the channels a badge search finds
//!
//! Claims succeed without a call. Nothing here reaches Twitch.

use std::path::{Path, PathBuf};

use cyex_twitch::gql::drops::{self, Campaign, Inventory};
use cyex_twitch::{Error, Result};
use serde_json::Value;
use tauri::AppHandle;

const VAR: &str = "CYEX_DROPS_MOCK";

/// The fixture folder when the mock is set: refused off a test data folder.
pub fn dir(app: &AppHandle) -> Option<std::result::Result<PathBuf, String>> {
    let raw = std::env::var(VAR).ok().filter(|r| !r.trim().is_empty())?;
    Some(check(&raw, crate::paths::webview(app).is_some()))
}

fn check(raw: &str, test_data: bool) -> std::result::Result<PathBuf, String> {
    if !test_data {
        return Err("The drops mock runs only on a test data folder.".into());
    }
    let p = PathBuf::from(raw.trim());
    if p.is_dir() {
        Ok(p)
    } else {
        Err(format!("The drops mock folder {} isn’t there.", p.display()))
    }
}

pub fn refused(why: String) -> Error {
    Error::Api {
        status: 0,
        message: why,
    }
}

fn read(dir: &Path, name: &str) -> Result<Value> {
    let text = std::fs::read_to_string(dir.join(name)).map_err(|e| refused(format!("{name}: {e}")))?;
    serde_json::from_str(&text).map_err(Error::Decode)
}

pub fn dashboard(dir: &Path) -> Result<Vec<String>> {
    drops::parse_dashboard(read(dir, "dashboard.json")?)
}

pub fn details(dir: &Path, ids: &[String]) -> Result<Vec<Campaign>> {
    let Value::Array(list) = read(dir, "details.json")? else {
        return Err(refused("details.json: not a list".into()));
    };
    Ok(list
        .into_iter()
        .filter_map(drops::campaign_from_details)
        .filter(|c| ids.contains(&c.id))
        .collect())
}

pub fn inventory(dir: &Path) -> Result<Inventory> {
    drops::parse_inventory(read(dir, "inventory.json")?)
}

pub fn channels(dir: &Path, ids: &[String]) -> Result<Vec<(String, Vec<String>)>> {
    let map = read(dir, "channels.json")?;
    Ok(ids
        .iter()
        .map(|id| (id.clone(), campaigns_of(&map, id)))
        .collect())
}

fn campaigns_of(map: &Value, id: &str) -> Vec<String> {
    let list = |v: &Value| -> Vec<String> {
        v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c.as_str().map(str::to_owned))
            .collect()
    };
    let mut out = list(&map[id]);
    for c in list(&map["*"]) {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

pub fn claim(_dir: &Path, _instance: &str) -> Result<bool> {
    Ok(true)
}

pub fn helpers(dir: &Path) -> Vec<(String, String)> {
    let Ok(Value::Array(list)) = read(dir, "helpers.json") else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|h| Some((h["id"].as_str()?.to_owned(), h["login"].as_str()?.to_lowercase())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn v110_u68_drops_mock_needs_a_test_data_folder() {
        assert!(check(".", false).is_err());
        assert!(check(".", true).is_ok());
        assert!(check("no/such/folder/here", true).is_err());
    }

    #[test]
    fn v110_u68_mock_channels_add_the_any_channel_list() {
        let map = json!({ "11": ["a"], "*": ["b", "a"] });
        assert_eq!(campaigns_of(&map, "11"), ["a", "b"]);
        assert_eq!(campaigns_of(&map, "12"), ["b", "a"]);
        assert!(campaigns_of(&json!({}), "1").is_empty());
    }

    #[test]
    fn v110_u68_the_shipped_fixtures_parse() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/mock/drops");
        let ids = dashboard(&dir).unwrap();
        assert!(!ids.is_empty());
        let c = details(&dir, &ids).unwrap();
        assert!(c.iter().any(|c| !cyex_miner::badges::watch_badges(c).is_empty()));
        inventory(&dir).unwrap();
        assert!(!channels(&dir, &["1".into()]).unwrap()[0].1.is_empty());
        assert!(!helpers(&dir).is_empty());
    }
}
