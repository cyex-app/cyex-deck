use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use time::Date;
use time::macros::format_description;

use crate::error::{Error, Result};
use crate::merge::merge_patch;
use crate::transfer::Bundle;
use crate::{Clock, Partition, Schema, backup, fsutil};

/// What happened while loading, for the UI to report.
#[derive(Debug, Default)]
pub struct LoadReport {
    /// Partitions whose file was unreadable. The file was renamed to
    /// `<name>.broken-<stamp>.json` and defaults were used.
    pub reset: Vec<Partition>,
}

/// In-memory settings, written back to disk by [`Store::flush`].
pub struct Store {
    dir: PathBuf,
    values: BTreeMap<Partition, Value>,
    dirty: BTreeSet<Partition>,
    clock: Arc<dyn Clock>,
}

impl Store {
    /// Loads every partition from `dir`, creating it if needed. Takes today's backup
    /// before anything is changed.
    pub fn open(dir: impl Into<PathBuf>, clock: Arc<dyn Clock>) -> Result<(Self, LoadReport)> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        backup::snapshot_if_due(&dir, clock.today())?;

        let mut report = LoadReport::default();
        let mut values = BTreeMap::new();
        for p in Partition::ALL {
            let path = dir.join(p.file_name());
            let value = match read_partition(&path, p) {
                Ok(Some(v)) => v,
                Ok(None) => p.default_value(),
                Err(_) => {
                    quarantine(&path, &*clock)?;
                    report.reset.push(p);
                    p.default_value()
                }
            };
            values.insert(p, value);
        }
        Ok((
            Self {
                dir,
                values,
                dirty: BTreeSet::new(),
                clock,
            },
            report,
        ))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn get(&self, p: Partition) -> &Value {
        &self.values[&p]
    }

    pub fn all(&self) -> &BTreeMap<Partition, Value> {
        &self.values
    }

    /// Typed copy of a partition.
    pub fn read<T: Schema>(&self, p: Partition) -> T {
        serde_json::from_value(self.get(p).clone()).unwrap_or_default()
    }

    /// Replaces a partition after running it through the schema. Returns the stored value.
    pub fn set(&mut self, p: Partition, value: Value) -> Result<&Value> {
        let normalized = p.normalize(value)?;
        if self.values[&p] != normalized {
            self.values.insert(p, normalized);
            self.dirty.insert(p);
        }
        Ok(&self.values[&p])
    }

    /// Applies a JSON merge patch. `null` resets a field to its default.
    pub fn patch(&mut self, p: Partition, patch: &Value) -> Result<&Value> {
        let mut value = self.values[&p].clone();
        merge_patch(&mut value, patch);
        self.set(p, value)
    }

    pub fn reset(&mut self, p: Partition) -> &Value {
        self.set(p, p.default_value()).expect("defaults are valid")
    }

    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Writes changed partitions. Takes today's backup first if the day rolled over.
    pub fn flush(&mut self) -> Result<()> {
        if self.dirty.is_empty() {
            return Ok(());
        }
        backup::snapshot_if_due(&self.dir, self.clock.today())?;
        for p in std::mem::take(&mut self.dirty) {
            let json = serde_json::to_string_pretty(&self.values[&p])?;
            fsutil::write_atomic(&self.dir.join(p.file_name()), json.as_bytes())?;
        }
        Ok(())
    }

    /// Rewrites partition `p` in every kept backup through `f` (e.g. to blank a value that has
    /// since moved to the credential store). Unreadable files are left alone.
    pub fn scrub_backups(&self, p: Partition, f: impl Fn(&mut Value)) -> Result<()> {
        for date in backup::list(&self.dir)? {
            let path = backup::snapshot_dir(&self.dir, date).join(p.file_name());
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let Ok(mut value) = serde_json::from_str::<Value>(&text) else { continue };
            let before = value.clone();
            f(&mut value);
            if value != before {
                fsutil::write_atomic(&path, serde_json::to_string_pretty(&value)?.as_bytes())?;
            }
        }
        Ok(())
    }

    /// Partitions that can't leave this machine (accounts) are skipped.
    pub fn export(&self, partitions: &[Partition]) -> Bundle {
        let picked = partitions
            .iter()
            .filter(|p| p.is_portable())
            .map(|p| (p.key().to_owned(), self.values[p].clone()))
            .collect();
        Bundle::new(picked, self.clock.now())
    }

    /// Applies every partition in the bundle, or none if any of them is invalid.
    /// Partitions that can't come from another machine (accounts) are ignored.
    pub fn import(&mut self, bundle: Bundle) -> Result<Vec<Partition>> {
        self.apply(bundle, false)
    }

    fn apply(&mut self, bundle: Bundle, local: bool) -> Result<Vec<Partition>> {
        let mut checked = Vec::with_capacity(bundle.partitions.len());
        for (key, value) in bundle.partitions {
            let p = Partition::from_key(&key).ok_or(Error::UnknownPartition(key))?;
            if local || p.is_portable() {
                checked.push((p, p.normalize(value)?));
            }
        }
        for (p, value) in &checked {
            self.set(*p, value.clone())?;
        }
        Ok(checked.into_iter().map(|(p, _)| p).collect())
    }

    /// Snapshot dates, newest first.
    pub fn backups(&self) -> Result<Vec<Date>> {
        backup::list(&self.dir)
    }

    /// Loads every partition saved in that day's snapshot.
    pub fn restore_backup(&mut self, date: Date) -> Result<Vec<Partition>> {
        self.restore_from(backup::snapshot_dir(&self.dir, date), backup::date_key(date))
    }

    /// V1052-03 Saves the current state (secrets left out) before a restore. Returns its name.
    pub fn snapshot_now(&mut self) -> Result<Option<String>> {
        self.flush()?;
        backup::snapshot_now(&self.dir, self.clock.now())
    }

    /// Names of the "before restore" snapshots, newest first.
    pub fn snapshots(&self) -> Result<Vec<String>> {
        backup::list_snapshots(&self.dir)
    }

    pub fn restore_snapshot(&mut self, name: &str) -> Result<Vec<Partition>> {
        if !backup::is_snapshot_name(name) {
            return Err(Error::NoBackup(name.to_owned()));
        }
        self.restore_from(backup::named_dir(&self.dir, name), name.to_owned())
    }

    fn restore_from(&mut self, snapshot: PathBuf, label: String) -> Result<Vec<Partition>> {
        if !snapshot.is_dir() {
            return Err(Error::NoBackup(label));
        }
        let mut restored = BTreeMap::new();
        for p in Partition::ALL {
            if let Some(value) = read_partition(&snapshot.join(p.file_name()), p)? {
                restored.insert(p.key().to_owned(), value);
            }
        }
        self.apply(Bundle::new(restored, self.clock.now()), true)
    }
}

fn read_partition(path: &Path, p: Partition) -> Result<Option<Value>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&text)?;
    Ok(Some(p.normalize(value)?))
}

fn quarantine(path: &Path, clock: &dyn Clock) -> Result<()> {
    let stamp = clock
        .now()
        .format(format_description!("[year][month][day]-[hour][minute][second]"))
        .expect("date formats");
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    fs::rename(path, path.with_file_name(format!("{stem}.broken-{stamp}.json")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Appearance, Chat, General};
    use crate::testutil::{FakeClock, TempDir, day};
    use serde_json::json;

    fn open(dir: &Path, clock: &Arc<FakeClock>) -> (Store, LoadReport) {
        Store::open(dir, clock.clone()).unwrap()
    }

    #[test]
    fn starts_with_defaults_and_writes_nothing() {
        let tmp = TempDir::new("store-defaults");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (store, report) = open(tmp.path(), &clock);
        assert!(report.reset.is_empty());
        assert_eq!(store.read::<General>(Partition::General), General::default());
        assert!(!tmp.path().join("general.json").exists());
    }

    #[test]
    fn patch_flush_and_reload() {
        let tmp = TempDir::new("store-reload");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store
            .patch(Partition::General, &json!({"chatFontSize": 16}))
            .unwrap();
        assert!(store.is_dirty());
        store.flush().unwrap();
        assert!(!store.is_dirty());

        let (reloaded, _) = open(tmp.path(), &clock);
        assert_eq!(reloaded.read::<General>(Partition::General).chat_font_size, 16);
    }

    #[test]
    fn unchanged_values_are_not_marked_dirty() {
        let tmp = TempDir::new("store-clean");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store
            .patch(Partition::General, &json!({"tooltips": true}))
            .unwrap();
        assert!(!store.is_dirty());
    }

    #[test]
    fn null_resets_a_field_and_unknown_fields_are_dropped() {
        let tmp = TempDir::new("store-null");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store
            .patch(Partition::General, &json!({"chatFontSize": 20, "bogus": 1}))
            .unwrap();
        assert!(store.get(Partition::General).get("bogus").is_none());
        store
            .patch(Partition::General, &json!({"chatFontSize": null}))
            .unwrap();
        assert_eq!(store.read::<General>(Partition::General).chat_font_size, 13);
    }

    #[test]
    fn wrong_types_are_rejected_and_nothing_changes() {
        let tmp = TempDir::new("store-types");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        assert!(
            store
                .patch(Partition::General, &json!({"chatFontSize": "big"}))
                .is_err()
        );
        assert_eq!(store.read::<General>(Partition::General), General::default());
    }

    #[test]
    fn sanitizes_values() {
        let tmp = TempDir::new("store-sanitize");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store
            .patch(Partition::Appearance, &json!({"accent": "nope"}))
            .unwrap();
        store
            .patch(Partition::General, &json!({"chatFontSize": 200}))
            .unwrap();
        store
            .patch(
                Partition::Chat,
                &json!({"timeoutPresets": [600, 0, 60, 600, 99_999_999]}),
            )
            .unwrap();
        assert_eq!(store.read::<Appearance>(Partition::Appearance).accent, "#145aff");
        assert_eq!(store.read::<General>(Partition::General).chat_font_size, 32);
        assert_eq!(store.read::<Chat>(Partition::Chat).timeout_presets, vec![60, 600]);
    }

    #[test]
    fn broken_file_is_kept_aside_and_reported() {
        let tmp = TempDir::new("store-broken");
        fs::write(tmp.path().join("chat.json"), "{ not json").unwrap();
        let clock = Arc::new(FakeClock::at(day(9)));
        let (store, report) = open(tmp.path(), &clock);
        assert_eq!(report.reset, vec![Partition::Chat]);
        assert_eq!(store.read::<Chat>(Partition::Chat), Chat::default());
        assert!(!tmp.path().join("chat.json").exists());
        assert!(tmp.path().join("chat.broken-20261009-120000.json").exists());
    }

    #[test]
    fn import_is_all_or_nothing() {
        let tmp = TempDir::new("store-import");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        let bad = Bundle::from_json(
            &json!({"format": "cyex-settings", "version": 1, "exported": "",
                    "partitions": {"general": {"tooltips": false}, "nope": {}}})
            .to_string(),
        )
        .unwrap();
        assert!(matches!(store.import(bad), Err(Error::UnknownPartition(k)) if k == "nope"));
        assert!(store.read::<General>(Partition::General).tooltips);
    }

    #[test]
    fn accounts_never_leave_the_machine() {
        let tmp = TempDir::new("store-local");
        let (mut store, _) = open(tmp.path(), &Arc::new(FakeClock::at(day(9))));
        store
            .patch(
                Partition::Accounts,
                &json!({"list": [{"userId": "1", "login": "a"}]}),
            )
            .unwrap();
        let bundle = store.export(&[Partition::Accounts, Partition::General]);
        assert_eq!(bundle.partitions.keys().collect::<Vec<_>>(), ["general"]);
        let foreign = Bundle::from_json(
            &json!({"format": "cyex-settings", "version": 1, "exported": "",
                "partitions": {"accounts": {"list": []}}})
            .to_string(),
        )
        .unwrap();
        assert!(store.import(foreign).unwrap().is_empty());
        assert_eq!(store.get(Partition::Accounts)["list"][0]["login"], "a");
    }

    #[test]
    fn export_import_round_trip_between_stores() {
        let a_dir = TempDir::new("store-export-a");
        let b_dir = TempDir::new("store-export-b");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut a, _) = open(a_dir.path(), &clock);
        a.patch(
            Partition::Appearance,
            &json!({"theme": "light", "accent": "#ff0000"}),
        )
        .unwrap();
        a.patch(Partition::General, &json!({"tooltips": false})).unwrap();

        let code = a.export(&[Partition::Appearance]).to_share_code();
        let (mut b, _) = open(b_dir.path(), &clock);
        let imported = b.import(Bundle::from_share_code(&code).unwrap()).unwrap();
        assert_eq!(imported, vec![Partition::Appearance]);
        assert_eq!(b.get(Partition::Appearance), a.get(Partition::Appearance));
        assert!(
            b.read::<General>(Partition::General).tooltips,
            "other partitions untouched"
        );
    }

    #[test]
    fn backup_rotation_across_days_and_restore() {
        let tmp = TempDir::new("store-rotation");
        let clock = Arc::new(FakeClock::at(day(1)));
        let (mut store, _) = open(tmp.path(), &clock);

        for size in 10..=15u8 {
            store
                .patch(Partition::General, &json!({"chatFontSize": size}))
                .unwrap();
            store.flush().unwrap();
            clock.advance_days(1);
        }
        // Day 1 had no files yet; days 2-6 were snapshotted; the newest three remain.
        assert_eq!(store.backups().unwrap(), vec![day(6), day(5), day(4)]);

        // Day 4's snapshot holds the state saved on day 3.
        store.restore_backup(day(4)).unwrap();
        assert_eq!(store.read::<General>(Partition::General).chat_font_size, 12);
        assert!(matches!(store.restore_backup(day(1)), Err(Error::NoBackup(_))));
    }

    #[test]
    fn snapshot_before_restore_holds_unsaved_changes_and_restores() {
        let tmp = TempDir::new("store-snapshot");
        let clock = Arc::new(FakeClock::at(day(1)));
        let (mut store, _) = open(tmp.path(), &clock);
        store.patch(Partition::General, &json!({"chatFontSize": 11})).unwrap();
        let name = store.snapshot_now().unwrap().unwrap();
        assert_eq!(store.snapshots().unwrap(), vec![name.clone()]);
        assert!(store.backups().unwrap().iter().all(|d| *d == day(1)), "daily list ignores it");

        store.patch(Partition::General, &json!({"chatFontSize": 20})).unwrap();
        store.restore_snapshot(&name).unwrap();
        assert_eq!(store.read::<General>(Partition::General).chat_font_size, 11);
        assert!(matches!(store.restore_snapshot("../x"), Err(Error::NoBackup(_))));
    }
}
