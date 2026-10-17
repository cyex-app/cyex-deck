//! V106-25 Tabs muted from their right-click menu: everything in them is silent until unmuted or the
//! app restarts (never saved). Pages hush their players; here the chat and live-alert
//! sounds of their channels are skipped, unless the channel is also in a tab that isn't muted.

use std::collections::HashSet;
use std::sync::Mutex;

use cyex_settings::Partition;
use cyex_settings::schema::{TabChannel, Tabs};
use tauri::{AppHandle, Emitter, Manager};

use crate::settings::Settings;

pub const EVENT: &str = "tabs:muted";

#[derive(Default)]
pub struct Muted(Mutex<HashSet<String>>);

impl Muted {
    fn with<T>(&self, f: impl FnOnce(&mut HashSet<String>) -> T) -> T {
        f(&mut self.0.lock().unwrap_or_else(|p| p.into_inner()))
    }
}

fn sorted(set: &HashSet<String>) -> Vec<String> {
    let mut ids: Vec<String> = set.iter().cloned().collect();
    ids.sort();
    ids
}

/// Mutes or unmutes a tab; every window hears of it. Returns the muted tabs.
pub fn set(app: &AppHandle, tab: &str, on: bool) -> Vec<String> {
    let ids = app.state::<Muted>().with(|m| {
        if on {
            m.insert(tab.to_owned());
        } else {
            m.remove(tab);
        }
        sorted(m)
    });
    let _ = app.emit(EVENT, &ids);
    ids
}

pub fn list(app: &AppHandle) -> Vec<String> {
    app.state::<Muted>().with(|m| sorted(m))
}

/// Closed tabs drop out.
pub fn prune(app: &AppHandle) {
    let Some(state) = app.try_state::<Muted>() else {
        return;
    };
    if state.with(|m| m.is_empty()) {
        return;
    }
    let tabs: Tabs = app.state::<Settings>().read(Partition::Tabs);
    if let Some(ids) = state.with(|m| prune_ids(m, &tabs).then(|| sorted(m))) {
        let _ = app.emit(EVENT, ids);
    }
}

/// A channel (by login) is only in muted tabs.
pub fn hushes_login(app: &AppHandle, login: &str) -> bool {
    hushes(app, |c| c.login.eq_ignore_ascii_case(login))
}

/// A channel (by user id) is only in muted tabs.
pub fn hushes_id(app: &AppHandle, channel_id: &str) -> bool {
    hushes(app, |c| c.id == channel_id)
}

fn hushes(app: &AppHandle, hit: impl Fn(&TabChannel) -> bool) -> bool {
    let Some(state) = app.try_state::<Muted>() else {
        return false;
    };
    let muted = state.with(|m| m.clone());
    if muted.is_empty() {
        return false;
    }
    let tabs: Tabs = app.state::<Settings>().read(Partition::Tabs);
    hushes_in(&tabs, &muted, hit)
}

/// The channel is in a tab, and every tab it is in is muted.
fn hushes_in(tabs: &Tabs, muted: &HashSet<String>, hit: impl Fn(&TabChannel) -> bool) -> bool {
    let mut found = false;
    for tab in tabs.tabs.iter().filter(|t| t.tiles().any(|x| hit(&x.channel))) {
        if !muted.contains(&tab.id) {
            return false;
        }
        found = true;
    }
    found
}

/// Drops ids no tab has; true when any went.
fn prune_ids(muted: &mut HashSet<String>, tabs: &Tabs) -> bool {
    let before = muted.len();
    muted.retain(|id| tabs.tab(id).is_some());
    muted.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_settings::schema::{Column, Tab, Tile};

    fn tab(id: &str, login: &str) -> Tab {
        Tab {
            id: id.into(),
            columns: vec![Column {
                size: 1.0,
                tiles: vec![Tile {
                    id: format!("{id}-1"),
                    channel: TabChannel {
                        id: format!("u{login}"),
                        login: login.into(),
                        name: login.to_uppercase(),
                    },
                    ..Tile::default()
                }],
            }],
            ..Tab::default()
        }
    }

    fn tabs() -> Tabs {
        Tabs {
            tabs: vec![tab("a", "alpha"), tab("b", "beta")],
            ..Tabs::default()
        }
    }

    fn set(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn hushes_channels_of_muted_tabs_only() {
        let tabs = tabs();
        let muted = set(&["a"]);
        assert!(hushes_in(&tabs, &muted, |c| c
            .login
            .eq_ignore_ascii_case("ALPHA")));
        assert!(hushes_in(&tabs, &muted, |c| c.id == "ualpha"));
        assert!(!hushes_in(&tabs, &muted, |c| c
            .login
            .eq_ignore_ascii_case("beta")));
        assert!(!hushes_in(&tabs, &muted, |c| c.id == "ubeta"));
        assert!(!hushes_in(&tabs, &HashSet::new(), |c| c.login == "alpha"));
    }

    #[test]
    fn a_channel_also_in_an_unmuted_tab_still_sounds() {
        let mut tabs = tabs();
        tabs.tabs.push(tab("c", "alpha"));
        assert!(!hushes_in(&tabs, &set(&["a"]), |c| c.login == "alpha"));
        assert!(hushes_in(&tabs, &set(&["a", "c"]), |c| c.login == "alpha"));
        assert!(!hushes_in(&tabs, &set(&["a", "c"]), |c| c.login == "nobody"));
    }

    #[test]
    fn prune_drops_closed_tabs() {
        let tabs = tabs();
        let mut muted = set(&["a", "gone"]);
        assert!(prune_ids(&mut muted, &tabs));
        assert_eq!(muted, set(&["a"]));
        assert!(!prune_ids(&mut muted, &tabs));
        assert_eq!(muted, set(&["a"]));
    }
}
