//! V1041-02 What a read layout adds to the tabs: Chatterino's order, channels that have a
//! tab already skipped, up to the join limit (1.10). Nothing is dropped without a line in
//! the report.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use ts_rs::TS;

use super::{Layout, LayoutTab, SplitKind};
use crate::schema::{ImportTab, TabChannel, Tabs};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub tabs: Vec<ImportTab>,
    pub report: ImportReport,
}

/// The preview before the import, and the result after it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportReport {
    /// Tabs and channels that go in.
    pub tabs: u32,
    pub channels: u32,
    /// Logins that have a tab already, or came up twice.
    pub already_open: Vec<String>,
    /// Splits that aren't Twitch channels.
    pub not_channels: Vec<NotChannel>,
    /// Names Twitch doesn't know: renamed, banned or deleted.
    pub unresolved: Vec<String>,
    /// Logins past the chats one account can join (1.10).
    pub over_limit: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NotChannel {
    pub kind: SplitKind,
    pub count: u32,
}

/// The account the import joins with.
#[derive(Clone, Copy, Debug)]
pub struct Joins<'a> {
    /// Chats it can still join before Twitch's limit.
    pub slots: usize,
    /// Channel ids that never count: its own (V102-21) and the ones it moderates.
    pub free: &'a [String],
}

/// `found` maps logins to channels (see `resolve`); `tabs` are the tabs open now.
pub fn plan(layout: &Layout, found: &HashMap<String, TabChannel>, tabs: &Tabs, joins: Joins<'_>) -> Plan {
    let mut open: HashSet<String> = tabs.channel_ids().into_iter().collect();
    let mut slots = joins.slots;
    let mut plan = Plan::default();
    let report = &mut plan.report;
    report.not_channels = layout
        .skipped
        .iter()
        .map(|(&kind, &count)| NotChannel { kind, count })
        .collect();

    for tab in &layout.tabs {
        let mut columns = Vec::new();
        for column in &tab.columns {
            let mut tiles = Vec::new();
            for name in column {
                let Some(channel) = found.get(name) else {
                    add_once(&mut report.unresolved, name);
                    continue;
                };
                let free = joins.free.contains(&channel.id);
                if open.contains(&channel.id) {
                    add_once(&mut report.already_open, &channel.login);
                } else if !free && slots == 0 {
                    add_once(&mut report.over_limit, &channel.login);
                } else {
                    open.insert(channel.id.clone());
                    slots -= usize::from(!free);
                    tiles.push(channel.clone());
                }
            }
            if !tiles.is_empty() {
                columns.push(tiles);
            }
        }
        if columns.is_empty() {
            continue;
        }
        report.tabs += 1;
        report.channels += columns.iter().map(Vec::len).sum::<usize>() as u32;
        plan.tabs.push(ImportTab {
            alias: alias(tab),
            columns,
        });
    }
    plan
}

/// 1.15 The custom title, unless it's just the name of the tab's only channel: without
/// an alias the tab follows that channel's renames.
fn alias(tab: &LayoutTab) -> Option<String> {
    let title = tab.title.as_deref()?;
    let mut names = tab.channels();
    let only = names.next().filter(|_| names.next().is_none());
    if only.is_some_and(|n| n.eq_ignore_ascii_case(title)) {
        return None;
    }
    Some(title.to_owned())
}

fn add_once(list: &mut Vec<String>, login: &str) {
    if !list.iter().any(|l| l == login) {
        list.push(login.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chatterino::parse;
    use crate::chatterino::resolve::testing::{channel, now};
    use crate::chatterino::resolve::{names, resolve};
    use crate::schema::{Column, Tab, Tile};

    fn fixture(name: &str) -> Layout {
        let path = format!("{}/tests/fixtures/chatterino/{name}", env!("CARGO_MANIFEST_DIR"));
        parse(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// Looks the layout up on a fake Twitch that knows every name but `unknown`.
    fn found(layout: &Layout, unknown: &[&str]) -> HashMap<String, TabChannel> {
        now(resolve(&names(layout), |batch| {
            let known = batch
                .iter()
                .filter(|n| !unknown.contains(&n.as_str()))
                .map(|n| channel(n))
                .collect();
            async move { Ok::<Vec<TabChannel>, ()>(known) }
        }))
        .unwrap()
    }

    fn logins(t: &ImportTab) -> Vec<Vec<&str>> {
        t.columns
            .iter()
            .map(|c| c.iter().map(|x| x.login.as_str()).collect())
            .collect()
    }

    const PLENTY: Joins<'static> = Joins {
        slots: 100,
        free: &[],
    };

    #[test]
    fn tabs_keep_their_order_titles_and_splits() {
        let layout = fixture("current.json");
        let tile = Tile {
            id: "x1".into(),
            channel: channel("bravo_two"),
            ..Tile::default()
        };
        let open = Tabs {
            tabs: vec![Tab {
                id: "x".into(),
                columns: vec![Column {
                    size: 1.0,
                    tiles: vec![tile],
                }],
                ..Tab::default()
            }],
            ..Tabs::default()
        };
        let plan = plan(&layout, &found(&layout, &["golf_seven"]), &open, PLENTY);

        let got: Vec<(Option<&str>, Vec<Vec<&str>>)> = plan
            .tabs
            .iter()
            .map(|t| (t.alias.as_deref(), logins(t)))
            .collect();
        assert_eq!(
            got,
            [
                (Some("Morning"), vec![vec!["alpha_one"]]),
                (None, vec![vec!["charlie_three"]]),
                // Just the channel's name: no alias, so renames follow (1.15).
                (None, vec![vec!["delta_four"]]),
                (Some("🎮 Mixed"), vec![vec!["echo_five"]]),
                (Some("Pop"), vec![vec!["foxtrot_six"]]),
            ]
        );
        let r = &plan.report;
        assert_eq!((r.tabs, r.channels), (5, 5));
        assert_eq!(r.already_open, ["bravo_two", "alpha_one"]);
        assert_eq!(r.unresolved, ["golf_seven"]);
        assert!(r.over_limit.is_empty());
        let not: u32 = r.not_channels.iter().map(|n| n.count).sum();
        assert_eq!(not, 9);
        assert_eq!(
            r.not_channels[0],
            NotChannel {
                kind: SplitKind::Mentions,
                count: 2
            }
        );
    }

    #[test]
    fn the_join_limit_cuts_in_order_and_lists_the_rest() {
        let layout = fixture("many.json");
        let found = found(&layout, &[]);
        let plan = plan(&layout, &found, &Tabs::default(), Joins { slots: 95, free: &[] });
        let r = &plan.report;
        assert_eq!((r.tabs, r.channels), (95, 95));
        assert_eq!(r.over_limit.len(), 25);
        assert_eq!(
            (r.over_limit[0].as_str(), r.over_limit[24].as_str()),
            ("chan_096", "chan_120")
        );
        assert_eq!(logins(&plan.tabs[94]), [["chan_095"]]);

        // Halfway through a tab: what fits goes in, under the tab's title.
        let plan = plan_with(&layout, &found, 101, &[]);
        let last = plan.tabs.last().unwrap();
        assert_eq!(
            (last.alias.as_deref(), logins(last)),
            (Some("Pair 1"), vec![vec!["chan_101"]])
        );
        assert_eq!(
            plan.report.over_limit.first().map(String::as_str),
            Some("chan_102")
        );

        // Its own channel and the ones it moderates never count.
        let plan = plan_with(&layout, &found, 0, &["id-chan_008".into()]);
        assert_eq!((plan.report.tabs, plan.report.over_limit.len()), (1, 119));
        assert_eq!(plan.tabs[0].alias, None);
    }

    fn plan_with(
        layout: &Layout,
        found: &HashMap<String, TabChannel>,
        slots: usize,
        free: &[String],
    ) -> Plan {
        plan(layout, found, &Tabs::default(), Joins { slots, free })
    }

    #[test]
    fn nothing_to_import() {
        let layout = parse(
            r#"{"windows":[{"tabs":[{"title":"x","splits2":{"type":"split","data":{"type":"mentions"}}}]}]}"#,
        )
        .unwrap();
        let plan = plan(&layout, &HashMap::new(), &Tabs::default(), PLENTY);
        assert!(plan.tabs.is_empty());
        assert_eq!(
            plan.report.not_channels,
            [NotChannel {
                kind: SplitKind::Mentions,
                count: 1
            }]
        );
        assert_eq!((plan.report.tabs, plan.report.channels), (0, 0));
    }

    #[test]
    fn names_that_cant_be_logins_are_listed_as_not_found() {
        let layout = parse(r#"{"windows":[{"tabs":[{"splits2":{"type":"split","data":{"type":"twitch","name":"no such/name"}}}]}]}"#).unwrap();
        let plan = plan(&layout, &found(&layout, &[]), &Tabs::default(), PLENTY);
        assert_eq!(plan.report.unresolved, ["no such/name"]);
        assert!(plan.tabs.is_empty());
    }

    /// Real layouts from a folder, counts only (never names or titles):
    /// `CYEX_CHATTERINO_LAYOUTS=<dir> cargo test -p cyex-settings real_layouts -- --ignored --nocapture`
    #[test]
    #[ignore = "reads CYEX_CHATTERINO_LAYOUTS"]
    fn real_layouts() {
        let Ok(dir) = std::env::var("CYEX_CHATTERINO_LAYOUTS") else {
            return;
        };
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        for path in files {
            let file = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
            let Ok(layout) = parse(&text) else {
                println!("{file}: not a layout");
                continue;
            };
            let windows = serde_json::from_str::<serde_json::Value>(&text).unwrap()["windows"]
                .as_array()
                .map_or(0, Vec::len);
            let all = found(&layout, &[]);
            let p = plan(&layout, &all, &Tabs::default(), PLENTY);
            let r = &p.report;
            let skipped: Vec<String> = r
                .not_channels
                .iter()
                .map(|n| format!("{:?} {}", n.kind, n.count))
                .collect();
            let aliases = p.tabs.iter().filter(|t| t.alias.is_some()).count();
            println!(
                "{file}: windows {windows}, tabs with channels {}, channels {}, unique {}, not channels [{}]; \
                 import: tabs {}, channels {}, aliases {aliases}, already open {}, over limit {}, not found {}",
                layout.tabs.len(),
                layout.channel_count(),
                names(&layout).len(),
                skipped.join(", "),
                r.tabs,
                r.channels,
                r.already_open.len(),
                r.over_limit.len(),
                r.unresolved.len(),
            );
        }
    }

    #[test]
    fn reports_reach_the_ui_in_camel_case() {
        let report = ImportReport {
            not_channels: vec![NotChannel {
                kind: SplitKind::YouTube,
                count: 1,
            }],
            ..ImportReport::default()
        };
        let json = serde_json::to_value(report).unwrap();
        assert_eq!(json["notChannels"][0]["kind"], "youtube");
        assert!(json["overLimit"].is_array());
    }
}
