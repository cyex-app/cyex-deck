//! V108-N21 After a crash, pick which tabs come back. A `running` marker is written at
//! start and removed on a clean exit; finding it at the next start means the last run
//! ended without one (crash, killed, power loss). The tabs themselves are saved as they
//! change, so they are already loaded: picking only closes the ones left out, through the
//! normal close, so they wait in "Reopen closed tab" (U10).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

const MARKER: &str = "running";

/// Set at start when the last run didn't end cleanly; taken once by the page.
#[derive(Default)]
pub struct Unclean(Mutex<bool>);

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

/// Tabs to close so only `keep` stay, in list order. Unknown ids in `keep` are ignored.
fn to_close(all: &[String], keep: &[String]) -> Vec<String> {
    all.iter().filter(|id| !keep.contains(id)).cloned().collect()
}

pub fn init(app: &AppHandle) {
    let unclean = crate::paths::local(app).is_ok_and(|l| begin(&l));
    app.manage(Unclean(Mutex::new(unclean)));
}

/// Runs on exit, after the settings are flushed.
pub fn clean_exit(app: &AppHandle) {
    if let Ok(local) = crate::paths::local(app) {
        end(&local);
    }
}

/// True once per unclean start, when there are tabs to pick from.
pub fn take_pending(app: &AppHandle) -> bool {
    let flag = std::mem::take(&mut *app.state::<Unclean>().0.lock().unwrap_or_else(|p| p.into_inner()));
    flag && !app.state::<crate::settings::Settings>().read::<cyex_settings::schema::Tabs>(cyex_settings::Partition::Tabs).tabs.is_empty()
}

/// Keeps the tabs in `keep`, closes the rest (they go to the reopen list).
pub fn resume(app: &AppHandle, keep: &[String]) -> Result<super::TabsApplied, String> {
    let all: Vec<String> = app
        .state::<crate::settings::Settings>()
        .read::<cyex_settings::schema::Tabs>(cyex_settings::Partition::Tabs)
        .tabs
        .iter()
        .map(|t| t.id.clone())
        .collect();
    let close = to_close(&all, keep);
    if close.is_empty() {
        let tabs = app.state::<crate::settings::Settings>().read(cyex_settings::Partition::Tabs);
        return Ok(super::TabsApplied { made: None, tabs });
    }
    super::apply(app, cyex_settings::schema::TabOp::Close { tabs: close })
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
        assert_eq!(to_close(&all, &ids(&["a", "c"])), ids(&["b"]));
        assert_eq!(to_close(&all, &ids(&["a", "b", "c"])), Vec::<String>::new());
        assert_eq!(to_close(&all, &[]), all, "start empty");
        assert_eq!(to_close(&all, &ids(&["zz"])), all, "unknown ids keep nothing");
    }
}
