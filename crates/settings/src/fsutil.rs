use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Writes via a temp file and rename, so a crash never leaves a half-written file.
/// V107-N10a A failed write (a full disk) removes its temp file; the old file stays.
pub(crate) fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    let written = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn a_failed_write_keeps_the_old_file_and_no_temp() {
        let tmp = TempDir::new("fsutil-fail");
        // The rename fails: a folder sits where the file goes.
        let target = tmp.path().join("chat.json");
        fs::create_dir(&target).unwrap();
        assert!(write_atomic(&target, b"{}").is_err());
        assert!(!tmp.path().join("chat.tmp").exists());
        assert!(target.is_dir());
    }
}
