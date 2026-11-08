//! Daily snapshots of the settings files (2.8).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use time::{Date, OffsetDateTime, PrimitiveDateTime};
use time::macros::format_description;

use crate::error::Result;
use crate::{Partition, legacy};

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
    // V110-F56 After a failure (a full disk, a locked folder) it waits an hour instead of
    // copying everything again on every flush.
    let mut failed = FAILED.lock().unwrap_or_else(|p| p.into_inner());
    if failed.get(settings_dir).is_some_and(|t| t.elapsed() < RETRY) {
        return Ok(false);
    }
    let res = snapshot_due(settings_dir, today);
    match &res {
        Ok(_) => {
            failed.remove(settings_dir);
        }
        Err(_) => {
            failed.insert(settings_dir.to_owned(), Instant::now());
        }
    }
    res
}

/// V110-F56 How long a failed daily backup waits before trying again.
const RETRY: Duration = Duration::from_secs(60 * 60);

static FAILED: LazyLock<Mutex<HashMap<PathBuf, Instant>>> = LazyLock::new(Mutex::default);

fn snapshot_due(settings_dir: &Path, today: Date) -> Result<bool> {
    let target = snapshot_dir(settings_dir, today);
    if target.exists() {
        // t5 Prune anyway: copies left from before (another build, a restore, a clock change)
        // would otherwise stay listed until tomorrow.
        prune(&settings_dir.join(BACKUP_DIR), today)?;
        return Ok(false);
    }
    // V106-33 The old mentions.json too: the merge-day backup must restore without losing it.
    let files: Vec<PathBuf> = Partition::ALL
        .iter()
        .map(|p| p.file_name())
        .chain([legacy::mentions::FILE.to_owned()])
        .map(|name| settings_dir.join(name))
        .filter(|f| reads_as_json(f))
        .collect();
    if files.is_empty() {
        return Ok(false);
    }

    // Copy into a temp folder first so a crash never leaves a half snapshot.
    let root = settings_dir.join(BACKUP_DIR);
    stage(&root, &format!(".partial-{}", date_key(today)), &files, &target)?;
    prune(&root, today)?;
    Ok(true)
}

/// V110-F56 Copies `files` into a `.partial-*` folder, then renames it to `target`. A failure
/// removes the folder again, so none is left behind.
fn stage(root: &Path, partial: &str, files: &[PathBuf], target: &Path) -> Result<()> {
    let staging = root.join(partial);
    let copy = || -> std::io::Result<()> {
        if staging.exists() {
            fs::remove_dir_all(&staging)?;
        }
        fs::create_dir_all(&staging)?;
        for file in files {
            fs::copy(file, staging.join(file.file_name().expect("settings files have names")))?;
        }
        fs::rename(&staging, target)
    };
    copy().inspect_err(|_| {
        let _ = fs::remove_dir_all(&staging);
    })?;
    Ok(())
}

/// V110-F56 Removes `.partial-*` folders a crash left in the backups folder. Run at open,
/// before anything stages a new one.
pub(crate) fn sweep_partials(settings_dir: &Path) {
    let Ok(entries) = fs::read_dir(settings_dir.join(BACKUP_DIR)) else { return };
    for entry in entries.filter_map(|e| e.ok()) {
        if entry.file_name().to_string_lossy().starts_with(".partial-") {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// V107-N10a A file the day's backup may take: one that's broken today would push the last
/// good copy out of the three days kept.
fn reads_as_json(file: &Path) -> bool {
    fs::read(file).is_ok_and(|b| serde_json::from_slice::<serde::de::IgnoredAny>(&b).is_ok())
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

/// How many "before restore" snapshots to keep.
pub const SNAPSHOT_KEEP: usize = 3;

const SNAPSHOT_PREFIX: &str = "before-restore-";
const STAMP_FORMAT: &[time::format_description::BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day]-[hour][minute][second]");

/// V1052-03 `before-restore-yyyy-mm-dd-hhmmss`.
pub(crate) fn snapshot_name(now: OffsetDateTime) -> String {
    format!("{SNAPSHOT_PREFIX}{}", now.format(STAMP_FORMAT).expect("date formats"))
}

pub(crate) fn is_snapshot_name(name: &str) -> bool {
    name.strip_prefix(SNAPSHOT_PREFIX)
        .is_some_and(|stamp| PrimitiveDateTime::parse(stamp, STAMP_FORMAT).is_ok())
}

pub(crate) fn named_dir(settings_dir: &Path, name: &str) -> PathBuf {
    settings_dir.join(BACKUP_DIR).join(name)
}

/// Writes the current files to their own dated folder, keeping the newest [`SNAPSHOT_KEEP`].
/// Accounts, plugins and the uploader stay out (they can hold secrets). Returns the folder
/// name, or `None` when there is nothing to save.
pub(crate) fn snapshot_now(settings_dir: &Path, now: OffsetDateTime) -> Result<Option<String>> {
    let files: Vec<PathBuf> = Partition::ALL
        .iter()
        .filter(|p| p.is_portable())
        .map(|p| p.file_name())
        .chain([legacy::mentions::FILE.to_owned()])
        .map(|name| settings_dir.join(name))
        .filter(|f| f.is_file())
        .collect();
    if files.is_empty() {
        return Ok(None);
    }
    let name = snapshot_name(now);
    let target = named_dir(settings_dir, &name);
    if target.exists() {
        return Ok(Some(name));
    }
    let root = settings_dir.join(BACKUP_DIR);
    stage(&root, &format!(".partial-{name}"), &files, &target)?;
    for old in list_snapshots(settings_dir)?.into_iter().skip(SNAPSHOT_KEEP) {
        fs::remove_dir_all(named_dir(settings_dir, &old))?;
    }
    Ok(Some(name))
}

/// Snapshot folder names, newest first.
pub(crate) fn list_snapshots(settings_dir: &Path) -> Result<Vec<String>> {
    let root = settings_dir.join(BACKUP_DIR);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = fs::read_dir(&root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_snapshot_name(name))
        .collect();
    names.sort_unstable_by(|a, b| b.cmp(a));
    Ok(names)
}

/// Keeps the newest [`BACKUP_DAYS`] up to `today`. t5 A folder dated after today (the clock was
/// once ahead) doesn't count: it used to take a slot, so each new day's copy was pruned at once.
/// It is left alone and counts again once its date comes.
fn prune(root: &Path, today: Date) -> Result<()> {
    let settings_dir = root.parent().expect("backups live inside the settings folder");
    let past = list(settings_dir)?.into_iter().filter(|d| *d <= today);
    for old in past.skip(BACKUP_DAYS) {
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
    fn v110_f56_a_failed_copy_leaves_no_partial_and_waits() {
        let tmp = TempDir::new("backup-fail");
        let root = tmp.path().join(BACKUP_DIR);
        fs::create_dir_all(&root).unwrap();
        let missing = vec![tmp.path().join("gone.json")];
        assert!(stage(&root, ".partial-x", &missing, &root.join("x")).is_err());
        assert!(!root.join(".partial-x").exists());
        // A blocked staging folder (a file in its place) fails the day's backup; the next flush skips it.
        write_settings(tmp.path(), "{\"v\":1}");
        fs::write(root.join(".partial-2026-10-01"), "x").unwrap();
        assert!(snapshot_if_due(tmp.path(), day(1)).is_err());
        fs::remove_file(root.join(".partial-2026-10-01")).unwrap();
        assert!(!snapshot_if_due(tmp.path(), day(1)).unwrap(), "waits an hour");
        assert!(list(tmp.path()).unwrap().is_empty());
    }

    #[test]
    fn v110_f56_stale_partials_are_swept() {
        let tmp = TempDir::new("backup-sweep");
        let root = tmp.path().join(BACKUP_DIR);
        fs::create_dir_all(root.join(".partial-2026-10-01")).unwrap();
        fs::create_dir_all(root.join("2026-10-01")).unwrap();
        sweep_partials(tmp.path());
        assert!(!root.join(".partial-2026-10-01").exists());
        assert!(root.join("2026-10-01").exists());
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

    fn at(d: u8, h: u8, m: u8, s: u8) -> OffsetDateTime {
        day(d).with_time(time::Time::from_hms(h, m, s).unwrap()).assume_utc()
    }

    #[test]
    fn prunes_leftovers_even_when_today_exists() {
        let tmp = TempDir::new("backup-leftovers");
        write_settings(tmp.path(), "{}");
        for d in 1..=5 {
            fs::create_dir_all(snapshot_dir(tmp.path(), day(d))).unwrap();
        }
        assert!(!snapshot_if_due(tmp.path(), day(5)).unwrap());
        assert_eq!(list(tmp.path()).unwrap(), vec![day(5), day(4), day(3)]);
    }

    #[test]
    fn a_future_dated_copy_does_not_take_a_slot() {
        let tmp = TempDir::new("backup-future");
        write_settings(tmp.path(), "{}");
        fs::create_dir_all(snapshot_dir(tmp.path(), day(20))).unwrap();
        for d in 1..=5 {
            snapshot_if_due(tmp.path(), day(d)).unwrap();
        }
        assert_eq!(list(tmp.path()).unwrap(), vec![day(20), day(5), day(4), day(3)]);
    }

    #[test]
    fn snapshot_names_carry_the_time() {
        assert_eq!(snapshot_name(at(16, 14, 3, 9)), "before-restore-2026-10-16-140309");
        assert!(is_snapshot_name("before-restore-2026-10-16-140309"));
        assert!(!is_snapshot_name("before-restore-"));
        assert!(!is_snapshot_name("before-restore-2026-10-16"));
        assert!(!is_snapshot_name("2026-10-16"));
    }

    #[test]
    fn snapshots_keep_the_newest_three_and_leave_daily_ones_alone() {
        let tmp = TempDir::new("backup-snap-rotate");
        write_settings(tmp.path(), "{}");
        snapshot_if_due(tmp.path(), day(1)).unwrap();
        for s in 1..=5 {
            snapshot_now(tmp.path(), at(2, 10, 0, s)).unwrap();
        }
        let kept = list_snapshots(tmp.path()).unwrap();
        assert_eq!(
            kept,
            ["before-restore-2026-10-02-100005", "before-restore-2026-10-02-100004", "before-restore-2026-10-02-100003"]
        );
        assert_eq!(list(tmp.path()).unwrap(), vec![day(1)], "the daily slot is untouched");
    }

    #[test]
    fn snapshots_hold_no_secrets() {
        let tmp = TempDir::new("backup-snap-secrets");
        write_settings(tmp.path(), "{\"v\":1}");
        for name in ["accounts.json", "plugins.json", "uploader.json"] {
            fs::write(tmp.path().join(name), "s3cr3t").unwrap();
        }
        let name = snapshot_now(tmp.path(), at(2, 10, 0, 0)).unwrap().unwrap();
        let dir = named_dir(tmp.path(), &name);
        assert!(dir.join("general.json").is_file());
        for name in ["accounts.json", "plugins.json", "uploader.json"] {
            assert!(!dir.join(name).exists(), "{name} stays out");
        }
    }

    // V106-33
    #[test]
    fn old_mentions_file_is_backed_up() {
        let tmp = TempDir::new("backup-mentions");
        write_settings(tmp.path(), "{}");
        fs::write(tmp.path().join("mentions.json"), "{\"list\":[]}").unwrap();
        snapshot_if_due(tmp.path(), day(1)).unwrap();
        assert!(snapshot_dir(tmp.path(), day(1)).join("mentions.json").is_file());
        let name = snapshot_now(tmp.path(), at(2, 10, 0, 0)).unwrap().unwrap();
        assert!(named_dir(tmp.path(), &name).join("mentions.json").is_file());
    }

    #[test]
    fn nothing_to_snapshot_writes_nothing() {
        let tmp = TempDir::new("backup-snap-empty");
        assert_eq!(snapshot_now(tmp.path(), at(2, 10, 0, 0)).unwrap(), None);
        assert!(!tmp.path().join(BACKUP_DIR).exists());
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
