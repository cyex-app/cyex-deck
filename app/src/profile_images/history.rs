//! V107-R19c The last images changed in the app, per account and kind, for switching back:
//! `local/profile-backup/<userId>/history/<kind>-<unix ms>-<hash>.<ext>`, beside a small
//! `.thumb.png` and, once Twitch showed it, a `.url` with the address it got. Newest first,
//! identical files once (the newest stamp wins), older ones pruned past the cap. The original
//! backup (`backup.rs`) lives one folder up and is never touched here.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const HASH_LEN: usize = 16;

/// One saved image. `id` is `<unix ms>-<hash>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    pub id: String,
    pub path: PathBuf,
    pub thumb: Option<PathBuf>,
    pub url: Option<PathBuf>,
}

impl Saved {
    fn stamp(&self) -> u64 {
        self.id.split('-').next().and_then(|s| s.parse().ok()).unwrap_or(0)
    }

    fn hash(&self) -> &str {
        self.id.split_once('-').map_or("", |(_, h)| h)
    }
}

pub fn dir(backup_dir: &Path) -> PathBuf {
    backup_dir.join("history")
}

/// `<ms>-<16 hex>` and nothing else: ids come from the window, never a path.
pub fn valid_id(id: &str) -> bool {
    let Some((ms, hash)) = id.split_once('-') else { return false };
    !ms.is_empty()
        && ms.bytes().all(|b| b.is_ascii_digit())
        && hash.len() == HASH_LEN
        && hash.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn hash(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d.iter().take(HASH_LEN / 2).map(|b| format!("{b:02x}")).collect()
}

/// `(id, rest)` from `avatar-12-<hash>.png`: rest is `png`, `thumb.png`, `url` or `part`.
fn split<'a>(name: &'a str, kind: &str) -> Option<(&'a str, &'a str)> {
    let rest = name.strip_prefix(kind)?.strip_prefix('-')?;
    let (id, ext) = rest.split_once('.')?;
    valid_id(id).then_some((id, ext))
}

/// Every complete entry of a kind, newest first.
pub fn list(dir: &Path, kind: &str) -> Vec<Saved> {
    let Ok(read) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Saved> = Vec::new();
    let mut side: Vec<(String, &'static str, PathBuf)> = Vec::new();
    for e in read.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some((id, ext)) = split(&name, kind) else { continue };
        match ext {
            "png" | "jpg" | "gif" => out.push(Saved { id: id.to_owned(), path: e.path(), thumb: None, url: None }),
            "thumb.png" => side.push((id.to_owned(), "thumb", e.path())),
            "url" => side.push((id.to_owned(), "url", e.path())),
            _ => {}
        }
    }
    for (id, what, path) in side {
        if let Some(s) = out.iter_mut().find(|s| s.id == id) {
            if what == "thumb" {
                s.thumb = Some(path);
            } else {
                s.url = Some(path);
            }
        }
    }
    out.sort_by(|a, b| b.stamp().cmp(&a.stamp()).then_with(|| b.id.cmp(&a.id)));
    out
}

pub fn find(dir: &Path, kind: &str, id: &str) -> Option<Saved> {
    valid_id(id).then(|| list(dir, kind).into_iter().find(|s| s.id == id)).flatten()
}

/// Every file of one entry, side files and leftovers included.
fn drop_id(dir: &Path, kind: &str, id: &str) {
    let Ok(read) = fs::read_dir(dir) else { return };
    for e in read.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if split(&name, kind).is_some_and(|(i, _)| i == id) {
            let _ = fs::remove_file(e.path());
        }
    }
}

/// Saves `bytes` as the newest entry and returns its id. The same image saved before moves
/// to the front; entries past `max` go.
pub fn add(dir: &Path, kind: &str, ext: &str, now_ms: u64, bytes: &[u8], thumb: Option<&[u8]>, max: usize) -> std::io::Result<String> {
    fs::create_dir_all(dir)?;
    let h = hash(bytes);
    for old in list(dir, kind).iter().filter(|s| s.hash() == h) {
        drop_id(dir, kind, &old.id);
    }
    let id = format!("{now_ms}-{h}");
    if let Some(t) = thumb {
        fs::write(dir.join(format!("{kind}-{id}.thumb.png")), t)?;
    }
    let tmp = dir.join(format!("{kind}-{id}.part"));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, dir.join(format!("{kind}-{id}.{ext}")))?;
    for old in list(dir, kind).iter().skip(max) {
        drop_id(dir, kind, &old.id);
    }
    Ok(id)
}

/// The address Twitch gave the entry, so the window can mark the one in use.
pub fn set_url(dir: &Path, kind: &str, id: &str, url: &str) {
    if valid_id(id) {
        let _ = fs::write(dir.join(format!("{kind}-{id}.url")), url);
    }
}

pub fn url_of(s: &Saved) -> Option<String> {
    fs::read_to_string(s.url.as_ref()?).ok().map(|u| u.trim().to_owned())
}

/// False when there was no such entry.
pub fn remove(dir: &Path, kind: &str, id: &str) -> bool {
    let found = find(dir, kind, id).is_some();
    if found {
        drop_id(dir, kind, id);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cyex-phist-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn ids(d: &Path, kind: &str) -> Vec<String> {
        list(d, kind).into_iter().map(|s| s.id).collect()
    }

    #[test]
    fn newest_first_and_pruned_past_the_cap() {
        let d = temp("prune");
        for i in 1..=7u8 {
            add(&d, "avatar", "png", u64::from(i) * 10, &[i], Some(b"t"), 5).unwrap();
        }
        let got = list(&d, "avatar");
        assert_eq!(got.len(), 5);
        let stamps: Vec<u64> = got.iter().map(Saved::stamp).collect();
        assert_eq!(stamps, vec![70, 60, 50, 40, 30]);
        assert!(got.iter().all(|s| s.thumb.is_some()));
        // Pruned entries leave no side files behind.
        assert_eq!(fs::read_dir(&d).unwrap().count(), 10);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_same_image_moves_to_the_front() {
        let d = temp("dedupe");
        let a = add(&d, "avatar", "png", 1, b"aaa", None, 5).unwrap();
        add(&d, "avatar", "png", 2, b"bbb", None, 5).unwrap();
        set_url(&d, "avatar", &a, "https://x/a.png");
        let again = add(&d, "avatar", "png", 3, b"aaa", None, 5).unwrap();
        let got = ids(&d, "avatar");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], again);
        assert!(again.starts_with("3-"));
        assert_eq!(fs::read(&list(&d, "avatar")[0].path).unwrap(), b"aaa");
        assert!(url_of(&list(&d, "avatar")[0]).is_none(), "old side files go with the old entry");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn kinds_stay_apart_and_remove_takes_one() {
        let d = temp("remove");
        let a = add(&d, "avatar", "png", 1, b"a", Some(b"t"), 5).unwrap();
        let b = add(&d, "avatar", "jpg", 2, b"b", None, 5).unwrap();
        add(&d, "banner", "png", 3, b"a", None, 3).unwrap();
        set_url(&d, "avatar", &a, "u");
        assert!(remove(&d, "avatar", &a));
        assert!(!remove(&d, "avatar", &a));
        assert!(!remove(&d, "avatar", "../x"));
        assert_eq!(ids(&d, "avatar"), vec![b]);
        assert_eq!(list(&d, "banner").len(), 1, "same bytes, other kind: kept");
        assert_eq!(fs::read_dir(&d).unwrap().count(), 2);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn survives_a_restart() {
        let d = temp("restart");
        let a = add(&d, "offline", "gif", 5, b"x", Some(b"t"), 3).unwrap();
        set_url(&d, "offline", &a, " https://x/o.gif\n");
        // A fresh read of the folder, as the next run does; a half-written file is skipped.
        fs::write(d.join("offline-9-0123456789abcdef.part"), b"half").unwrap();
        let got = list(&d, "offline");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, a);
        assert_eq!(url_of(&got[0]).as_deref(), Some("https://x/o.gif"));
        assert_eq!(find(&d, "offline", &a).map(|s| s.path), Some(got[0].path.clone()));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn ids_are_stamp_and_hash_only() {
        assert!(valid_id("12-0123456789abcdef"));
        assert!(!valid_id("12-0123456789ABCDEF"));
        assert!(!valid_id("12-0123"));
        assert!(!valid_id("-0123456789abcdef"));
        assert!(!valid_id("1a-0123456789abcdef"));
        assert!(!valid_id("..\\x-0123456789abcdef"));
        assert_eq!(hash(b"a").len(), HASH_LEN);
    }
}
