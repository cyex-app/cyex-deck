//! V107-R19 The image a change replaced, one file per kind and account:
//! `local/profile-backup/<userId>/<kind>-<unix ms>.<ext>`. One per kind: the original,
//! kept until a restore succeeds.

use std::fs;
use std::path::{Path, PathBuf};

/// `<root>/<userId>`. `None` for an id that isn't plain digits (never a path).
pub fn dir(root: &Path, user_id: &str) -> Option<PathBuf> {
    let plain = !user_id.is_empty() && user_id.bytes().all(|b| b.is_ascii_digit());
    plain.then(|| root.join(user_id))
}

/// Stamp from a file name like `avatar-1760000000000.png`.
fn stamp(name: &str, kind: &str) -> Option<u64> {
    let rest = name.strip_prefix(kind)?.strip_prefix('-')?;
    rest.split('.').next()?.parse().ok()
}

fn of_kind(dir: &Path, kind: &str) -> Vec<(u64, PathBuf)> {
    let Ok(read) = fs::read_dir(dir) else { return Vec::new() };
    read.flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let done = !name.ends_with(".part");
            stamp(&name, kind).filter(|_| done).map(|s| (s, e.path()))
        })
        .collect()
}

/// The newest backup of a kind.
pub fn newest(dir: &Path, kind: &str) -> Option<PathBuf> {
    of_kind(dir, kind).into_iter().max_by_key(|(s, _)| *s).map(|(_, p)| p)
}

/// Writes the backup, then drops older ones of the same kind.
pub fn save(dir: &Path, kind: &str, ext: &str, now_ms: u64, bytes: &[u8]) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{kind}-{now_ms}.{ext}"));
    let tmp = dir.join(format!("{kind}-{now_ms}.part"));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, &path)?;
    for (s, old) in of_kind(dir, kind) {
        if s < now_ms || (s == now_ms && old != path) {
            let _ = fs::remove_file(old);
        }
    }
    Ok(path)
}

/// Drops every backup of a kind (after a restore worked).
pub fn remove(dir: &Path, kind: &str) {
    for (_, p) in of_kind(dir, kind) {
        let _ = fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cyex-pbk-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn keeps_only_the_newest_per_kind() {
        let d = temp("rot");
        save(&d, "avatar", "png", 100, b"a").unwrap();
        save(&d, "banner", "jpg", 150, b"b").unwrap();
        let last = save(&d, "avatar", "gif", 200, b"c").unwrap();
        assert_eq!(newest(&d, "avatar"), Some(last.clone()));
        assert_eq!(fs::read(&last).unwrap(), b"c");
        assert_eq!(of_kind(&d, "avatar").len(), 1);
        assert!(newest(&d, "banner").is_some(), "other kinds stay");
        assert_eq!(newest(&d, "offline"), None);
        remove(&d, "avatar");
        assert_eq!(newest(&d, "avatar"), None);
        assert!(newest(&d, "banner").is_some(), "remove keeps other kinds");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn same_stamp_replaces() {
        let d = temp("same");
        save(&d, "offline", "png", 5, b"1").unwrap();
        let p = save(&d, "offline", "jpg", 5, b"2").unwrap();
        assert_eq!(of_kind(&d, "offline").len(), 1);
        assert_eq!(newest(&d, "offline"), Some(p));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn ids_are_digits_only() {
        let root = Path::new("r");
        assert_eq!(dir(root, "1294600497"), Some(root.join("1294600497")));
        assert_eq!(dir(root, "../x"), None);
        assert_eq!(dir(root, ""), None);
    }

    #[test]
    fn stamps_need_the_kind_prefix() {
        assert_eq!(stamp("avatar-12.png", "avatar"), Some(12));
        assert_eq!(stamp("avatarx-12.png", "avatar"), None);
        assert_eq!(stamp("avatar-12.part", "avatar"), Some(12));
        assert_eq!(stamp("banner-12.png", "avatar"), None);
    }
}
