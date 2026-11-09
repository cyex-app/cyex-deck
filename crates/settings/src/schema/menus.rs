//! V110-U67 How the right-click and ⋯ menus are arranged. One layout per menu id
//! (`chat.name`, `tabs.tab`, …). A menu without a layout uses its built-in one, so this
//! file only holds what the user changed. The items themselves live in the UI's menu registry.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// Most nodes a layout may hold, groups and their children counted.
pub const MAX_NODES: usize = 100;
/// Most menus that can have a layout.
pub const MAX_LAYOUTS: usize = 40;
/// Longest label, in characters.
pub const MAX_LABEL: usize = 40;
const MAX_ID: usize = 64;
const MAX_KNOWN: usize = 300;
const MAX_TEMPLATE: usize = 500;

/// Layouts by menu id. A missing key means the built-in layout; resetting a menu sends
/// `{layouts: {"chat.name": null}}`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct MenuLayouts {
    pub layouts: BTreeMap<String, MenuLayout>,
}

/// One menu as the user arranged it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct MenuLayout {
    pub nodes: Vec<MenuNode>,
    /// Every registry item id this layout has seen. An id the registry has that is neither
    /// here nor in the tree is new, and the app places it.
    pub known: Vec<String>,
}

/// A row of the menu. One level of groups: a group's children are never groups.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
#[ts(export)]
pub enum MenuNode {
    Item {
        id: String,
        /// Own name for the row, `null` keeps the built-in one.
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        hidden: bool,
    },
    /// A row that opens a submenu.
    Group {
        id: String,
        label: String,
        #[serde(default)]
        icon: Option<String>,
        #[serde(default)]
        hidden: bool,
        #[serde(default)]
        children: Vec<MenuNode>,
    },
    /// A line between rows.
    Sep,
    /// An entry the user made, no code.
    Custom {
        id: String,
        label: String,
        #[serde(default)]
        icon: Option<String>,
        #[serde(default)]
        hidden: bool,
        action: CustomAction,
    },
}

/// What a custom entry does. Values may hold `$(nick)`, `$(display-nick)`, `$(user-id)`,
/// `$(msg-id)`, `$(msg)` and `$(chan)`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum CustomAction {
    /// An http or https link; placeholder values are URL-encoded.
    Link { url: String },
    /// Sends this in the chat the menu belongs to.
    Chat { command: String },
    /// Puts this in the message box.
    Prefill { text: String },
    /// Copies this.
    Copy { text: String },
}

impl Schema for MenuLayouts {
    fn sanitize(&mut self) {
        self.layouts.retain(|k, _| valid_id(k));
        while self.layouts.len() > MAX_LAYOUTS {
            self.layouts.pop_last();
        }
        for layout in self.layouts.values_mut() {
            layout.sanitize();
        }
    }
}

impl MenuLayout {
    fn sanitize(&mut self) {
        let mut seen = HashSet::new();
        let mut budget = MAX_NODES;
        let nodes = std::mem::take(&mut self.nodes);
        self.nodes = clean(nodes, true, &mut seen, &mut budget);

        let mut known_seen = HashSet::new();
        self.known.retain(|id| valid_id(id) && known_seen.insert(id.clone()));
        self.known.truncate(MAX_KNOWN);
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().count() <= MAX_ID && !id.chars().any(char::is_control)
}

fn label(s: &str) -> String {
    s.trim().chars().filter(|c| !c.is_control()).take(MAX_LABEL).collect::<String>().trim().to_string()
}

fn icon(s: Option<String>) -> Option<String> {
    s.filter(|s| !s.is_empty() && s.len() <= 40 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

fn text(s: &str) -> String {
    s.chars().take(MAX_TEMPLATE).collect()
}

fn is_web(url: &str) -> bool {
    let lower = url.trim_start().to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

fn action(a: CustomAction) -> Option<CustomAction> {
    Some(match a {
        CustomAction::Link { url } => {
            let url = text(url.trim());
            if !is_web(&url) {
                return None;
            }
            CustomAction::Link { url }
        }
        CustomAction::Chat { command } => {
            let command = text(command.trim());
            if command.is_empty() {
                return None;
            }
            CustomAction::Chat { command }
        }
        CustomAction::Prefill { text: t } => CustomAction::Prefill { text: text(&t) },
        CustomAction::Copy { text: t } => {
            let t = text(&t);
            if t.is_empty() {
                return None;
            }
            CustomAction::Copy { text: t }
        }
    })
}

/// Repairs one level. `top` is true at the root; a group found below the root is dissolved
/// into its children, so there is never more than one level.
fn clean(nodes: Vec<MenuNode>, top: bool, seen: &mut HashSet<String>, budget: &mut usize) -> Vec<MenuNode> {
    let mut out = Vec::new();
    for node in nodes {
        if *budget == 0 {
            break;
        }
        match node {
            MenuNode::Sep => {
                *budget -= 1;
                out.push(MenuNode::Sep);
            }
            MenuNode::Item { id, label: l, hidden } => {
                if !valid_id(&id) || !seen.insert(id.clone()) {
                    continue;
                }
                *budget -= 1;
                let l = l.map(|l| label(&l)).filter(|l| !l.is_empty());
                out.push(MenuNode::Item { id, label: l, hidden });
            }
            MenuNode::Custom { id, label: l, icon: i, hidden, action: a } => {
                let l = label(&l);
                let Some(a) = action(a) else { continue };
                if l.is_empty() || !valid_id(&id) || !seen.insert(id.clone()) {
                    continue;
                }
                *budget -= 1;
                out.push(MenuNode::Custom { id, label: l, icon: icon(i), hidden, action: a });
            }
            MenuNode::Group { id, label: l, icon: i, hidden, children } => {
                if !top {
                    // Too deep: keep the rows, lose the wrapper.
                    out.extend(clean(children, false, seen, budget));
                    continue;
                }
                if !valid_id(&id) || !seen.insert(id.clone()) {
                    continue;
                }
                *budget -= 1;
                let l = label(&l);
                let l = if l.is_empty() { "Group".to_string() } else { l };
                let children = clean(children, false, seen, budget);
                out.push(MenuNode::Group { id, label: l, icon: icon(i), hidden, children });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(id: &str) -> MenuNode {
        MenuNode::Item { id: id.into(), label: None, hidden: false }
    }
    fn group(id: &str, children: Vec<MenuNode>) -> MenuNode {
        MenuNode::Group { id: id.into(), label: id.into(), icon: None, hidden: false, children }
    }
    fn layouts(nodes: Vec<MenuNode>) -> MenuLayouts {
        let mut m = MenuLayouts::default();
        m.layouts.insert("chat.name".into(), MenuLayout { nodes, known: vec![] });
        m.sanitize();
        m
    }
    fn nodes(m: &MenuLayouts) -> &Vec<MenuNode> {
        &m.layouts["chat.name"].nodes
    }

    #[test]
    fn round_trips_every_kind() {
        let v = json!({"layouts": {"chat.name": {
            "nodes": [
                {"t": "item", "id": "usercard", "label": "Card", "hidden": true},
                {"t": "sep"},
                {"t": "group", "id": "g1", "label": "More", "icon": "list", "hidden": false,
                 "children": [{"t": "item", "id": "copy"}]},
                {"t": "custom", "id": "c1", "label": "Search", "icon": null, "hidden": false,
                 "action": {"kind": "link", "url": "https://example.com/?q=$(nick)"}},
            ],
            "known": ["usercard", "copy"],
        }}});
        let mut m: MenuLayouts = serde_json::from_value(v.clone()).unwrap();
        m.sanitize();
        let back = serde_json::to_value(&m).unwrap();
        let again: MenuLayouts = serde_json::from_value(back).unwrap();
        assert_eq!(m, again);
        assert_eq!(nodes(&m).len(), 4);
        assert_eq!(MenuLayouts::default(), serde_json::from_str("{}").unwrap());
    }

    #[test]
    fn groups_stay_one_level() {
        let m = layouts(vec![group("a", vec![item("x"), group("b", vec![item("y")])])]);
        let MenuNode::Group { children, .. } = &nodes(&m)[0] else { panic!() };
        assert_eq!(children, &vec![item("x"), item("y")]);
    }

    #[test]
    fn at_most_one_hundred_nodes() {
        let many = (0..250).map(|i| item(&format!("i{i}"))).collect();
        assert_eq!(nodes(&layouts(many)).len(), MAX_NODES);
        // Children count too.
        let m = layouts(vec![group("g", (0..250).map(|i| item(&format!("i{i}"))).collect())]);
        let MenuNode::Group { children, .. } = &nodes(&m)[0] else { panic!() };
        assert_eq!(children.len(), MAX_NODES - 1);
    }

    #[test]
    fn duplicate_and_empty_ids_go() {
        let m = layouts(vec![item("a"), item("a"), item(""), group("g", vec![item("a"), item("b")]), group("a", vec![])]);
        assert_eq!(nodes(&m), &vec![item("a"), group("g", vec![item("b")])]);
    }

    #[test]
    fn labels_are_trimmed_and_capped() {
        let long = "x".repeat(90);
        let m = layouts(vec![
            MenuNode::Item { id: "a".into(), label: Some(format!("  {long} ")), hidden: false },
            MenuNode::Item { id: "b".into(), label: Some("   ".into()), hidden: false },
            MenuNode::Group { id: "g".into(), label: " ".into(), icon: Some("bad icon!".into()), hidden: false, children: vec![] },
        ]);
        let MenuNode::Item { label: l, .. } = &nodes(&m)[0] else { panic!() };
        assert_eq!(l.as_deref().map(|l| l.chars().count()), Some(MAX_LABEL));
        let MenuNode::Item { label: l, .. } = &nodes(&m)[1] else { panic!() };
        assert_eq!(l, &None);
        let MenuNode::Group { label: l, icon, .. } = &nodes(&m)[2] else { panic!() };
        assert_eq!((l.as_str(), icon), ("Group", &None));
    }

    #[test]
    fn custom_links_are_web_only() {
        let c = |url: &str| MenuNode::Custom {
            id: url.len().to_string(),
            label: "x".into(),
            icon: None,
            hidden: false,
            action: CustomAction::Link { url: url.into() },
        };
        let m = layouts(vec![c("javascript:alert(1)"), c("file:///c:/x"), c("https://a.b/$(nick)"), c("HTTP://a.b")]);
        assert_eq!(nodes(&m).len(), 2);
        // No label, no entry.
        let m = layouts(vec![MenuNode::Custom {
            id: "c".into(),
            label: " ".into(),
            icon: None,
            hidden: false,
            action: CustomAction::Copy { text: "hi".into() },
        }]);
        assert!(nodes(&m).is_empty());
    }

    #[test]
    fn known_is_deduped_and_capped() {
        let mut m = MenuLayouts::default();
        let known: Vec<String> = (0..400).map(|i| format!("k{}", i % 350)).collect();
        m.layouts.insert("tabs.tab".into(), MenuLayout { nodes: vec![], known });
        m.sanitize();
        let k = &m.layouts["tabs.tab"].known;
        assert_eq!(k.len(), MAX_KNOWN);
        assert_eq!(k.iter().collect::<HashSet<_>>().len(), k.len());
    }

    #[test]
    fn too_many_layouts_are_cut() {
        let mut m = MenuLayouts::default();
        for i in 0..60 {
            m.layouts.insert(format!("m{i:02}"), MenuLayout::default());
        }
        m.sanitize();
        assert_eq!(m.layouts.len(), MAX_LAYOUTS);
    }

    #[test]
    fn null_resets_one_menu() {
        use crate::merge::merge_patch;
        let mut cur = serde_json::to_value(layouts(vec![item("a")])).unwrap();
        cur["layouts"]["tabs.tab"] = json!({"nodes": [], "known": []});
        merge_patch(&mut cur, &json!({"layouts": {"chat.name": null}}));
        assert!(cur["layouts"].get("chat.name").is_none());
        assert!(cur["layouts"].get("tabs.tab").is_some());
    }
}
