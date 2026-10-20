//! V1041-02 What a read layout adds to the tabs, in Chatterino's order. C1 Like
//! Chatterino, a channel goes in everywhere the file has it, also when another tab shows
//! it already: those share one chat room. Nothing is dropped without a line in the report.

use std::collections::HashMap;

use serde::Serialize;
use ts_rs::TS;

use super::{Layout, LayoutTab, SplitKind};
use crate::schema::{ImportColumn, ImportTab, TabChannel};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    /// The tabs to add, in Chatterino's order.
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
    /// Splits that aren't Twitch channels.
    pub not_channels: Vec<NotChannel>,
    /// Names Twitch doesn't know: renamed, banned or deleted.
    pub unresolved: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NotChannel {
    pub kind: SplitKind,
    pub count: u32,
}

/// `found` maps logins to channels (see `resolve`).
pub fn plan(layout: &Layout, found: &HashMap<String, TabChannel>) -> Plan {
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
            for split in &column.splits {
                match found.get(&split.name) {
                    Some(channel) => tiles.push((channel.clone(), split.size)),
                    None => add_once(&mut report.unresolved, &split.name),
                }
            }
            if !tiles.is_empty() {
                columns.push(ImportColumn {
                    size: column.size,
                    tiles,
                });
            }
        }
        if columns.is_empty() {
            continue;
        }
        report.tabs += 1;
        report.channels += columns.iter().map(|c| c.tiles.len()).sum::<usize>() as u32;
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

    fn fixture(name: &str) -> Layout {
        let path = format!("{}/tests/fixtures/chatterino/{name}", env!("CARGO_MANIFEST_DIR"));
        parse(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// Looks the layout up on a fake Twitch that knows every name but `unknown`.
    fn found(layout: &Layout, unknown: &[&str]) -> HashMap<String, TabChannel> {
        now(resolve(&names(layout), &HashMap::new(), |batch| {
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
            .map(|c| c.tiles.iter().map(|(x, _)| x.login.as_str()).collect())
            .collect()
    }

    #[test]
    fn tabs_keep_their_order_titles_and_splits() {
        let layout = fixture("current.json");
        let plan = plan(&layout, &found(&layout, &["golf_seven"]));

        let got: Vec<(Option<&str>, Vec<Vec<&str>>)> = plan
            .tabs
            .iter()
            .map(|t| (t.alias.as_deref(), logins(t)))
            .collect();
        assert_eq!(
            got,
            [
                (Some("Morning"), vec![vec!["alpha_one"]]),
                (None, vec![vec!["bravo_two"], vec!["charlie_three"]]),
                // Just the channel's name: no alias, so renames follow (1.15).
                (None, vec![vec!["delta_four"]]),
                (Some("🎮 Mixed"), vec![vec!["echo_five"]]),
                (Some("Pop"), vec![vec!["foxtrot_six"]]),
                // C1 In the file twice: in both tabs, like Chatterino shows it.
                (None, vec![vec!["alpha_one"]]),
            ]
        );
        let r = &plan.report;
        assert_eq!((r.tabs, r.channels), (6, 7));
        assert_eq!(r.unresolved, ["golf_seven"]);
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

    /// C1 The user's layout: lilb_lxryer and jannik_vdb have tabs of their own and are
    /// in Comfi's columns too. Every split goes in, with Chatterino's sizes.
    #[test]
    fn c1_channels_in_two_tabs_go_in_both() {
        let layout = fixture("comfi-tabs.json");
        let plan = plan(&layout, &found(&layout, &[]));
        let got: Vec<(Option<&str>, Vec<Vec<&str>>)> = plan
            .tabs
            .iter()
            .map(|t| (t.alias.as_deref(), logins(t)))
            .collect();
        assert_eq!(
            got,
            [
                (None, vec![vec!["lilb_lxryer"]]),
                (None, vec![vec!["jannik_vdb"]]),
                (
                    Some("Comfi"),
                    vec![
                        vec!["zjbw", "aussieantics"],
                        vec!["lilb_lxryer", "cakefire_"],
                        vec!["cnfz", "jannik_vdb"],
                        vec!["stableronaldo", "rvbies", "potatbotat"],
                    ]
                ),
            ]
        );
        assert_eq!((plan.report.tabs, plan.report.channels), (3, 11));
        let col = &plan.tabs[2].columns[1];
        let tall = col.tiles[0].1 / col.tiles[1].1;
        assert!((tall - 1.637_630_7 / 0.362_369_33).abs() < 1e-3, "{tall}");
    }

    /// A name Twitch doesn't know leaves its column's other splits and sizes alone.
    #[test]
    fn unknown_names_drop_only_their_split() {
        let layout = fixture("comfi.json");
        let plan = plan(&layout, &found(&layout, &["lilb_lxryer"]));
        let col = &plan.tabs[0].columns[1];
        assert_eq!(col.tiles.len(), 1);
        assert_eq!(col.tiles[0].0.login, "cakefire_");
        assert_eq!(plan.report.unresolved, ["lilb_lxryer"]);
        assert_eq!(plan.tabs[0].columns.len(), 4);
    }

    /// V107-R20 The user's app had a lilb_lxryer tab before importing Comfi, and the login
    /// lookup missed that name. The open tab's channel fills in, so column 2 keeps both
    /// splits, and the open tab stays as it was.
    #[test]
    fn r20_channel_with_its_own_tab_before_import_goes_in() {
        use crate::chatterino::resolve::known;
        use crate::schema::{Column, Tab, Tabs, Tile, TileShow};

        let open = TabChannel {
            id: "676966284".into(),
            login: "lilb_lxryer".into(),
            name: "lilb_lxryer".into(),
        };
        let mut tabs = Tabs {
            tabs: vec![Tab {
                id: "own".into(),
                columns: vec![Column {
                    size: 1.0,
                    tiles: vec![Tile {
                        id: "own-1".into(),
                        channel: open.clone(),
                        account: Some("other".into()),
                        ..Tile::default()
                    }],
                }],
                ..Tab::default()
            }],
            ..Tabs::default()
        };
        let before = tabs.tabs[0].clone();
        let layout = fixture("comfi.json");
        let mut asked = Vec::new();
        let found = now(resolve(&names(&layout), &known(&tabs), |batch| {
            asked.extend(batch.clone());
            // Twitch's login lookup misses lilb_lxryer.
            let answer: Vec<TabChannel> = batch.iter().filter(|n| *n != "lilb_lxryer").map(|n| channel(n)).collect();
            async move { Ok::<_, ()>(answer) }
        }))
        .unwrap();
        assert!(!asked.iter().any(|n| n == "lilb_lxryer"));

        let plan = plan(&layout, &found);
        assert!(plan.report.unresolved.is_empty(), "{:?}", plan.report.unresolved);
        assert_eq!(logins(&plan.tabs[0])[1], ["lilb_lxryer", "cakefire_"]);
        assert_eq!(plan.tabs[0].columns[1].tiles[0].0, open);

        let mut n = 0;
        tabs.import(plan.tabs, TileShow::default(), Some("me".into()), &mut || {
            n += 1;
            format!("n{n}")
        });
        assert_eq!(tabs.tabs[0], before);
        let comfi = &tabs.tabs[1];
        let col: Vec<&str> = comfi.columns[1].tiles.iter().map(|t| t.channel.login.as_str()).collect();
        assert_eq!(col, ["lilb_lxryer", "cakefire_"]);
        assert_eq!(comfi.tiles().count(), 9);
    }

    #[test]
    fn nothing_to_import() {
        let layout = parse(
            r#"{"windows":[{"tabs":[{"title":"x","splits2":{"type":"split","data":{"type":"mentions"}}}]}]}"#,
        )
        .unwrap();
        let plan = plan(&layout, &HashMap::new());
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
        let plan = plan(&layout, &found(&layout, &[]));
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
            let p = plan(&layout, &all);
            let r = &p.report;
            let skipped: Vec<String> = r
                .not_channels
                .iter()
                .map(|n| format!("{:?} {}", n.kind, n.count))
                .collect();
            let aliases = p.tabs.iter().filter(|t| t.alias.is_some()).count();
            println!(
                "{file}: windows {windows}, tabs with channels {}, channels {}, unique {}, not channels [{}]; \
                 import: tabs {}, channels {}, aliases {aliases}, not found {}",
                layout.tabs.len(),
                layout.channel_count(),
                names(&layout).len(),
                skipped.join(", "),
                r.tabs,
                r.channels,
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
        assert!(json["unresolved"].is_array());
        assert!(json.get("alreadyOpen").is_none());
    }
}
