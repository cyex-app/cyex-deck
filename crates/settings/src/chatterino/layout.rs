//! V1041-02 Reads Chatterino's `window-layout.json` and its `.bkp-N` copies: upstream 2.1
//! to now, the old `splits` columns and the 7TV, technorino and moltorino forks (see
//! `docs/research/chatterino-layout.md`). Only what the import needs: tabs in order, their
//! custom titles and Twitch channels; everything else is counted by kind.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

/// What a split shows when it isn't a plain Twitch channel. Named like Chatterino's types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum SplitKind {
    Mentions,
    Whispers,
    Live,
    Watching,
    Automod,
    Irc,
    Kick,
    YouTube,
    TikTok,
    /// Chatterino's own test and special channels.
    Misc,
    /// A split with no channel picked.
    Empty,
    /// A type this reader doesn't know.
    Other,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    /// Tabs with at least one Twitch channel: the windows in order, then their tabs.
    pub tabs: Vec<LayoutTab>,
    /// Splits that aren't Twitch channels, by kind.
    pub skipped: BTreeMap<SplitKind, u32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutTab {
    /// The custom title, if the tab has one.
    pub title: Option<String>,
    /// Left to right, each column top to bottom.
    pub columns: Vec<LayoutColumn>,
}

/// C1 Sizes are shares like Chatterino's `flexh` and `flexv`: only their ratio counts.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutColumn {
    /// Width share within the tab.
    pub size: f32,
    pub splits: Vec<LayoutSplit>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSplit {
    /// Lowercase, without `#`.
    pub name: String,
    /// Height share within the column.
    pub size: f32,
}

/// The file isn't a window layout.
#[derive(Debug, PartialEq, Eq)]
pub struct NotALayout;

impl LayoutTab {
    pub fn channels(&self) -> impl Iterator<Item = &String> {
        self.columns.iter().flat_map(|c| c.splits.iter().map(|s| &s.name))
    }
}

impl Layout {
    pub fn channel_count(&self) -> usize {
        self.tabs.iter().map(|t| t.channels().count()).sum()
    }
}

/// Like Chatterino: the root must be an object with a `windows` array; anything below
/// that it can't read is passed over.
pub fn parse(text: &str) -> Result<Layout, NotALayout> {
    let root: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|_| NotALayout)?;
    let windows = root.get("windows").and_then(Value::as_array).ok_or(NotALayout)?;
    let mut layout = Layout::default();
    let tabs = windows.iter().filter_map(|w| w.get("tabs")?.as_array()).flatten();
    for tab in tabs {
        let columns = layout.columns(tab);
        if columns.is_empty() {
            continue;
        }
        let title = tab
            .get("title")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|t| !t.is_empty());
        layout.tabs.push(LayoutTab {
            title: title.map(str::to_owned),
            columns,
        });
    }
    Ok(layout)
}

impl Layout {
    /// `splits2` is a tree of splits in horizontal and vertical containers since 2.0;
    /// older files have `splits`, a list of columns. A horizontal container's items are
    /// columns (a horizontal one inside spreads into more columns); anything below a
    /// column is stacked in order. A lone merged split spreads out. C1 Sizes follow
    /// `flexh` and `flexv` at every depth.
    fn columns(&mut self, tab: &Value) -> Vec<LayoutColumn> {
        let mut columns = Vec::new();
        match tab.get("splits2").filter(|r| r.is_object()) {
            Some(root) => match kind(root) {
                "split" => {
                    for name in self.split(root.get("data")) {
                        columns.push(column(1.0, vec![LayoutSplit { name, size: 1.0 }]));
                    }
                }
                "horizontal" => self.row(root, 1.0, &mut columns),
                "vertical" => {
                    let mut splits = Vec::new();
                    self.stack(root, 1.0, &mut splits);
                    columns.push(column(1.0, splits));
                }
                _ => {}
            },
            None => {
                for col in tab.get("splits").map(list).into_iter().flatten() {
                    let splits = list(col)
                        .flat_map(|s| self.split(Some(s)))
                        .map(|name| LayoutSplit { name, size: 1.0 })
                        .collect();
                    columns.push(column(1.0, splits));
                }
            }
        }
        columns.retain(|c| !c.splits.is_empty());
        columns
    }

    /// A horizontal container's items as columns, `share` of the tab's width split by
    /// their `flexh`.
    fn row(&mut self, node: &Value, share: f32, out: &mut Vec<LayoutColumn>) {
        for (item, part) in shares(node, "flexh", share) {
            if kind(item) == "horizontal" {
                self.row(item, part, out);
            } else {
                let mut splits = Vec::new();
                self.stack(item, 1.0, &mut splits);
                out.push(column(part, splits));
            }
        }
    }

    /// Every channel under a node, in order, `share` of the column's height each. A
    /// vertical container splits it by `flexv`; anything else that isn't a split (like
    /// Chatterino, read as a container) is stacked here, split by `flexh`.
    fn stack(&mut self, node: &Value, share: f32, out: &mut Vec<LayoutSplit>) {
        if kind(node) == "split" {
            let names = self.split(node.get("data"));
            let each = share / names.len().max(1) as f32;
            out.extend(names.into_iter().map(|name| LayoutSplit { name, size: each }));
            return;
        }
        let axis = if kind(node) == "vertical" { "flexv" } else { "flexh" };
        for (item, part) in shares(node, axis, share) {
            self.stack(item, part, out);
        }
    }

    /// A split's channels: one, or a merged split's Twitch ones (forks). The rest is
    /// counted.
    fn split(&mut self, data: Option<&Value>) -> Vec<String> {
        let Some(data) = data else {
            self.skip(SplitKind::Empty);
            return Vec::new();
        };
        let found = match kind(data) {
            "twitch" => {
                // Old and IRC splits write `channel`, newer ones `name`.
                let name = data.get("channel").or_else(|| data.get("name"));
                Ok(name.and_then(Value::as_str).map(clean).into_iter().collect())
            }
            "multi" => Ok(self.merged(data)),
            "mentions" => Err(SplitKind::Mentions),
            "whispers" => Err(SplitKind::Whispers),
            "live" => Err(SplitKind::Live),
            "watching" => Err(SplitKind::Watching),
            "automod" => Err(SplitKind::Automod),
            "irc" => Err(SplitKind::Irc),
            "kick" => Err(SplitKind::Kick),
            "youtube" => Err(SplitKind::YouTube),
            "tiktok" => Err(SplitKind::TikTok),
            "misc" => Err(SplitKind::Misc),
            // An empty split: "" now, "None" in some older builds.
            "" | "None" | "none" => Err(SplitKind::Empty),
            _ => Err(SplitKind::Other),
        };
        match found {
            Ok(names) => {
                let names: Vec<String> = names.into_iter().filter(|n| !n.is_empty()).collect();
                if names.is_empty() {
                    self.skip(SplitKind::Empty);
                }
                names
            }
            Err(kind) => {
                self.skip(kind);
                Vec::new()
            }
        }
    }

    /// 7TV's merged chat: `children` of `{platform, channel}`.
    fn merged(&mut self, data: &Value) -> Vec<String> {
        let children: Vec<&Value> = data
            .get("children")
            .map(|c| list(c).collect())
            .unwrap_or_default();
        let mut names = Vec::new();
        for child in children {
            let platform = child.get("platform").and_then(Value::as_str).unwrap_or("");
            let name = child
                .get("channel")
                .and_then(Value::as_str)
                .map(clean)
                .unwrap_or_default();
            if platform.eq_ignore_ascii_case("twitch") {
                names.push(name);
            } else if platform.eq_ignore_ascii_case("kick") {
                self.skip(SplitKind::Kick);
            } else {
                self.skip(SplitKind::Other);
            }
        }
        names
    }

    fn skip(&mut self, kind: SplitKind) {
        *self.skipped.entry(kind).or_default() += 1;
    }
}

fn kind(v: &Value) -> &str {
    v.get("type").and_then(Value::as_str).unwrap_or("")
}

fn column(size: f32, splits: Vec<LayoutSplit>) -> LayoutColumn {
    LayoutColumn { size, splits }
}

/// A container's items with their part of `share`, by `axis` (`flexh` or `flexv`).
/// A missing, zero or broken flex counts as 1, like Chatterino before 2.3.
fn shares<'a>(node: &'a Value, axis: &str, share: f32) -> Vec<(&'a Value, f32)> {
    let items: Vec<&Value> = node.get("items").map(|i| list(i).collect()).unwrap_or_default();
    let flex = |v: &Value| {
        let f = v.get(axis).and_then(Value::as_f64).unwrap_or(1.0) as f32;
        if f.is_finite() && f > 0.0 { f } else { 1.0 }
    };
    let sum: f32 = items.iter().map(|i| flex(i)).sum();
    items.into_iter().map(|i| (i, share * flex(i) / sum)).collect()
}

fn list(v: &Value) -> impl Iterator<Item = &Value> {
    v.as_array().into_iter().flatten()
}

/// Chatterino keeps names as typed; Twitch logins are lowercase, without `#`.
fn clean(name: &str) -> String {
    name.trim().trim_start_matches('#').trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let path = format!("{}/tests/fixtures/chatterino/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(path).unwrap()
    }

type Shape = (Option<String>, Vec<Vec<String>>);

    fn tab(title: Option<&str>, columns: &[&[&str]]) -> Shape {
        let columns = columns
            .iter()
            .map(|c| c.iter().map(|s| (*s).to_owned()).collect())
            .collect();
        (title.map(Into::into), columns)
    }

    /// Titles and channel names, sizes left out.
    fn shape(tabs: &[LayoutTab]) -> Vec<Shape> {
        tabs.iter()
            .map(|t| {
                let columns = t
                    .columns
                    .iter()
                    .map(|c| c.splits.iter().map(|s| s.name.clone()).collect())
                    .collect();
                (t.title.clone(), columns)
            })
            .collect()
    }

    #[test]
    fn current_layout_keeps_windows_tabs_and_splits_in_order() {
        let layout = parse(&fixture("current.json")).unwrap();
        assert_eq!(
            shape(&layout.tabs),
            [
                tab(Some("Morning"), &[&["alpha_one"]]),
                tab(None, &[&["bravo_two"], &["charlie_three"]]),
                tab(Some("Delta_Four"), &[&["delta_four"]]),
                tab(Some("🎮 Mixed"), &[&["echo_five", "golf_seven"]]),
                tab(Some("Pop"), &[&["foxtrot_six"]]),
                tab(None, &[&["alpha_one"]]),
            ]
        );
        let skipped: Vec<(SplitKind, u32)> = layout.skipped.into_iter().collect();
        assert_eq!(
            skipped,
            [
                (SplitKind::Mentions, 2),
                (SplitKind::Whispers, 1),
                (SplitKind::Live, 1),
                (SplitKind::Watching, 1),
                (SplitKind::Automod, 1),
                (SplitKind::Misc, 1),
                (SplitKind::Empty, 2),
            ]
        );
    }

    #[test]
    fn old_layouts_read_their_columns_and_irc() {
        let layout = parse(&fixture("legacy.json")).unwrap();
        assert_eq!(
            shape(&layout.tabs),
            [
                tab(Some("Old"), &[&["hotel_eight", "india_nine"], &["juliet_ten"]]),
                tab(None, &[&["kilo_eleven"]]),
            ]
        );
        let skipped: Vec<(SplitKind, u32)> = layout.skipped.into_iter().collect();
        assert_eq!(
            skipped,
            [
                (SplitKind::Whispers, 1),
                (SplitKind::Irc, 2),
                (SplitKind::Empty, 1),
                (SplitKind::Other, 1)
            ]
        );
    }

    #[test]
    fn fork_layouts_keep_twitch_and_count_the_rest() {
        let layout = parse(&fixture("forks.json")).unwrap();
        assert_eq!(
            shape(&layout.tabs),
            [
                tab(Some("Merged"), &[&["lima_twelve"], &["mike_thirteen"]]),
                tab(None, &[&["november_14"], &["oscar_15", "papa_16"]]),
            ]
        );
        let skipped: Vec<(SplitKind, u32)> = layout.skipped.into_iter().collect();
        assert_eq!(
            skipped,
            [
                (SplitKind::Kick, 2),
                (SplitKind::YouTube, 1),
                (SplitKind::TikTok, 1),
                (SplitKind::Other, 1)
            ]
        );
    }

    #[test]
    fn many_channels_parse() {
        let layout = parse(&fixture("many.json")).unwrap();
        assert_eq!((layout.tabs.len(), layout.channel_count()), (110, 120));
        assert!(layout.skipped.is_empty());
    }

    #[test]
    fn files_that_arent_layouts() {
        assert_eq!(parse(&fixture("malformed.json")), Err(NotALayout));
        assert_eq!(parse(&fixture("not-layout.json")), Err(NotALayout));
        for text in ["", "  ", "[]", "null", "{\"windows\":{}}", "{\"windows\":5}"] {
            assert_eq!(parse(text), Err(NotALayout), "{text:?}");
        }
        // A layout with nothing in it is still a layout.
        assert_eq!(parse("\u{feff}{\"windows\":[]}"), Ok(Layout::default()));
        let odd = r#"{"windows":[5,{"tabs":7},{"tabs":[3,{"splits2":[]},{"splits2":{"type":"grid"}}]}]}"#;
        assert_eq!(parse(odd), Ok(Layout::default()));
    }

    /// Sizes as (column share, [split shares]), rounded to 4 places.
    fn sizes(t: &LayoutTab) -> Vec<(f32, Vec<f32>)> {
        let r = |v: f32| (v * 10_000.0).round() / 10_000.0;
        t.columns
            .iter()
            .map(|c| (r(c.size), c.splits.iter().map(|s| r(s.size)).collect()))
            .collect()
    }

    #[test]
    fn c1_comfi_keeps_all_nine_splits_and_their_sizes() {
        let layout = parse(&fixture("comfi.json")).unwrap();
        assert_eq!(
            shape(&layout.tabs),
            [tab(
                Some("Comfi"),
                &[
                    &["zjbw", "aussieantics"],
                    &["lilb_lxryer", "cakefire_"],
                    &["cnfz", "jannik_vdb"],
                    &["stableronaldo", "rvbies", "potatbotat"],
                ]
            )]
        );
        assert_eq!(
            sizes(&layout.tabs[0]),
            [
                (0.25, vec![0.5, 0.5]),
                (0.2518, vec![0.8188, 0.1812]),
                (0.2491, vec![0.5, 0.5]),
                (0.2491, vec![0.3333, 0.3333, 0.3333]),
            ]
        );
        assert!(layout.skipped.is_empty());
    }

    fn split(name: &str, flexh: f32, flexv: f32) -> String {
        format!(r#"{{"type":"split","flexh":{flexh},"flexv":{flexv},"data":{{"type":"twitch","name":"{name}"}}}}"#)
    }

    fn container(kind: &str, flexh: f32, flexv: f32, items: &[String]) -> String {
        format!(
            r#"{{"type":"{kind}","flexh":{flexh},"flexv":{flexv},"items":[{}]}}"#,
            items.join(",")
        )
    }

    fn one_tab(root: &str) -> LayoutTab {
        let text = format!(r#"{{"windows":[{{"tabs":[{{"splits2":{root}}}]}}]}}"#);
        let mut layout = parse(&text).unwrap();
        assert_eq!(layout.tabs.len(), 1);
        layout.tabs.remove(0)
    }

    #[test]
    fn c1_nested_containers_at_any_depth() {
        // horizontal > [vertical > [a, horizontal > [b, vertical > [c, d]]], horizontal > [e, f]]
        let inner = container("vertical", 1.0, 1.0, &[split("c", 1.0, 3.0), split("d", 1.0, 1.0)]);
        let middle = container("horizontal", 1.0, 3.0, &[split("b", 1.0, 1.0), inner]);
        let deep = container("vertical", 3.0, 1.0, &[split("a", 1.0, 1.0), middle]);
        let side = container("horizontal", 1.0, 1.0, &[split("e", 1.0, 1.0), split("f", 3.0, 1.0)]);
        let t = one_tab(&container("horizontal", 1.0, 1.0, &[deep, side]));
        assert_eq!(
            shape(std::slice::from_ref(&t)),
            [tab(None, &[&["a", "b", "c", "d"], &["e"], &["f"]])]
        );
        // a gets a quarter of its column; b and c + d split the rest in half, c + d by 3:1.
        // The horizontal one beside it spreads into two columns.
        assert_eq!(
            sizes(&t),
            [
                (0.75, vec![0.25, 0.375, 0.2813, 0.0938]),
                (0.0625, vec![1.0]),
                (0.1875, vec![1.0]),
            ]
        );
    }

    #[test]
    fn c1_single_items_three_items_and_missing_flex() {
        let single = container("vertical", 1.0, 1.0, &[split("solo", 1.0, 1.0)]);
        let three = container(
            "vertical",
            2.0,
            1.0,
            &[split("x", 1.0, 1.0), split("y", 1.0, 2.0), split("z", 1.0, 1.0)],
        );
        // 2.1 wrote no flex at all; zero and negative ones count as 1 too.
        let bare = r#"{"type":"vertical","flexh":0,"items":[{"type":"split","flexv":-2,"data":{"type":"twitch","name":"p"}},{"type":"split","data":{"type":"twitch","name":"q"}}]}"#;
        let t = one_tab(&container("horizontal", 1.0, 1.0, &[single, three, bare.to_owned()]));
        assert_eq!(
            shape(std::slice::from_ref(&t)),
            [tab(None, &[&["solo"], &["x", "y", "z"], &["p", "q"]])]
        );
        assert_eq!(
            sizes(&t),
            [(0.25, vec![1.0]), (0.5, vec![0.25, 0.5, 0.25]), (0.25, vec![0.5, 0.5])]
        );
        // A vertical root is one column, a lone split too.
        let v = one_tab(&container("vertical", 1.0, 1.0, &[split("m", 1.0, 1.0), split("n", 1.0, 3.0)]));
        assert_eq!(sizes(&v), [(1.0, vec![0.25, 0.75])]);
        assert_eq!(sizes(&one_tab(&split("k", 5.0, 5.0))), [(1.0, vec![1.0])]);
    }

    #[test]
    fn c1_the_same_channel_stays_in_every_split_and_tab() {
        let layout = parse(&fixture("comfi-tabs.json")).unwrap();
        let shapes = shape(&layout.tabs);
        assert_eq!(shapes[0], tab(None, &[&["lilb_lxryer"]]));
        assert_eq!(shapes[1], tab(None, &[&["jannik_vdb"]]));
        assert_eq!(layout.tabs[2].channels().count(), 9);
        let twice = one_tab(&container("vertical", 1.0, 1.0, &[split("dup", 1.0, 1.0), split("dup", 1.0, 1.0)]));
        assert_eq!(twice.channels().collect::<Vec<_>>(), ["dup", "dup"]);
    }
}
