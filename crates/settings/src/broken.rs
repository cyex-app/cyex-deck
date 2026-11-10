//! V110-A44 The `<name>.broken-<yyyymmdd>-<hhmmss>.json` files a start sets aside, listed for
//! the "Restore from a broken file" list. Only a name from this listing is ever opened.

use std::fs;
use std::path::Path;

use serde::Serialize;
use time::macros::format_description;
use time::{Date, Time};

use crate::Partition;

/// One set-aside file.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrokenFile {
    /// The file name in the settings folder; what `restore_broken` takes.
    pub name: String,
    pub partition: Partition,
    /// `yyyy/mm/dd HH:MM`, when it was set aside.
    pub when: String,
    /// Bytes.
    pub size: u64,
}

/// `(partition, stamp "yyyymmdd-hhmmss")` of a set-aside file name, `None` for any other name.
/// Accounts, plugins and the uploader never leave this PC, so their files are not offered.
fn parse(name: &str) -> Option<(Partition, &str)> {
    let (stem, rest) = name.split_once(".broken-")?;
    let stamp = rest.strip_suffix(".json")?;
    let p = Partition::from_key(stem).filter(|p| p.is_portable())?;
    let (day, time) = stamp.split_once('-')?;
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(day, 8) || !digits(time, 6) {
        return None;
    }
    Date::parse(day, format_description!("[year][month][day]")).ok()?;
    Time::parse(time, format_description!("[hour][minute][second]")).ok()?;
    Some((p, stamp))
}

fn when(stamp: &str) -> String {
    format!("{}/{}/{} {}:{}", &stamp[..4], &stamp[4..6], &stamp[6..8], &stamp[9..11], &stamp[11..13])
}

/// The set-aside files in `dir`, newest first.
pub fn list(dir: &Path) -> Vec<BrokenFile> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut found: Vec<(String, BrokenFile)> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let (partition, stamp) = parse(&name)?;
            let meta = e.metadata().ok().filter(|m| m.is_file())?;
            let key = stamp.to_owned();
            Some((key, BrokenFile { when: when(stamp), partition, size: meta.len(), name }))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
    found.into_iter().map(|(_, f)| f).collect()
}

/// The entry called `name` in the listing, or `None`: a path, another file or a made-up name.
pub fn find(dir: &Path, name: &str) -> Option<BrokenFile> {
    list(dir).into_iter().find(|f| f.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_set_aside_names_parse() {
        assert!(parse("general.broken-20261009-120000.json").is_some());
        assert_eq!(when("20261009-120507"), "2026/10/09 12:05");
        for bad in [
            "general.json",
            "general.broken-x.json",
            "general.broken-20261309-120000.json",
            "general.broken-20261009-126000.json",
            "nope.broken-20261009-120000.json",
            "accounts.broken-20261009-120000.json",
            "../general.broken-20261009-120000.json",
            "general.broken-20261009-120000.json.bak",
        ] {
            assert!(parse(bad).is_none(), "{bad}");
        }
    }
}
