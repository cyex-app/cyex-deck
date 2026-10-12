//! S13 Plugins open a channel's tab or switch to one (Tabs permission, limited in
//! `cyex_plugins`). Rust finds the tab or looks the channel up; the main window shows it
//! the way home does, without taking focus from other apps.

use cyex_plugins::Level;
use cyex_settings::Partition;
use cyex_settings::schema::{Tab, TabChannel, Tabs};
use cyex_twitch::gql;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::logs;
use crate::auth::Auth;
use crate::settings::Settings;

pub const OPEN_EVENT: &str = "plugins:open";

/// What the main window shows: a tab it has, or a channel to open like home does.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, rename = "PluginOpen")]
pub enum Open {
    Tab { id: String },
    Channel { channel: TabChannel },
}

/// The tab showing only `login`, else the first that shows it among others.
fn tab_of(tabs: &Tabs, login: &str) -> Option<String> {
    let shows = |t: &&Tab| t.tiles().any(|x| x.channel.login == login);
    let only = |t: &&Tab| t.tiles().all(|x| x.channel.login == login);
    let mut list = tabs.tabs.iter().filter(shows);
    let first = list.clone().next();
    list.find(only).or(first).map(|t| t.id.clone())
}

pub fn open_channel(app: &AppHandle, plugin: &str, login: &str) {
    let (app, plugin, login) = (app.clone(), plugin.to_owned(), login.to_owned());
    tauri::async_runtime::spawn(async move {
        let tabs: Tabs = app.state::<Settings>().read(Partition::Tabs);
        let open = match tab_of(&tabs, &login) {
            Some(id) => Open::Tab { id },
            None => {
                let http = app.state::<Auth>().http.clone();
                let found = gql::lookup(&http, &login).await;
                match found.ok().and_then(|l| l.into_iter().find(|c| c.login == login)) {
                    Some(c) => Open::Channel {
                        channel: TabChannel {
                            id: c.id,
                            login: c.login,
                            name: c.name,
                        },
                    },
                    None => {
                        let text =
                            format!("open_channel: no channel called {login}, or Twitch didn't answer.");
                        return logs::push(&app, &plugin, Level::Warning, &text);
                    }
                }
            }
        };
        let _ = app.emit_to("main", OPEN_EVENT, open);
    });
}

pub fn select_tab(app: &AppHandle, plugin: &str, id: &str) {
    let tabs: Tabs = app.state::<Settings>().read(Partition::Tabs);
    if tabs.tabs.iter().any(|t| t.id == id) {
        let _ = app.emit_to("main", OPEN_EVENT, Open::Tab { id: id.to_owned() });
    } else {
        logs::push(
            app,
            plugin,
            Level::Warning,
            &format!("select_tab: there's no tab {id}."),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_settings::schema::{TabOp, TileShow};

    fn add(t: &mut Tabs, logins: &[&str]) {
        let channels: Vec<TabChannel> = logins
            .iter()
            .map(|l| TabChannel {
                id: format!("id-{l}"),
                login: (*l).into(),
                name: (*l).into(),
            })
            .collect();
        let mut n = 0;
        let mut next = || {
            n += 1;
            format!("{}-{n}", logins.join("-"))
        };
        let op = if channels.len() == 1 {
            TabOp::Add {
                channel: channels[0].clone(),
                show: TileShow::default(),
                account: None,
                alias: None,
                into: None,
            }
        } else {
            TabOp::AddMany {
                channels,
                show: TileShow::default(),
                account: None,
            }
        };
        t.apply(op, &mut next).unwrap();
    }

    #[test]
    fn a_tab_of_its_own_wins() {
        let mut t = Tabs::default();
        add(&mut t, &["a", "b"]);
        add(&mut t, &["b"]);
        assert_eq!(tab_of(&t, "b"), Some(t.tabs[1].id.clone()));
        assert_eq!(tab_of(&t, "a"), Some(t.tabs[0].id.clone()));
        assert_eq!(tab_of(&t, "c"), None);
    }
}
