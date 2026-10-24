//! V108-03 The browser the user used last: the top-most normal browser window (cyex deck's
//! own windows skipped) decides which browser, and its command line which profile, a
//! sign-in opens in. Falls back to that browser without profile flags, then to the default
//! browser (the caller).

mod args;
#[cfg(windows)]
mod win32;

use std::path::PathBuf;

use args::{family, profile_args, split};

/// Exe names (lowercase, no `.exe`) always taken for browsers; the ones Windows lists as
/// installed browsers (Clients\StartMenuInternet: Thorium, Mullvad…) come on top.
const BROWSERS: [&str; 15] = [
    "chrome",
    "msedge",
    "firefox",
    "brave",
    "opera",
    "opera_gx",
    "vivaldi",
    "arc",
    "zen",
    "librewolf",
    "waterfox",
    "chromium",
    "thorium",
    "floorp",
    "mullvadbrowser",
];

/// Listed as browsers but not one to sign in with.
const NOT_BROWSERS: [&str; 1] = ["iexplore"];

/// One top-level window, in Z-order (top first).
#[derive(Clone, Debug)]
pub struct Win {
    pub pid: u32,
    pub exe: String,
    pub visible: bool,
    /// Tool windows (floating palettes) or cloaked ones (other desktops, hidden UWP frames).
    pub hidden_kind: bool,
    /// Always on top (picture-in-picture players and the like): above every normal window
    /// whether used last or not, so ranked after them.
    pub topmost: bool,
}

/// `exe` lowercased, without `.exe`.
fn stem(exe: &str) -> String {
    let name = exe.to_ascii_lowercase();
    name.strip_suffix(".exe").unwrap_or(&name).to_owned()
}

fn is_browser(exe: &str, installed: &[String]) -> bool {
    let name = stem(exe);
    !name.is_empty()
        && !NOT_BROWSERS.contains(&name.as_str())
        && (BROWSERS.contains(&name.as_str()) || installed.contains(&name))
}

/// Browser windows' pids, the one used last first: normal windows in Z-order, then the
/// always-on-top ones; without `own_pid`'s (this app, in front after its button is clicked)
/// and without hidden, tool and cloaked windows. Repeats dropped.
pub fn candidates(windows: &[Win], own_pid: u32, installed: &[String]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for topmost in [false, true] {
        for w in windows.iter().filter(|w| w.topmost == topmost) {
            if w.pid != own_pid
                && w.visible
                && !w.hidden_kind
                && is_browser(&w.exe, installed)
                && !out.contains(&w.pid)
            {
                out.push(w.pid);
            }
        }
    }
    out
}

/// The exe of a registered browser's open command, which may be an unquoted path with
/// spaces (`C:\Program Files\…\iexplore.exe`).
fn exe_of(command: &str) -> String {
    let command = command.trim();
    if command.starts_with('"') {
        return split(command).swap_remove(0);
    }
    match command.to_ascii_lowercase().find(".exe") {
        Some(end) => command[..end + 4].to_owned(),
        None => split(command).swap_remove(0),
    }
}

/// What to start: the browser's exe and the flags for its profile.
#[derive(Debug, PartialEq, Eq)]
pub struct Launch {
    pub exe: PathBuf,
    pub args: Vec<String>,
}

/// The launch for the browser process `pid`: its exe (else the installed one of that name)
/// and its profile flags, read from its command line.
fn plan_for(exe: PathBuf, command_line: Option<&str>) -> Launch {
    let name = stem(
        &exe.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    let args = command_line
        .map(|line| profile_args(family(&name), &split(line)))
        .unwrap_or_default();
    Launch { exe, args }
}

/// The launch for the browser used last, if one is open.
#[cfg(windows)]
pub fn last_browser() -> Option<Launch> {
    let installed = win32::installed();
    let names: Vec<String> = installed.iter().map(|(n, _)| n.clone()).collect();
    let windows = win32::top_level();
    let own = std::process::id();
    candidates(&windows, own, &names).into_iter().find_map(|pid| {
        let exe = win32::exe_path(pid).or_else(|| {
            let name = stem(&windows.iter().find(|w| w.pid == pid)?.exe);
            installed.iter().find(|(n, _)| *n == name).map(|(_, p)| p.clone())
        })?;
        Some(plan_for(exe, win32::command_line(pid).as_deref()))
    })
}

#[cfg(not(windows))]
pub fn last_browser() -> Option<Launch> {
    None
}

/// Opens `url` in the browser (and profile) used last. `false`: none found or it wouldn't
/// start.
pub fn open_in_last_browser(url: &str) -> bool {
    last_browser().is_some_and(|l| launch(&l, url))
}

/// Exe and flags as separate arguments, the URL last; no shell in between. Without the
/// profile flags if it won't start with them.
fn launch(l: &Launch, url: &str) -> bool {
    let start = |args: &[String]| {
        std::process::Command::new(&l.exe)
            .args(args)
            .arg(url)
            .spawn()
            .is_ok()
    };
    start(&l.args) || (!l.args.is_empty() && start(&[]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(pid: u32, exe: &str, visible: bool, hidden_kind: bool) -> Win {
        Win {
            pid,
            exe: exe.into(),
            visible,
            hidden_kind,
            topmost: false,
        }
    }

    #[test]
    fn always_on_top_windows_rank_last() {
        let pip = Win {
            topmost: true,
            ..w(1, "brave.exe", true, false)
        };
        let list = [
            w(9, "cyex-deck.exe", true, false),
            pip,
            w(2, "firefox.exe", true, false),
        ];
        assert_eq!(candidates(&list, 9, &[]), vec![2, 1]);
    }

    #[test]
    fn top_most_visible_browser_wins() {
        let list = [
            w(1, "cyex-deck.exe", true, false),
            w(2, "Code.exe", true, false),
            w(3, "firefox.exe", false, false),
            w(4, "chrome.exe", true, true),
            w(5, "msedge.exe", true, false),
            w(6, "chrome.exe", true, false),
            w(5, "msedge.exe", true, false),
        ];
        assert_eq!(candidates(&list, 1, &[]), vec![5, 6]);
    }

    #[test]
    fn own_windows_are_skipped_even_on_top() {
        // The app was clicked, so it's in front; a browser-named copy of it still doesn't count.
        let list = [
            w(7, "cyex-deck.exe", true, false),
            w(7, "msedge.exe", true, false),
            w(9, "firefox.exe", true, false),
        ];
        assert_eq!(candidates(&list, 7, &[]), vec![9]);
    }

    #[test]
    fn installed_browsers_count_too() {
        let list = [
            w(1, "Discord.exe", true, false),
            w(2, "thorium.exe", true, false),
            w(3, "NewFork.exe", true, false),
        ];
        assert_eq!(candidates(&list, 0, &[]), vec![2]);
        assert_eq!(candidates(&list, 0, &["newfork".into()]), vec![2, 3]);
        assert!(candidates(&[w(1, "iexplore.exe", true, false)], 0, &["iexplore".into()]).is_empty());
    }

    #[test]
    fn names_match_without_case_or_exe() {
        assert!(is_browser("Opera_GX.EXE", &[]));
        assert!(is_browser("zen", &[]));
        assert!(!is_browser("chromedriver.exe", &[]));
        assert!(!is_browser("msedgewebview2.exe", &[]));
        assert!(!is_browser("explorer.exe", &[]));
        assert!(!is_browser("", &["".into()]));
    }

    #[test]
    fn nothing_found_falls_back() {
        assert!(candidates(&[], 0, &[]).is_empty());
        assert!(
            candidates(
                &[w(1, "notepad.exe", true, false), w(2, "brave.exe", false, false)],
                0,
                &[]
            )
            .is_empty()
        );
    }

    #[test]
    fn registered_commands() {
        assert_eq!(
            exe_of(r#""C:\Program Files\B\brave.exe" --single-argument %1"#),
            r"C:\Program Files\B\brave.exe"
        );
        assert_eq!(
            exe_of(r"C:\Program Files\Internet Explorer\iexplore.exe"),
            r"C:\Program Files\Internet Explorer\iexplore.exe"
        );
        assert_eq!(exe_of(r"C:\x\thorium.EXE -osint"), r"C:\x\thorium.EXE");
        assert_eq!(exe_of(r"C:\x\run"), r"C:\x\run");
    }

    #[test]
    fn plans_carry_the_profile() {
        let ff = PathBuf::from(r"C:\Program Files\Mozilla Firefox\firefox.exe");
        let p = plan_for(
            ff.clone(),
            Some(r#""C:\Program Files\Mozilla Firefox\firefox.exe" -P "Alt Twitch""#),
        );
        assert_eq!(
            p,
            Launch {
                exe: ff.clone(),
                args: vec!["-P".into(), "Alt Twitch".into()]
            }
        );
        assert!(
            plan_for(ff, None).args.is_empty(),
            "no command line: the browser alone"
        );
        let brave = PathBuf::from(r"C:\B\brave.exe");
        assert!(
            plan_for(brave, Some(r#"C:\B\brave.exe --profile-directory="Profile 1""#))
                .args
                .is_empty()
        );
    }

    /// Live check, prints what a sign-in would open; with CYEX_LAUNCH=1 it also opens
    /// https://example.com (or CYEX_URL) there. `cargo test -p cyex-deck live_last_browser -- --ignored --nocapture`
    #[test]
    #[ignore]
    #[cfg(windows)]
    fn live_last_browser() {
        let installed = win32::installed();
        println!(
            "installed: {:?}",
            installed.iter().map(|(n, _)| n).collect::<Vec<_>>()
        );
        let windows = win32::top_level();
        let names: Vec<String> = installed.iter().map(|(n, _)| n.clone()).collect();
        for pid in candidates(&windows, std::process::id(), &names) {
            let line = win32::command_line(pid);
            println!(
                "candidate {pid}: exe {:?}, command line read: {}",
                win32::exe_path(pid),
                line.is_some()
            );
            if let Some(exe) = win32::exe_path(pid) {
                println!("  flags: {:?}", plan_for(exe, line.as_deref()).args);
            }
        }
        // CYEX_PID: that browser process instead of the top one.
        let plan = match std::env::var("CYEX_PID").ok().and_then(|p| p.parse::<u32>().ok()) {
            Some(pid) => win32::exe_path(pid).map(|exe| plan_for(exe, win32::command_line(pid).as_deref())),
            None => last_browser(),
        };
        println!("plan: {plan:?}");
        if std::env::var("CYEX_LAUNCH").is_ok_and(|v| v == "1") {
            let url = std::env::var("CYEX_URL").unwrap_or_else(|_| "https://example.com/".into());
            assert!(plan.is_some_and(|l| launch(&l, &url)));
        }
    }
}
