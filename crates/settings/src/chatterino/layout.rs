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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    /// Tabs with at least one Twitch channel: the windows in order, then their tabs.
    pub tabs: Vec<LayoutTab>,
    /// Splits that aren't Twitch channels, by kind.
    pub skipped: BTreeMap<SplitKind, u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutTab {
    /// The custom title, if the tab has one.
    pub title: Option<String>,
    /// Channel names, lowercase without `#`: left to right, each column top to bottom.
    pub columns: Vec<Vec<String>>,
}

/// The file isn't a window layout.
#[derive(Debug, PartialEq, Eq)]
pub struct NotALayout;

impl LayoutTab {
    pub fn channels(&self) -> impl Iterator<Item = &String> {
        self.columns.iter().flatten()
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
    /// older files have `splits`, a list of columns. A horizontal root keeps its columns;
    /// anything else becomes one column, except a lone merged split, which spreads out.
    fn columns(&mut self, tab: &Value) -> Vec<Vec<String>> {
        let columns = match tab.get("splits2").filter(|r| r.is_object()) {
            Some(root) => match kind(root) {
                "split" => self
                    .split(root.get("data"))
                    .into_iter()
                    .map(|c| vec![c])
                    .collect(),
                "horizontal" => root.get("items").map_or_else(Vec::new, |items| {
                    list(items).map(|item| self.leaves(item)).collect()
                }),
                "vertical" => vec![self.leaves(root)],
                _ => Vec::new(),
            },
            None => tab.get("splits").map_or_else(Vec::new, |cols| {
                list(cols)
                    .map(|col| list(col).flat_map(|s| self.split(Some(s))).collect())
                    .collect()
            }),
        };
        columns
            .into_iter()
            .filter(|c: &Vec<String>| !c.is_empty())
            .collect()
    }

    /// Every channel under a node, in order. Like Chatterino, an item that isn't a split
    /// is read as a container.
    fn leaves(&mut self, node: &Value) -> Vec<String> {
        if kind(node) == "split" {
            return self.split(node.get("data"));
        }
        let items: Vec<&Value> = node.get("items").map(|i| list(i).collect()).unwrap_or_default();
        items.into_iter().flat_map(|item| self.leaves(item)).collect()
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

    fn tab(title: Option<&str>, columns: &[&[&str]]) -> LayoutTab {
        LayoutTab {
            title: title.map(Into::into),
            columns: columns
                .iter()
                .map(|c| c.iter().map(|s| (*s).to_owned()).collect())
                .collect(),
        }
    }

    #[test]
    fn current_layout_keeps_windows_tabs_and_splits_in_order() {
        let layout = parse(&fixture("current.json")).unwrap();
        assert_eq!(
            layout.tabs,
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
            layout.tabs,
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
            layout.tabs,
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
}
