//! V1052-02 How much disk the picture cache (WebView2's HTTP cache) takes.

use std::fs;
use std::path::Path;

/// Folders `clear_picture_cache` empties, inside a profile's `Default` folder.
const CACHE_DIRS: &[&str] = &["Cache", "Code Cache"];

/// Bytes in the cache folders under a WebView2 data folder (the app's own, with or without
/// the `EBWebView` level). Missing folders count as 0.
pub fn picture_cache_size(data_dir: &Path) -> u64 {
    [data_dir.join("EBWebView"), data_dir.to_path_buf()]
        .iter()
        .flat_map(|root| CACHE_DIRS.iter().map(move |d| root.join("Default").join(d)))
        .map(|dir| dir_size(&dir))
        .sum()
}

fn dir_size(dir: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else { return 0 };
    entries
        .filter_map(Result::ok)
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map_or(0, |m| m.len()),
            _ => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![0u8; bytes]).unwrap();
    }

    #[test]
    fn counts_the_cache_folders_only() {
        let dir = std::env::temp_dir().join(format!("cyex-test-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        put(&dir.join("EBWebView/Default/Cache/Cache_Data/f_1"), 100);
        put(&dir.join("EBWebView/Default/Cache/index"), 20);
        put(&dir.join("EBWebView/Default/Code Cache/js/a"), 3);
        put(&dir.join("EBWebView/Default/Cookies"), 9_999);
        assert_eq!(picture_cache_size(&dir), 123);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_folder_is_zero() {
        assert_eq!(picture_cache_size(Path::new("Z:/does/not/exist")), 0);
    }
}
