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
use crate::legacy::{self, MergeReport, Source, fold, needs_fold};
use crate::schema::Highlights;
use crate::{Clock, Partition, Schema, backup, fsutil};

/// What happened while loading, for the UI to report.
#[derive(Debug, Default)]
pub struct LoadReport {
    /// Partitions whose file was unreadable. The file was renamed to
    /// `<name>.broken-<stamp>.json` and defaults were used.
    pub reset: Vec<Partition>,
    /// V106-33 The old mentions were merged into Highlights on this start.
    pub merged: Option<MergeReport>,
    /// V106-33 `mentions.json` couldn’t be read: left as is, not merged.
    pub old_mentions_unreadable: bool,
}

/// In-memory settings, written back to disk by [`Store::flush`].
pub struct Store {
    dir: PathBuf,
    values: BTreeMap<Partition, Value>,
    dirty: BTreeSet<Partition>,
    clock: Arc<dyn Clock>,
    old_mentions_unreadable: bool,
}

impl Store {
    /// V106-33 An old `mentions.json` (disk or restored snapshot) had the wrong shape and was skipped.
    pub fn old_mentions_unreadable(&self) -> bool {
        self.old_mentions_unreadable
    }

    /// Loads every partition from `dir`, creating it if needed. Takes today's backup
    /// before anything is changed.
    pub fn open(dir: impl Into<PathBuf>, clock: Arc<dyn Clock>) -> Result<(Self, LoadReport)> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        backup::snapshot_if_due(&dir, clock.today())?;

        let mut report = LoadReport::default();
        let mut values = BTreeMap::new();
        let mut raw_highlights = None;
        for p in Partition::ALL {
            let path = dir.join(p.file_name());
            let read = read_raw(&path).and_then(|raw| {
                if p == Partition::Highlights {
                    raw_highlights.clone_from(&raw);
                }
                Ok(raw.map(|v| p.normalize(v)).transpose()?)
            });
            let value = match read {
                Ok(Some(v)) => v,
                Ok(None) => p.default_value(),
                Err(_) => {
                    if p == Partition::Highlights {
                        raw_highlights = None;
                    }
                    quarantine(&path, &*clock)?;
                    report.reset.push(p);
                    p.default_value()
                }
            };
            values.insert(p, value);
        }
        let mut store = Self {
            dir,
            values,
            dirty: BTreeSet::new(),
            clock,
            old_mentions_unreadable: false,
        };
        if report.reset.contains(&Partition::Highlights) {
            // V106-33 Quarantined: the flagged defaults stand, a stale mentions.json isn't folded back in.
            store.mark_dirty(Partition::Highlights);
        } else {
            store.merge_old_mentions(raw_highlights, &mut report)?;
        }
        store.old_mentions_unreadable = report.old_mentions_unreadable;
        Ok((store, report))
    }

    /// V106-33 Once: folds `mentions.json` and pre-v1.06 highlights into Highlights. The
    /// day's backup was taken first; `mentions.json` itself is never touched.
    fn merge_old_mentions(&mut self, raw_highlights: Option<Value>, report: &mut LoadReport) -> Result<()> {
        let mentions = read_raw(&self.dir.join(legacy::mentions::FILE)).unwrap_or_else(|_| {
            report.old_mentions_unreadable = true;
            None
        });
        if !needs_fold(raw_highlights.as_ref(), mentions.is_some()) {
            return Ok(());
        }
        let current = Highlights::default();
        let folded = match fold(raw_highlights.clone(), mentions, &current, Source::Disk) {
            Ok(f) => f,
            Err(_) => {
                // The old file has the wrong shape: merge as if it weren't there.
                report.old_mentions_unreadable = true;
                if raw_highlights.is_none() {
                    return Ok(());
                }
                fold(raw_highlights, None, &current, Source::Disk)?
            }
        };
        let (h, merged) = folded;
        self.values.insert(Partition::Highlights, serde_json::to_value(h)?);
        self.mark_dirty(Partition::Highlights);
        report.merged = Some(merged);
        Ok(())
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

    /// Writes `p` at the next flush even if unchanged: a one-time migration in `sanitize` ran on load.
    pub fn mark_dirty(&mut self, p: Partition) {
        self.dirty.insert(p);
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

    fn apply(&mut self, mut bundle: Bundle, local: bool) -> Result<Vec<Partition>> {
        // V106-33 Old exports, share codes and backups: fold their mentions into Highlights.
        let old = bundle.partitions.remove(legacy::mentions::KEY);
        let raw = bundle.partitions.get(Partition::Highlights.key()).cloned();
        if needs_fold(raw.as_ref(), old.is_some()) {
            let source = if local { Source::Disk } else { Source::Bundle };
            let current = self.read(Partition::Highlights);
            let folded = match fold(raw.clone(), old, &current, source) {
                Ok((h, _)) => Some(h),
                Err(_) if local => {
                    // A snapshot's old mentions have the wrong shape: restore as if absent.
                    self.old_mentions_unreadable = true;
                    match raw {
                        Some(_) => Some(fold(raw, None, &current, source)?.0),
                        None => None,
                    }
                }
                Err(e) => return Err(e.into()),
            };
            if let Some(h) = folded {
                bundle
                    .partitions
                    .insert(Partition::Highlights.key().to_owned(), serde_json::to_value(h)?);
            }
        }
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
            if let Some(value) = read_raw(&snapshot.join(p.file_name()))? {
                restored.insert(p.key().to_owned(), value);
            }
        }
        // V106-33 A merge-day backup holds the old mentions. Ignored once highlights are merged.
        if let Ok(Some(old)) = read_raw(&snapshot.join(legacy::mentions::FILE)) {
            restored.insert(legacy::mentions::KEY.to_owned(), old);
        }
        self.apply(Bundle::new(restored, self.clock.now()), true)
    }
}

/// A file as JSON, not normalized. `None` when it doesn't exist.
fn read_raw(path: &Path) -> Result<Option<Value>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&text)?))
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
    fn mark_dirty_writes_at_flush() {
        let tmp = TempDir::new("store-mark-dirty");
        let clock = Arc::new(FakeClock::at(day(9)));
        let (mut store, _) = open(tmp.path(), &clock);
        store.mark_dirty(Partition::Chat);
        store.flush().unwrap();
        assert!(tmp.path().join("chat.json").exists());
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
    // V106-33 The old mentions fold into Highlights.
    mod merge {
        use super::*;
        use crate::schema::Highlights;

        const OLD_HIGHLIGHTS: &str = r##"{"mention": {"enabled": true, "color": "#ff0000ff"}, "lookalikes": true,
            "phrases": [{"pattern": "hello"}, {"pattern": "a+", "regex": true}], "users": [{"login": "bob"}]}"##;
        const OLD_MENTIONS: &str = r##"{"me": {"sound": false, "color": "#00ff00"},
            "list": [{"pattern": "cyex", "flavor": "python", "regex": true}, {"pattern": "hello"}],
            "global": true, "globalList": [{"pattern": "deck", "soundFile": "C:\\s\\g.wav"}]}"##;

        fn write(dir: &Path, name: &str, text: &str) {
            fs::write(dir.join(name), text).unwrap();
        }

        fn highlights(store: &Store) -> Highlights {
            store.read(Partition::Highlights)
        }

        fn patterns(list: &[crate::schema::PhraseRule]) -> Vec<&str> {
            list.iter().map(|p| p.pattern.as_str()).collect()
        }

        fn bundle(partitions: Value) -> Bundle {
            Bundle::from_json(
                &json!({"format": "cyex-settings", "version": 1, "exported": "", "partitions": partitions})
                    .to_string(),
            )
            .unwrap()
        }

        fn v105(dir: &Path) {
            write(dir, "highlights.json", OLD_HIGHLIGHTS);
            write(dir, "mentions.json", OLD_MENTIONS);
        }

        fn file(dir: &Path, name: &str) -> String {
            fs::read_to_string(dir.join(name)).unwrap()
        }

        #[test]
        fn fresh_user_merges_nothing() {
            let tmp = TempDir::new("merge-fresh");
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, report) = open(tmp.path(), &clock);
            assert!(report.merged.is_none() && !report.old_mentions_unreadable);
            let h = highlights(&store);
            assert!(h.me.alert.sound && !h.replies.alert.sound);
            assert!(h.me.alert.show && h.replies.alert.show);
            assert!(h.mentions_merged && h.duplicates_asked);
            store.flush().unwrap();
            assert!(!tmp.path().join("highlights.json").exists(), "nothing written");
        }

        #[test]
        fn v105_folder_is_merged_once() {
            let tmp = TempDir::new("merge-v105");
            v105(tmp.path());
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, report) = open(tmp.path(), &clock);
            let r = report.merged.unwrap();
            assert_eq!((r.from_mentions, r.from_highlights, r.global, r.duplicates, r.cut), (2, 2, 1, 1, 0));
            let h = highlights(&store);
            assert_eq!(patterns(&h.phrases), ["cyex", "hello", "hello", "a+"]);
            assert_eq!(h.me.alert.color, "#00ff00");
            assert!(h.me.alert.sound && !h.replies.alert.sound, "V106-09");
            assert_eq!(h.users[0].login, "bob");
            assert!(h.global && h.global_list[0].alert.sound_file == "C:\\s\\g.wav");
            assert!(!h.duplicates_asked);
            store.flush().unwrap();
            let saved: Value = serde_json::from_str(&file(tmp.path(), "highlights.json")).unwrap();
            assert_eq!(saved["mentionsMerged"], true);
            assert_eq!(file(tmp.path(), "mentions.json"), OLD_MENTIONS, "untouched");

            // Runs once.
            let (again, report) = open(tmp.path(), &clock);
            assert!(report.merged.is_none());
            assert_eq!(highlights(&again), h);
        }

        #[test]
        fn mentions_only_file_is_merged() {
            let tmp = TempDir::new("merge-mentions-only");
            write(tmp.path(), "mentions.json", OLD_MENTIONS);
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, report) = open(tmp.path(), &clock);
            assert_eq!(report.merged.unwrap().from_mentions, 2);
            let h = highlights(&store);
            assert_eq!(patterns(&h.phrases), ["cyex", "hello"]);
            assert_eq!(h.phrases[0].flavor, crate::schema::RegexFlavor::Python);
            assert!(h.global && h.me.alert.color == "#00ff00");
            store.flush().unwrap();
            let saved: Value = serde_json::from_str(&file(tmp.path(), "highlights.json")).unwrap();
            assert_eq!(saved["mentionsMerged"], true);
            assert_eq!(file(tmp.path(), "mentions.json"), OLD_MENTIONS);
        }

        #[test]
        fn unreadable_mentions_file_is_left_as_is() {
            let tmp = TempDir::new("merge-unreadable");
            write(tmp.path(), "highlights.json", OLD_HIGHLIGHTS);
            write(tmp.path(), "mentions.json", "{ nope");
            let clock = Arc::new(FakeClock::at(day(9)));
            let (_, report) = open(tmp.path(), &clock);
            assert!(report.old_mentions_unreadable);
            assert_eq!(report.merged.unwrap().from_highlights, 2);
            assert_eq!(file(tmp.path(), "mentions.json"), "{ nope");
        }

        #[test]
        fn reset_never_merges_again() {
            let tmp = TempDir::new("merge-reset");
            v105(tmp.path());
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, _) = open(tmp.path(), &clock);
            store.flush().unwrap();
            store.reset(Partition::Highlights);
            store.flush().unwrap();
            let (store, report) = open(tmp.path(), &clock);
            assert!(report.merged.is_none());
            assert_eq!(highlights(&store), Highlights::default());
        }

        #[test]
        fn backup_before_merge() {
            let tmp = TempDir::new("merge-backup");
            v105(tmp.path());
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, _) = open(tmp.path(), &clock);
            store.flush().unwrap();
            let snap = backup::snapshot_dir(tmp.path(), day(9));
            assert_eq!(file(&snap, "highlights.json"), OLD_HIGHLIGHTS);
            assert_eq!(file(&snap, "mentions.json"), OLD_MENTIONS);
        }

        #[test]
        fn name_and_replies_always_in_mentions_tab() {
            let tmp = TempDir::new("merge-show");
            write(tmp.path(), "mentions.json", r#"{"me": {"show": false}}"#);
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, _) = open(tmp.path(), &clock);
            assert!(highlights(&store).me.alert.show && highlights(&store).replies.alert.show);
            store
                .patch(Partition::Highlights, &json!({"me": {"show": false}, "replies": {"show": false}}))
                .unwrap();
            assert!(highlights(&store).me.alert.show && highlights(&store).replies.alert.show);
        }

        #[test]
        fn import_v105_bundle() {
            let tmp = TempDir::new("merge-import-v105");
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, _) = open(tmp.path(), &clock);
            let parsed = |t: &str| serde_json::from_str::<Value>(t).unwrap();
            let got = store
                .import(bundle(json!({"highlights": parsed(OLD_HIGHLIGHTS), "mentions": parsed(OLD_MENTIONS)})))
                .unwrap();
            assert_eq!(got, vec![Partition::Highlights]);
            let h = highlights(&store);
            assert_eq!(patterns(&h.phrases), ["cyex", "hello", "hello", "a+"]);
            assert!(h.global && h.me.alert.sound && h.me.alert.color == "#00ff00");
        }

        #[test]
        fn import_mentions_only_bundle() {
            let tmp = TempDir::new("merge-import-mentions");
            v105(tmp.path());
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, _) = open(tmp.path(), &clock);
            let got = store
                .import(bundle(json!({"mentions": {"me": {"color": "#abcdef"},
                    "list": [{"pattern": "cyex", "flavor": "python", "regex": true}, {"pattern": "new"}]}})))
                .unwrap();
            assert_eq!(got, vec![Partition::Highlights]);
            let h = highlights(&store);
            assert_eq!(patterns(&h.phrases), ["new", "cyex", "hello", "hello", "a+"], "exact copies skipped");
            assert_eq!(h.me.alert.color, "#abcdef");
            assert!(!h.global && h.global_list.is_empty(), "global replaced");
            assert_eq!(h.users[0].login, "bob", "the rest kept");
        }

        #[test]
        fn import_highlights_only_old_bundle() {
            let tmp = TempDir::new("merge-import-highlights");
            v105(tmp.path());
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, _) = open(tmp.path(), &clock);
            store.patch(Partition::Highlights, &json!({"me": {"sound": false}})).unwrap();
            let before = highlights(&store);
            store
                .import(bundle(
                    json!({"highlights": {"phrases": [{"pattern": "x"}, {"pattern": "y+", "regex": true}]}}),
                ))
                .unwrap();
            let h = highlights(&store);
            assert_eq!(patterns(&h.phrases), ["x", "y+"]);
            assert!(h.phrases[0].whole_word && !h.phrases[1].whole_word);
            assert_eq!((&h.me, &h.replies), (&before.me, &before.replies));
            assert_eq!((h.global, &h.global_list), (before.global, &before.global_list));
        }

        #[test]
        fn share_code_old_bundle() {
            let tmp = TempDir::new("merge-share");
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, _) = open(tmp.path(), &clock);
            let old = serde_json::from_str::<Value>(OLD_MENTIONS).unwrap();
            let code = bundle(json!({ "mentions": old })).to_share_code();
            store.import(Bundle::from_share_code(&code).unwrap()).unwrap();
            assert_eq!(patterns(&highlights(&store).phrases), ["cyex", "hello"]);
        }

        #[test]
        fn restore_v105_daily_backup() {
            let tmp = TempDir::new("merge-restore-v105");
            v105(tmp.path());
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, _) = open(tmp.path(), &clock);
            let merged = highlights(&store);
            store.reset(Partition::Highlights);
            store.restore_backup(day(9)).unwrap();
            assert_eq!(highlights(&store), merged);
        }

        #[test]
        fn restore_v106_snapshot_ignores_stale_mentions() {
            let tmp = TempDir::new("merge-restore-v106");
            v105(tmp.path());
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, _) = open(tmp.path(), &clock);
            store.flush().unwrap();
            let merged = highlights(&store);
            let name = store.snapshot_now().unwrap().unwrap();
            assert!(backup::named_dir(tmp.path(), &name).join("mentions.json").is_file());
            store.reset(Partition::Highlights);
            store.restore_snapshot(&name).unwrap();
            assert_eq!(highlights(&store), merged, "no second merge, no doubled words");
        }

        #[test]
        fn corrupt_highlights_is_not_refolded() {
            let tmp = TempDir::new("merge-corrupt-highlights");
            write(tmp.path(), "highlights.json", "{ nope");
            write(tmp.path(), "mentions.json", OLD_MENTIONS);
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, report) = open(tmp.path(), &clock);
            assert_eq!(report.reset, vec![Partition::Highlights]);
            assert!(report.merged.is_none());
            assert_eq!(highlights(&store), Highlights::default());
            store.flush().unwrap();
            let saved: Value = serde_json::from_str(&file(tmp.path(), "highlights.json")).unwrap();
            assert_eq!(saved["mentionsMerged"], true);
            let (again, report) = open(tmp.path(), &clock);
            assert!(report.merged.is_none());
            assert_eq!(highlights(&again), Highlights::default());
        }

        #[test]
        fn restore_with_misshaped_mentions() {
            let tmp = TempDir::new("merge-restore-bad-mentions");
            v105(tmp.path());
            let clock = Arc::new(FakeClock::at(day(9)));
            let (mut store, _) = open(tmp.path(), &clock);
            store.flush().unwrap();
            let merged = highlights(&store);
            let snap = backup::snapshot_dir(tmp.path(), day(9));
            write(&snap, "highlights.json", OLD_HIGHLIGHTS);
            write(&snap, "mentions.json", "[1, 2]");
            store.reset(Partition::Highlights);
            assert!(!store.old_mentions_unreadable());
            store.restore_backup(day(9)).unwrap();
            assert!(store.old_mentions_unreadable());
            assert_eq!(patterns(&highlights(&store).phrases), ["hello", "a+"]);
            assert_ne!(highlights(&store), merged);
        }

        #[test]
        fn export_import_round_trip() {
            let a_dir = TempDir::new("merge-export-a");
            let b_dir = TempDir::new("merge-export-b");
            v105(a_dir.path());
            let clock = Arc::new(FakeClock::at(day(9)));
            let (a, _) = open(a_dir.path(), &clock);
            let out = a.export(&Partition::ALL);
            assert!(!out.partitions.contains_key("mentions"));
            let (mut b, _) = open(b_dir.path(), &clock);
            b.import(Bundle::from_json(&out.to_json()).unwrap()).unwrap();
            assert_eq!(highlights(&b), highlights(&a));
        }
    }
}
