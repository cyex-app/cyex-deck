//! This app's Twitch client ID (D5). Never compiled in. Looked up in order: the
//! `TWITCH_CLIENT_ID` environment variable, then `.env` files. The first-run prompt saves
//! it to the `.env` in the app's data folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const ENV_KEY: &str = "TWITCH_CLIENT_ID";

pub fn resolve(env_files: &[PathBuf]) -> Option<String> {
    std::env::var(ENV_KEY).ok().filter(|v| is_valid(v)).or_else(|| {
        env_files.iter().find_map(|f| {
            fs::read_to_string(f)
                .ok()
                .and_then(|text| read_key(&text, ENV_KEY))
        })
    })
}

/// Client IDs are 30 lowercase letters and digits. Allow some slack.
pub fn is_valid(id: &str) -> bool {
    (20..=40).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// Writes or replaces the key in the `.env` file, keeping its other lines.
pub fn save(path: &Path, id: &str) -> io::Result<()> {
    let text = fs::read_to_string(path).unwrap_or_default();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, write_key(&text, ENV_KEY, id))
}

fn read_key(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (k, v) = split_line(line)?;
        let v = v.trim().trim_matches(['"', '\'']);
        (k == key && is_valid(v)).then(|| v.to_owned())
    })
}

fn write_key(text: &str, key: &str, value: &str) -> String {
    let entry = format!("{key}={value}");
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| match split_line(line) {
            Some((k, _)) if k == key => {
                found = true;
                entry.clone()
            }
            _ => line.to_owned(),
        })
        .collect();
    if !found {
        lines.push(entry);
    }
    lines.join("\n") + "\n"
}

/// `KEY=value` or `export KEY=value`. Comments and blank lines give `None`.
fn split_line(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let line = line.strip_prefix("export ").unwrap_or(line);
    let (k, v) = line.split_once('=')?;
    Some((k.trim(), v))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "abcdefghijklmnopqrstuvwxyz0123";

    #[test]
    fn reads_the_key_and_skips_comments() {
        let text = format!("# TWITCH_CLIENT_ID=nope\nOTHER=1\nexport TWITCH_CLIENT_ID=\"{ID}\"\n");
        assert_eq!(read_key(&text, ENV_KEY).as_deref(), Some(ID));
        assert_eq!(read_key("TWITCH_CLIENT_ID=\n", ENV_KEY), None);
    }

    #[test]
    fn write_replaces_or_appends() {
        let replaced = write_key("A=1\nTWITCH_CLIENT_ID=old\n# note\n", ENV_KEY, ID);
        assert_eq!(replaced, format!("A=1\nTWITCH_CLIENT_ID={ID}\n# note\n"));
        assert_eq!(write_key("", ENV_KEY, ID), format!("TWITCH_CLIENT_ID={ID}\n"));
    }

    #[test]
    fn validity() {
        assert!(is_valid(ID));
        assert!(!is_valid("short"));
        assert!(!is_valid("has spaces in it but is long enough"));
    }
}
