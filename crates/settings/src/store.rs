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
mod tests;
