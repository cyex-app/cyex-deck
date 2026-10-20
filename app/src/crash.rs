//! V107-R12 Crash notes. Release builds abort on a panic, so a panic hook writes
//! `logs\crash-<yyyy-mm-dd-HHmmss>.txt` into the local data folder first: the panic message,
//! where it happened, a backtrace and the app version. Never tokens or settings: only the
//! panic's own text goes in, with long token-like runs hidden. The newest 10 are kept. The
//! next start shows one toast about it, then marks it seen.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use time::{OffsetDateTime, UtcOffset};

const KEEP: usize = 10;
const PREFIX: &str = "crash-";
const SEEN: &str = "crash-seen";
/// Twitch tokens are 30 letters and digits; anything this long without a break is hidden.
const TOKEN_RUN: usize = 24;

static DIR: OnceLock<PathBuf> = OnceLock::new();

/// The crash notes' folder: `<local data>\logs`.
pub fn dir(local: &Path) -> PathBuf {
    local.join("logs")
}

/// Runs right after the data folders are known. The local offset is read now: reading it
/// inside the hook, with other threads running, isn't reliable.
pub fn install(local: &Path) {
    let dir = dir(local);
    let _ = DIR.set(dir.clone());
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = payload(info.payload());
        let place = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".into());
        let thread = std::thread::current().name().unwrap_or("unnamed").to_owned();
        let backtrace = readable(std::backtrace::Backtrace::force_capture().to_string());
        let now = OffsetDateTime::now_utc().to_offset(offset);
        let text = note(&message, &place, &thread, &backtrace, now);
        if let Err(e) = write(&dir, now, &text) {
            eprintln!("crash: note not written: {e}");
        }
        previous(info);
    }));
    test_trigger();
}

/// Release builds are stripped: a backtrace of only `<unknown>` frames says nothing.
fn readable(bt: String) -> String {
    let named = bt.lines().any(|l| {
        let l = l.trim();
        !l.is_empty() && !l.ends_with("<unknown>")
    });
    if named {
        bt
    } else {
        "(not available in this build)".into()
    }
}

fn payload(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "(no message)".into())
}

fn note(message: &str, place: &str, thread: &str, backtrace: &str, now: OffsetDateTime) -> String {
    format!(
        "cyex deck {} closed unexpectedly\n\
         When: {}\n\
         Where: {place}\n\
         Thread: {thread}\n\
         Message: {}\n\n\
         Backtrace:\n{backtrace}\n",
        env!("CARGO_PKG_VERSION"),
        stamp(now, "/", " ", ":"),
        scrub(message),
    )
}

/// `2026/10/20 15:30:12` for the note, `2026-10-20-153012` for the file name.
fn stamp(t: OffsetDateTime, date: &str, mid: &str, clock: &str) -> String {
    format!(
        "{:04}{date}{:02}{date}{:02}{mid}{:02}{clock}{:02}{clock}{:02}",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

/// Hides every run of letters, digits, `_` and `-` long enough to be a token or key.
fn scrub(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        if run.chars().count() >= TOKEN_RUN {
            out.push_str("[hidden]");
        } else {
            out.push_str(run);
        }
        run.clear();
    };
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

fn write(dir: &Path, now: OffsetDateTime, text: &str) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join(format!("{PREFIX}{}.txt", stamp(now, "-", "-", ""))),
        text,
    )?;
    prune(dir);
    Ok(())
}

/// Crash notes, oldest first (the names sort by time).
fn notes(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with(PREFIX) && n.ends_with(".txt"))
        .collect();
    names.sort();
    names
}

fn prune(dir: &Path) {
    let names = notes(dir);
    for old in names.iter().take(names.len().saturating_sub(KEEP)) {
        let _ = fs::remove_file(dir.join(old));
    }
}

/// True once per new crash note: the newest one hasn't been shown yet. Marks it seen.
fn take_unseen(dir: &Path) -> bool {
    let Some(newest) = notes(dir).pop() else {
        return false;
    };
    let seen = dir.join(SEEN);
    if fs::read_to_string(&seen).is_ok_and(|s| s.trim() == newest) {
        return false;
    }
    fs::write(seen, &newest).is_ok()
}

/// Whether to show "closed unexpectedly" on this start. Once per crash.
#[tauri::command]
pub fn crash_take_note() -> bool {
    DIR.get().is_some_and(|d| take_unseen(d))
}

/// Opens the logs folder.
#[tauri::command]
pub fn crash_open_folder(app: AppHandle) -> Result<(), String> {
    let dir = DIR.get().ok_or("No logs folder.")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Mocks builds only: `CYEX_TEST_PANIC=1` panics a few seconds after start, to prove the note
/// and the next start's toast. Release builds don't have it.
#[cfg(feature = "mocks")]
fn test_trigger() {
    if std::env::var_os("CYEX_TEST_PANIC").is_some_and(|v| v == "1") {
        let _ = std::thread::Builder::new().name("test-panic".into()).spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(4));
            panic!("CYEX_TEST_PANIC: test crash, token abcdefghijklmnopqrstuvwxyz0123 must not show");
        });
    }
}

#[cfg(not(feature = "mocks"))]
fn test_trigger() {}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cyex-crash-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn hides_token_like_runs_only() {
        assert_eq!(
            scrub("bad oauth:abcdefghijklmnopqrstuvwxyz0123 here"),
            "bad oauth:[hidden] here"
        );
        assert_eq!(
            scrub("index 5 out of range for len 3"),
            "index 5 out of range for len 3"
        );
        assert_eq!(scrub("src/chat/mod.rs"), "src/chat/mod.rs");
    }

    #[test]
    fn note_has_version_place_and_dates() {
        let n = note(
            "boom",
            "app/src/x.rs:1:2",
            "main",
            "bt",
            datetime!(2026-10-20 15:30:12 UTC),
        );
        assert!(n.contains(env!("CARGO_PKG_VERSION")));
        assert!(n.contains("When: 2026/10/20 15:30:12"));
        assert!(n.contains("Where: app/src/x.rs:1:2"));
        assert!(n.contains("Message: boom"));
    }

    #[test]
    fn drops_a_backtrace_without_names() {
        assert_eq!(
            readable(
                "   0: <unknown>
   1: <unknown>
"
                .into()
            ),
            "(not available in this build)"
        );
        let named = "   0: cyex_deck::main
             at app/src/main.rs:5
"
        .to_owned();
        assert_eq!(readable(named.clone()), named);
    }

    #[test]
    fn keeps_the_newest_ten() {
        let d = tmp("keep");
        for s in 0..12u8 {
            let t = datetime!(2026-10-20 15:30:00 UTC) + time::Duration::seconds(s.into());
            write(&d, t, "x").unwrap();
        }
        let left = notes(&d);
        assert_eq!(left.len(), KEEP);
        assert_eq!(left[0], "crash-2026-10-20-153002.txt");
        assert_eq!(left[9], "crash-2026-10-20-153011.txt");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn shows_once_per_crash() {
        let d = tmp("seen");
        assert!(!take_unseen(&d));
        write(&d, datetime!(2026-10-20 15:30:00 UTC), "x").unwrap();
        assert!(take_unseen(&d));
        assert!(!take_unseen(&d));
        write(&d, datetime!(2026-10-20 16:00:00 UTC), "x").unwrap();
        assert!(take_unseen(&d));
        assert!(!take_unseen(&d));
        let _ = fs::remove_dir_all(&d);
    }
}
