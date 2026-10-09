//! Daily snapshots of the settings files (2.8).

use std::fs;
use std::path::{Path, PathBuf};

use time::Date;
use time::macros::format_description;

use crate::Partition;
use crate::error::Result;

/// How many daily snapshots to keep.
pub const BACKUP_DAYS: usize = 3;

const BACKUP_DIR: &str = "backups";
const DATE_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day]");

pub(crate) fn date_key(date: Date) -> String {
    date.format(DATE_FORMAT).expect("date formats")
}

fn parse_key(key: &str) -> Option<Date> {
    Date::parse(key, DATE_FORMAT).ok()
}

pub(crate) fn snapshot_dir(settings_dir: &Path, date: Date) -> PathBuf {
    settings_dir.join(BACKUP_DIR).join(date_key(date))
}

/// Copies the current files into today's snapshot unless it exists, then prunes old ones.
/// Returns whether a snapshot was written.
pub(crate) fn snapshot_if_due(settings_dir: &Path, today: Date) -> Result<bool> {
    let target = snapshot_dir(settings_dir, today);
    if target.exists() {
        return Ok(false);
    }
    let files: Vec<PathBuf> = Partition::ALL
        .iter()
        .map(|p| settings_dir.join(p.file_name()))
        .filter(|f| f.is_file())
        .collect();
    if files.is_empty() {
        return Ok(false);
    }

    // Copy into a temp folder first so a crash never leaves a half snapshot.
    let root = settings_dir.join(BACKUP_DIR);
    let staging = root.join(format!(".partial-{}", date_key(today)));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;
    for file in &files {
        fs::copy(
            file,
            staging.join(file.file_name().expect("settings files have names")),
        )?;
    }
    fs::rename(&staging, &target)?;
    prune(&root)?;
    Ok(true)
}

/// Snapshot dates, newest first.
pub(crate) fn list(settings_dir: &Path) -> Result<Vec<Date>> {
    let root = settings_dir.join(BACKUP_DIR);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut dates: Vec<Date> = fs::read_dir(&root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| parse_key(&entry.file_name().to_string_lossy()))
        .collect();
    dates.sort_unstable_by(|a, b| b.cmp(a));
    Ok(dates)
}

fn prune(root: &Path) -> Result<()> {
    let settings_dir = root.parent().expect("backups live inside the settings folder");
    for old in list(settings_dir)?.into_iter().skip(BACKUP_DAYS) {
        fs::remove_dir_all(snapshot_dir(settings_dir, old))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{TempDir, day};

    fn write_settings(dir: &Path, contents: &str) {
        fs::write(dir.join("general.json"), contents).unwrap();
    }

    #[test]
    fn skips_when_there_is_nothing_to_back_up() {
        let tmp = TempDir::new("backup-empty");
        assert!(!snapshot_if_due(tmp.path(), day(1)).unwrap());
        assert!(list(tmp.path()).unwrap().is_empty());
    }

    #[test]
    fn one_snapshot_per_day() {
        let tmp = TempDir::new("backup-daily");
        write_settings(tmp.path(), "{\"v\":1}");
        assert!(snapshot_if_due(tmp.path(), day(1)).unwrap());

        write_settings(tmp.path(), "{\"v\":2}");
        assert!(!snapshot_if_due(tmp.path(), day(1)).unwrap());

        let saved = fs::read_to_string(snapshot_dir(tmp.path(), day(1)).join("general.json")).unwrap();
        assert_eq!(saved, "{\"v\":1}", "the first state of the day is kept");
    }

    #[test]
    fn keeps_only_the_newest_three_days() {
        let tmp = TempDir::new("backup-prune");
        write_settings(tmp.path(), "{}");
        for d in 1..=5 {
            snapshot_if_due(tmp.path(), day(d)).unwrap();
        }
        assert_eq!(list(tmp.path()).unwrap(), vec![day(5), day(4), day(3)]);
        assert!(!snapshot_dir(tmp.path(), day(2)).exists());
    }

    #[test]
    fn ignores_foreign_folders() {
        let tmp = TempDir::new("backup-foreign");
        write_settings(tmp.path(), "{}");
        fs::create_dir_all(tmp.path().join(BACKUP_DIR).join("not-a-date")).unwrap();
        snapshot_if_due(tmp.path(), day(1)).unwrap();
        assert_eq!(list(tmp.path()).unwrap(), vec![day(1)]);
    }
}
