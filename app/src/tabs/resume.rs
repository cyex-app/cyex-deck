//! V108-N21 After a crash, pick which tabs come back. A `running` marker is written at
//! start and removed on a clean exit; finding it at the next start means the last run
//! ended without one (crash, killed, power loss). The tabs themselves are saved as they
//! change, so they are already loaded: picking only closes the ones left out, through the
//! normal close, so they wait in "Reopen closed tab" (U10). Until the user answers, the
//! rooms and the live poll stay off, so a tab that crashed the app can't do it again.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

const MARKER: &str = "running";

/// Set at start when the last run didn't end cleanly and there are tabs to pick from;
/// cleared when the user answers.
#[derive(Default)]
pub struct Awaiting(Mutex<bool>);

fn marker(local: &Path) -> PathBuf {
    local.join(MARKER)
}

/// Writes the marker. True when the last run left it behind.
fn begin(local: &Path) -> bool {
    let path = marker(local);
    let left = path.exists();
    let _ = fs::create_dir_all(local);
    let _ = fs::write(&path, std::process::id().to_string());
    left
}

fn end(local: &Path) {
    let _ = fs::remove_file(marker(local));
}

/// Tabs to close so only `keep` stay: the ones the dialog `shown` minus `keep`, that still
/// exist in `all`, in list order. Tabs opened since the dialog went up are left alone.
fn to_close(all: &[String], shown: &[String], keep: &[String]) -> Vec<String> {
    all.iter().filter(|id| shown.contains(id) && !keep.contains(id)).cloned().collect()
}

pub fn init(app: &AppHandle) {
    let unclean = crate::paths::local(app).is_ok_and(|l| begin(&l));
    let tabs_there = !app.state::<crate::settings::Settings>().read::<cyex_settings::schema::Tabs>(cyex_settings::Partition::Tabs).tabs.is_empty();
    app.manage(Awaiting(Mutex::new(unclean && tabs_there)));
}

/// Runs on exit, after the settings are flushed.
pub fn clean_exit(app: &AppHandle) {
    if let Ok(local) = crate::paths::local(app) {
        end(&local);
    }
}

/// True from an unclean start until the user answers. Rooms and live stay off meanwhile.
pub fn awaiting(app: &AppHandle) -> bool {
    app.try_state::<Awaiting>().is_some_and(|a| *a.0.lock().unwrap_or_else(|p| p.into_inner()))
}

/// True while the page still has to ask (it asks again after a reload).
pub fn take_pending(app: &AppHandle) -> bool {
    awaiting(app)
}

/// Keeps the tabs in `keep` out of the `shown` ones, closes the rest (they go to the reopen
/// list), then starts what was held back.
pub fn resume(app: &AppHandle, shown: &[String], keep: &[String]) -> Result<super::TabsApplied, String> {
    if let Some(a) = app.try_state::<Awaiting>() {
        *a.0.lock().unwrap_or_else(|p| p.into_inner()) = false;
    }
    let all: Vec<String> = app
        .state::<crate::settings::Settings>()
        .read::<cyex_settings::schema::Tabs>(cyex_settings::Partition::Tabs)
        .tabs
        .iter()
        .map(|t| t.id.clone())
        .collect();
    let close = to_close(&all, shown, keep);
    let out = if close.is_empty() {
        let tabs = app.state::<crate::settings::Settings>().read(cyex_settings::Partition::Tabs);
        Ok(super::TabsApplied { made: None, tabs })
    } else {
        super::apply(app, cyex_settings::schema::TabOp::Close { tabs: close })
    };
    super::start(app);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cyex-resume-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_clean_exit_leaves_no_marker() {
        let d = tmp("clean");
        assert!(!begin(&d), "first start");
        end(&d);
        assert!(!begin(&d), "after a clean exit");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_missing_exit_is_noticed_once() {
        let d = tmp("crash");
        assert!(!begin(&d));
        assert!(begin(&d), "the marker was left behind");
        end(&d);
        assert!(!begin(&d));
        end(&d);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn closes_only_the_tabs_not_picked() {
        let all = ids(&["a", "b", "c"]);
        assert_eq!(to_close(&all, &all, &ids(&["a", "c"])), ids(&["b"]));
        assert_eq!(to_close(&all, &all, &ids(&["a", "b", "c"])), Vec::<String>::new());
        assert_eq!(to_close(&all, &all, &[]), all, "start empty");
        assert_eq!(to_close(&all, &all, &ids(&["zz"])), all, "unknown ids keep nothing");
        assert_eq!(to_close(&all, &ids(&["a", "b"]), &ids(&["a"])), ids(&["b"]), "c wasn't shown");
    }
}
