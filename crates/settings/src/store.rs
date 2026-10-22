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
    /// V107-N10a Partitions whose file was unreadable and came back whole from the newest
    /// daily backup that still reads (the broken file is kept aside as for `reset`).
    pub restored: Vec<(Partition, Date)>,
    /// V107-N10a What couldn't be done on disk (today's backup, setting a broken file aside).
    /// Logged; the start goes on, like a failed backup at flush (V107-A3).
    pub io_errors: Vec<String>,
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
        let mut report = LoadReport::default();
        // V107-N10a A full disk or a locked backups folder no longer stops the app starting.
        if let Err(e) = backup::snapshot_if_due(&dir, clock.today()) {
            report.io_errors.push(format!("today's backup: {e}"));
        }

        let mut values = BTreeMap::new();
        let mut raw_highlights = None;
        // R19 Partitions that normalizing changed (a one-time move, a sanitize fix): written at flush.
        let mut changed = Vec::new();
        for p in Partition::ALL {
            let path = dir.join(p.file_name());
            let read = read_raw(&path).and_then(|raw| {
                if p == Partition::Highlights {
                    raw_highlights.clone_from(&raw);
                }
                let Some(raw) = raw else { return Ok(None) };
                let v = p.normalize(raw.clone())?;
                if v != raw {
                    changed.push(p);
                }
                Ok(Some(v))
            });
            // t5 One wrong value (a hand edit, an old build) used to reset the whole file,
            // nicknames and the presence choice included. Keep every field that still reads.
            // Highlights stay whole: their old-mentions merge reads the raw file.
            let salvaged = match &read {
                Err(_) if p != Partition::Highlights => salvage(p, &path),
                _ => None,
            };
            // V107-N10a Nothing in it reads (cut short, empty, not JSON): the newest backup
            // that still reads, before defaults.
            let recovered = match (&read, &salvaged) {
                (Err(_), None) => from_backups(&dir, p),
                _ => None,
            };
            let value = match read {
                Ok(Some(v)) => v,
                Ok(None) => p.default_value(),
                Err(_) if salvaged.is_some() => {
                    let v = salvaged.expect("checked above");
                    quarantine(&path, &*clock, &mut report);
                    report.reset.push(p);
                    changed.push(p);
                    v
                }
                Err(_) if recovered.is_some() => {
                    let (raw, v, date) = recovered.expect("checked above");
                    if p == Partition::Highlights {
                        raw_highlights = Some(raw);
                    }
                    quarantine(&path, &*clock, &mut report);
                    report.restored.push((p, date));
                    changed.push(p);
                    v
                }
                Err(_) => {
                    if p == Partition::Highlights {
                        raw_highlights = None;
                    }
                    quarantine(&path, &*clock, &mut report);
                    report.reset.push(p);
                    if p == Partition::Chat {
                        // V107-R11 Written now, or the next start reads no file and turns it on.
                        changed.push(p);
                        presence_off(p.default_value())
                    } else {
                        p.default_value()
                    }
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
        for p in changed {
            store.mark_dirty(p);
        }
        if report.reset.contains(&Partition::Highlights) {
            // V106-33 Quarantined: the flagged defaults are written. R7 The old mention words
            // are user data, so mentions.json is folded into them.
            store.mark_dirty(Partition::Highlights);
            store.merge_old_mentions(None, &mut report)?;
        } else {
            store.merge_old_mentions(raw_highlights, &mut report)?;
        }
        store.old_mentions_unreadable = report.old_mentions_unreadable;
        Ok((store, report))
    }

    /// V106-33 Once: folds `mentions.json` and pre-v1.06 highlights into Highlights. The
    /// day's backup was taken first; `mentions.json` itself is never touched.
    fn merge_old_mentions(&mut self, raw_highlights: Option<Value>, report: &mut LoadReport) -> Result<()> {
        // R20 Read only when a merge is due, so a leftover broken file isn't flagged every start.
        let file = self.dir.join(legacy::mentions::FILE);
        if !needs_fold(raw_highlights.as_ref(), file.is_file()) {
            return Ok(());
        }
        let mentions = read_raw(&file).unwrap_or_else(|_| {
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
        // V107-A3 A missing backup must not stop saving, and one file that can't be written
        // (held by a scanner, say) must not drop the others: a part leaves `dirty` only once
        // its file is on disk. The first error is returned.
        let mut first = backup::snapshot_if_due(&self.dir, self.clock.today()).err();
        let todo: Vec<Partition> = self.dirty.iter().copied().collect();
        for p in todo {
            match self.write(p) {
                Ok(()) => {
                    self.dirty.remove(&p);
                }
                Err(e) => {
                    first.get_or_insert(e);
                }
            }
        }
        first.map_or(Ok(()), Err)
    }

    fn write(&self, p: Partition) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.values[&p])?;
        fsutil::write_atomic(&self.dir.join(p.file_name()), json.as_bytes())?;
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
                // V107-N5c What someone else exported never turns presence sharing on (or off).
                // V107-N10b Nor picks the folder the OBS status files are written to.
                let value = if local { value } else { keep_local(p, value, &self.values[&p]) };
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
        match read_raw(&snapshot.join(legacy::mentions::FILE)) {
            Ok(Some(old)) => {
                restored.insert(legacy::mentions::KEY.to_owned(), old);
            }
            Ok(None) => {}
            // R20 Flagged the same way `open` does.
            Err(_) => self.old_mentions_unreadable = true,
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

/// t5 The fields of a JSON object file that the schema still accepts, one top-level key at a
/// time; the rest fall back to defaults. `None` when the file isn't a JSON object at all.
fn salvage(p: Partition, path: &Path) -> Option<Value> {
    let Ok(Some(Value::Object(raw))) = read_raw(path) else { return None };
    let mut kept = serde_json::Map::new();
    for (key, value) in raw {
        kept.insert(key.clone(), value);
        if p.normalize(Value::Object(kept.clone())).is_err() {
            kept.remove(&key);
        }
    }
    let v = p.normalize(Value::Object(kept.clone())).ok()?;
    // V107-R11 A salvaged chat file keeps its own presence choice; without one it stays off.
    if p == Partition::Chat && !kept.contains_key(SHARE_PRESENCE) {
        return Some(presence_off(v));
    }
    Some(v)
}

const SHARE_PRESENCE: &str = "sharePresence";
const PRESENCE_ON_ONCE: &str = "presenceOnOnce";

/// V107-N5c An imported chat partition keeps this PC's presence choice: it shares which channel
/// you are in, so a share code or file can't change it. V107-N10b The stream manager keeps its
/// OBS status files (on/off and folder): a share code could otherwise pick where they're written.
fn keep_local(p: Partition, mut incoming: Value, current: &Value) -> Value {
    let keys: &[&str] = match p {
        Partition::Chat => &[SHARE_PRESENCE, PRESENCE_ON_ONCE],
        Partition::StreamManager => &["obs"],
        _ => return incoming,
    };
    if let (Value::Object(map), Value::Object(mine)) = (&mut incoming, current) {
        for &key in keys {
            match mine.get(key) {
                Some(v) => map.insert(key.to_owned(), v.clone()),
                None => map.remove(key),
            };
        }
    }
    incoming
}

/// V107-R11 A broken or unreadable chat file never turns presence on (it shares which channel
/// you're in). The one-time turn-on counts as done, so it stays off.
fn presence_off(mut v: Value) -> Value {
    if let Value::Object(map) = &mut v {
        map.insert(SHARE_PRESENCE.to_owned(), Value::Bool(false));
        map.insert("presenceOnOnce".to_owned(), Value::Bool(true));
    }
    v
}

/// Sets a broken file aside as `<name>.broken-<stamp>.json`. V107-N10a A rename that fails
/// (a locked file) is reported, not fatal: the start goes on and the next save replaces it.
fn quarantine(path: &Path, clock: &dyn Clock, report: &mut LoadReport) {
    let stamp = clock
        .now()
        .format(format_description!("[year][month][day]-[hour][minute][second]"))
        .expect("date formats");
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    if let Err(e) = fs::rename(path, path.with_file_name(format!("{stem}.broken-{stamp}.json"))) {
        report.io_errors.push(format!("setting {} aside: {e}", path.display()));
    }
}

/// V107-N10a The partition from the newest daily backup that reads cleanly: (raw, normalized, day).
fn from_backups(dir: &Path, p: Partition) -> Option<(Value, Value, Date)> {
    backup::list(dir).ok()?.into_iter().find_map(|date| {
        let raw = read_raw(&backup::snapshot_dir(dir, date).join(p.file_name())).ok()??;
        let v = p.normalize(raw.clone()).ok()?;
        Some((raw, v, date))
    })
}

#[cfg(test)]
mod tests;
