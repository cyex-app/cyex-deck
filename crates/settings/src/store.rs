use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use time::Date;
use time::macros::format_description;

use crate::error::{Error, Result};
use crate::{cuts, rescue};
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
    /// V110-F82 Partitions whose file couldn't be opened (held by a scanner, a folder in the
    /// way): left untouched, defaults in memory, never written this session.
    pub blocked: Vec<Partition>,
    /// V110-A44 Partitions whose file was cut short but still held readable rows: those were
    /// kept (the file is set aside as for `reset`), with how many.
    pub rescued: Vec<(Partition, usize)>,
}

/// In-memory settings, written back to disk by [`Store::flush`].
pub struct Store {
    dir: PathBuf,
    values: BTreeMap<Partition, Value>,
    /// Partitions to write, each with the edit number of its latest change.
    dirty: BTreeMap<Partition, u64>,
    edits: u64,
    clock: Arc<dyn Clock>,
    old_mentions_unreadable: bool,
    /// V110-F82 Never written this session (see [`LoadReport::blocked`]).
    blocked: BTreeSet<Partition>,
}

/// V110-F82 How often a file that can't be opened is tried again at start, and the wait between.
const OPEN_WAITS: u32 = 5;
const OPEN_PAUSE: std::time::Duration = std::time::Duration::from_millis(200);

type ReadFn<'a> = &'a dyn Fn(&Path) -> std::io::Result<Vec<u8>>;

impl Store {
    /// V106-33 An old `mentions.json` (disk or restored snapshot) had the wrong shape and was skipped.
    pub fn old_mentions_unreadable(&self) -> bool {
        self.old_mentions_unreadable
    }

    /// Loads every partition from `dir`, creating it if needed. Takes today's backup
    /// before anything is changed.
    pub fn open(dir: impl Into<PathBuf>, clock: Arc<dyn Clock>) -> Result<(Self, LoadReport)> {
        Self::open_with(dir, clock, &|p: &Path| fs::read(p), OPEN_PAUSE)
    }

    /// [`Store::open`] with the file read and the retry pause given (tests).
    fn open_with(
        dir: impl Into<PathBuf>,
        clock: Arc<dyn Clock>,
        read_file: ReadFn,
        pause: std::time::Duration,
    ) -> Result<(Self, LoadReport)> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        let mut report = LoadReport::default();
        backup::sweep_partials(&dir);
        // V107-N10a A full disk or a locked backups folder no longer stops the app starting.
        if let Err(e) = backup::snapshot_if_due(&dir, clock.today()) {
            report.io_errors.push(format!("today's backup: {e}"));
        }

        let mut values = BTreeMap::new();
        let mut raw_highlights = None;
        // R19 Partitions that normalizing changed (a one-time move, a sanitize fix): written at flush.
        let mut changed = Vec::new();
        let mut blocked = BTreeSet::new();
        // V110-F82 One wait budget for the whole start, so a locked folder can't stall it.
        let mut waits = OPEN_WAITS;
        for p in Partition::ALL {
            let path = dir.join(p.file_name());
            let first = loop {
                let r = read_raw_with(&path, read_file);
                // V110-F82 A file that can't be opened is not a broken file: wait and try again.
                let retry = matches!(&r, Err(Error::Io(_))) && waits > 0 && !path.is_dir();
                if !retry {
                    break r;
                }
                waits -= 1;
                std::thread::sleep(pause);
            };
            if matches!(&first, Err(Error::Io(_))) {
                // V110-F82 Still shut: the file stays as it is, this partition runs on defaults
                // and is not written (no quarantine, no backup restore, not dirty).
                blocked.insert(p);
                report.blocked.push(p);
                let v = p.default_value();
                values.insert(p, if p == Partition::Chat { presence_off(v) } else { v });
                continue;
            }
            let read = first.and_then(|raw| {
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
            // V110-A44 A cut-off tabs file keeps every complete tab before the backup is tried.
            let rescued = match (&read, &salvaged) {
                (Err(_), None) if p == Partition::Tabs => {
                    fs::read_to_string(&path).ok().and_then(|text| rescue::tabs(&text))
                }
                _ => None,
            };
            let recovered = match (&read, &salvaged, &rescued) {
                (Err(_), None, None) => from_backups(&dir, p, clock.today()),
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
                Err(_) if rescued.is_some() => {
                    let (v, kept) = rescued.expect("checked above");
                    quarantine(&path, &*clock, &mut report);
                    report.rescued.push((p, kept));
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
        // V110-A44 Files set aside more than 30 days ago go.
        prune_old_broken(&dir, clock.today());
        let mut store = Self {
            dir,
            values,
            dirty: BTreeMap::new(),
            edits: 0,
            clock,
            old_mentions_unreadable: false,
            blocked,
        };
        for p in changed {
            store.mark_dirty(p);
        }
        if store.blocked.contains(&Partition::Highlights) {
            // V110-F82 Its file wasn't opened: no merge, nothing to write over it.
        } else if report.reset.contains(&Partition::Highlights) {
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
            self.mark_dirty(p);
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

    /// V110-F82 The partitions still waiting to be written.
    pub fn dirty_partitions(&self) -> Vec<Partition> {
        self.dirty.keys().copied().collect()
    }

    /// Writes `p` at the next flush even if unchanged: a one-time migration in `sanitize` ran on load.
    pub fn mark_dirty(&mut self, p: Partition) {
        // V110-F82 A file that couldn't be opened at start is never overwritten.
        if self.blocked.contains(&p) {
            return;
        }
        self.edits += 1;
        self.dirty.insert(p, self.edits);
    }

    /// Writes changed partitions. Takes today's backup first if the day rolled over.
    pub fn flush(&mut self) -> Result<()> {
        let Some(job) = self.prepare_flush() else { return Ok(()) };
        let done = job.run();
        self.commit_flush(done)
    }

    /// Copies out what a flush has to write, so the disk work can run without holding the
    /// store (see [`FlushJob::run`]). `None` when nothing is dirty.
    pub fn prepare_flush(&self) -> Option<FlushJob> {
        if self.dirty.is_empty() {
            return None;
        }
        let parts = self
            .dirty
            .iter()
            .map(|(&p, &edit)| (p, edit, serde_json::to_string_pretty(&self.values[&p]).map_err(Error::from)))
            .collect();
        Some(FlushJob { dir: self.dir.clone(), today: self.clock.today(), parts })
    }

    /// Takes in the result of a [`FlushJob`]. A part leaves `dirty` only if its file was
    /// written and it wasn't edited since `prepare_flush`. The first error is returned.
    pub fn commit_flush(&mut self, done: FlushDone) -> Result<()> {
        for (p, edit) in done.written {
            if self.dirty.get(&p) == Some(&edit) {
                self.dirty.remove(&p);
            }
        }
        done.first.map_or(Ok(()), Err)
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
            .map(|p| (p.key().to_owned(), without_private(*p, self.values[p].clone())))
            .collect();
        Bundle::new(picked, self.clock.now())
    }

    /// Applies every partition in the bundle, or none if any of them is invalid.
    /// Partitions that can't come from another machine (accounts) are ignored.
    pub fn import(&mut self, bundle: Bundle) -> Result<Vec<Partition>> {
        Ok(self.apply(bundle, false)?.0)
    }

    /// V110-A43 Like [`Store::import`], and the lines for lists that were longer than their cap
    /// ("Kept the first 500 commands."); those lists are imported up to the cap.
    pub fn import_noted(&mut self, bundle: Bundle) -> Result<(Vec<Partition>, Vec<String>)> {
        self.apply(bundle, false)
    }

    fn apply(&mut self, mut bundle: Bundle, local: bool) -> Result<(Vec<Partition>, Vec<String>)> {
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
        let mut notes = Vec::new();
        for (key, value) in bundle.partitions {
            let p = Partition::from_key(&key).ok_or(Error::UnknownPartition(key))?;
            if local || p.is_portable() {
                // V107-N5c What someone else exported never turns presence sharing on (or off).
                // V107-N10b Nor picks the folder the OBS status files are written to.
                // V107-H4b A restore keeps this PC's recent searches too (an old backup has none).
                                let value = if local && p != Partition::General { value } else { keep_local(p, value, &self.values[&p]) };
                // V110-F87 Nor arms anything that acts by itself (a restore keeps all of it).
                let value = if !local && p == Partition::Commands { disarm(value, &self.values[&p]) } else { value };
                let before = cuts::counts(p, &value);
                let value = p.normalize(value)?;
                if !local {
                    notes.extend(cuts::lines(p, &before, &cuts::counts(p, &value)));
                }
                checked.push((p, value));
            }
        }
        for (p, value) in &checked {
            self.set(*p, value.clone())?;
        }
        Ok((checked.into_iter().map(|(p, _)| p).collect(), notes))
    }

    /// V110-A44 The files set aside as broken, newest first.
    pub fn broken_files(&self) -> Vec<crate::BrokenFile> {
        crate::broken::list(&self.dir)
    }

    /// V110-A44 Puts a set-aside file back as its partition: read as is, a cut-off tabs file by
    /// its complete tabs, a file with some wrong values by the fields that still read; then
    /// checked like any restore. `name` has to be in [`Store::broken_files`], never a path. A
    /// file that can't be read changes nothing. The file stays where it is.
    pub fn restore_broken(&mut self, name: &str) -> Result<Partition> {
        let file = crate::broken::find(&self.dir, name).ok_or(Error::Unreadable)?;
        let p = file.partition;
        let path = self.dir.join(&file.name);
        let bytes = fs::read(&path).map_err(|_| Error::Unreadable)?;
        let value = match parse_bytes(&bytes).ok().and_then(|v| p.normalize(v).ok()) {
            Some(v) => v,
            None => {
                let text = String::from_utf8_lossy(&bytes);
                let rescued = (p == Partition::Tabs).then(|| rescue::tabs(&text).map(|(v, _)| v)).flatten();
                rescued.or_else(|| salvage(p, &path)).ok_or(Error::Unreadable)?
            }
        };
        let bundle = Bundle::new(BTreeMap::from([(p.key().to_owned(), value)]), self.clock.now());
        self.apply(bundle, true)?;
        Ok(p)
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

    /// V110-U55 When the last daily copy was made and why the latest try failed.
    pub fn backup_status(&self) -> backup::BackupStatus {
        backup::status(&self.dir)
    }

    /// V110-U55 Tries today's copy now (the hourly wait after a failure is skipped).
    pub fn backup_now(&mut self) -> Result<backup::BackupStatus> {
        self.flush()?;
        backup::retry_now(&self.dir, self.clock.today())?;
        Ok(backup::status(&self.dir))
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
        // V107-M(c) Accounts, plugins and the uploader belong to this PC: a restore leaves them be.
        for p in Partition::ALL.into_iter().filter(|p| p.is_portable()) {
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
        Ok(self.apply(Bundle::new(restored, self.clock.now()), true)?.0)
    }
}

/// A file as JSON, not normalized. `None` when it doesn't exist.
fn read_raw(path: &Path) -> Result<Option<Value>> {
    read_raw_with(path, &|p: &Path| fs::read(p))
}

/// V110-F82 A file that can't be opened (or a folder in its place) is an `Io` error, a file that
/// opens but doesn't parse is a `Json` one; the caller treats them differently.
fn read_raw_with(path: &Path, read_file: ReadFn) -> Result<Option<Value>> {
    match fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
        Ok(m) if m.is_dir() => {
            return Err(std::io::Error::other("a folder is where the file should be").into());
        }
        Ok(_) => {}
    }
    Ok(Some(parse_bytes(&read_file(path)?)?))
}

/// V110-F82 JSON from file bytes: a UTF-8 BOM is skipped (Windows PowerShell 5.1 writes one),
/// UTF-16 is decoded by its BOM. Anything else must be UTF-8.
pub(crate) fn parse_bytes(bytes: &[u8]) -> Result<Value> {
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]).or_else(|| bytes.strip_prefix(&[0xFE, 0xFF])) {
        let le = bytes[0] == 0xFF;
        let units: Vec<u16> = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| if le { u16::from_le_bytes(c) } else { u16::from_be_bytes(c) })
            .collect();
        let text = String::from_utf16(&units).map_err(|_| Error::NotSettings)?;
        return Ok(serde_json::from_str(&text)?);
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    Ok(serde_json::from_slice(bytes)?)
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
        Partition::General => &["recentSearches"],
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

/// V107-Mo Kept on this PC only: never in exports or share codes (backups still have them).
fn without_private(p: Partition, mut v: Value) -> Value {
    if let (Partition::General, Value::Object(map)) = (p, &mut v) {
        map.remove("recentSearches");
    }
    // V110-F87 Running `/timer -r` timers belong to this PC's session.
    if let (Partition::Commands, Value::Object(map)) = (p, &mut v) {
        map.remove(TIMERS);
    }
    v
}

const TIMERS: &str = "timers";
const AUTOMATIONS_ON: &str = "automationsOn";

/// V110-F87 An imported commands partition (a file or share code, maybe someone else's) never
/// brings saved timers (they would run commands later; this PC's stay) and never leaves
/// automations switched on, which post and run commands from this account: they stay on only
/// when they were on and the import changes nothing else in the partition (your own export,
/// read back). Otherwise the user switches them on again, having seen the rules.
fn disarm(mut incoming: Value, current: &Value) -> Value {
    let Value::Object(map) = &mut incoming else {
        return incoming;
    };
    match current.get(TIMERS) {
        Some(mine) => map.insert(TIMERS.to_owned(), mine.clone()),
        None => map.remove(TIMERS),
    };
    let was_on = current.get(AUTOMATIONS_ON) == Some(&Value::Bool(true));
    let wants_on = map.get(AUTOMATIONS_ON) == Some(&Value::Bool(true));
    let keep_on = was_on && wants_on && {
        // Run counts move on by themselves; everything else has to be what is here already.
        let rest = |v: &Value| {
            let mut v = v.clone();
            if let Some(map) = v.as_object_mut() {
                map.remove(AUTOMATIONS_ON);
                for rule in map.get_mut("automations").and_then(Value::as_array_mut).into_iter().flatten() {
                    if let Some(rule) = rule.as_object_mut() {
                        rule.remove("count");
                    }
                }
            }
            v
        };
        rest(&Value::Object(map.clone())) == rest(current)
    };
    map.insert(AUTOMATIONS_ON.to_owned(), Value::Bool(keep_on));
    flag_vars(map, current);
    incoming
}

/// V110-A40 The mark `/set var` puts in front of a value that came from someone else's message
/// (`cyex_chat::commands::taint::flag`; a test in that crate holds the two together).
const FOREIGN_MARK: char = '\u{FDD0}';

/// V110-A40 `value` as a variable stored from outside: the flag, then the plain text.
pub fn flag_var(value: &str) -> String {
    let plain: String = value
        .chars()
        .filter_map(|c| match c {
            FOREIGN_MARK => None,
            '\u{FDD1}' => Some('|'),
            '\u{FDD2}' => Some('>'),
            c => Some(c),
        })
        .collect();
    format!("{FOREIGN_MARK}{plain}")
}

/// V110-A40 A variable that arrives in an import is flagged "from outside", like one a chat
/// message set, so no command of yours runs it as a command. One this PC already holds with the
/// same text keeps what it has (your own export read back stays as it was).
fn flag_vars(map: &mut serde_json::Map<String, Value>, current: &Value) {
    let Some(Value::Object(vars)) = map.get_mut("vars") else { return };
    let plain = |s: &str| s.chars().filter(|c| *c != FOREIGN_MARK).collect::<String>();
    for (key, value) in vars.iter_mut() {
        let Some(text) = value.as_str() else { continue };
        if text.is_empty() {
            continue;
        }
        let mine = current.get("vars").and_then(|v| v.get(key)).and_then(Value::as_str);
        *value = match mine {
            Some(mine) if plain(mine) == plain(text) => Value::String(mine.to_owned()),
            _ => Value::String(flag_var(text)),
        };
    }
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
    prune_broken(path, BROKEN_KEEP);
}

/// V110-A44 How many days a `<name>.broken-*.json` file is kept.
const BROKEN_DAYS: i64 = 30;

/// V110-A44 Removes the `*.broken-<yyyymmdd>-<hhmmss>.json` files in `dir` older than [`BROKEN_DAYS`].
fn prune_old_broken(dir: &Path, today: Date) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let limit = today - time::Duration::days(BROKEN_DAYS);
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stamp) = name.rsplit_once(".broken-").and_then(|(_, s)| s.strip_suffix(".json")) else { continue };
        let Some(day) = stamp.get(..8).filter(|d| d.bytes().all(|b| b.is_ascii_digit())) else { continue };
        let parsed = Date::parse(day, format_description!("[year][month][day]"));
        if parsed.is_ok_and(|d| d < limit) && entry.path().is_file() {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// V110-F56 How many `<name>.broken-*.json` files stay per partition.
const BROKEN_KEEP: usize = 5;

/// Keeps the newest `keep` of `<stem>.broken-<stamp>.json` next to `path` (stamps sort by time).
fn prune_broken(path: &Path, keep: usize) {
    let (Some(dir), Some(stem)) = (path.parent(), path.file_stem()) else { return };
    let prefix = format!("{}.broken-", stem.to_string_lossy());
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut broken: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.starts_with(&prefix) && n.ends_with(".json")
        })
        .map(|e| e.path())
        .collect();
    broken.sort_unstable();
    let extra = broken.len().saturating_sub(keep);
    for old in broken.into_iter().take(extra) {
        let _ = fs::remove_file(old);
    }
}

/// V107-N10a The partition from the newest daily backup that reads cleanly: (raw, normalized, day).
/// V110-F56 A folder dated after `today` (the clock was once ahead) is skipped.
fn from_backups(dir: &Path, p: Partition, today: Date) -> Option<(Value, Value, Date)> {
    backup::list(dir).ok()?.into_iter().filter(|d| *d <= today).find_map(|date| {
        let raw = read_raw(&backup::snapshot_dir(dir, date).join(p.file_name())).ok()??;
        let v = p.normalize(raw.clone()).ok()?;
        Some((raw, v, date))
    })
}

/// The disk side of a flush: everything it needs, copied out of the [`Store`].
pub struct FlushJob {
    dir: PathBuf,
    today: Date,
    parts: Vec<(Partition, u64, Result<String>)>,
}

/// What a [`FlushJob`] got done, for [`Store::commit_flush`].
pub struct FlushDone {
    written: Vec<(Partition, u64)>,
    first: Option<Error>,
}

impl FlushJob {
    /// V107-A3 A missing backup must not stop saving, and one file that can't be written
    /// (held by a scanner, say) must not drop the others.
    pub fn run(self) -> FlushDone {
        let mut first = backup::snapshot_if_due(&self.dir, self.today).err();
        let mut written = Vec::new();
        for (p, edit, json) in self.parts {
            let res = json.and_then(|j| Ok(fsutil::write_atomic(&self.dir.join(p.file_name()), j.as_bytes())?));
            match res {
                Ok(()) => written.push((p, edit)),
                Err(e) => {
                    first.get_or_insert(e);
                }
            }
        }
        FlushDone { written, first }
    }
}

#[cfg(test)]
mod tests;
