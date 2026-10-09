use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Writes via a temp file and rename, so a crash never leaves a half-written file.
pub(crate) fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut file = File::create(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)
}
