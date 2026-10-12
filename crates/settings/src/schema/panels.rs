//! V103-05, V103-06 One panel layout for tabs, the mod view and the stream manager: a tree
//! of splits whose leaves are panel ids. Sizes are shares of their split. The UI lays it
//! out (`ui/src/lib/panels.ts`); this only stores it and keeps it sane.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Splits deeper than this are cut off.
const MAX_DEPTH: usize = 8;
/// Panels in one layout.
const MAX_PANELS: usize = 32;
const MAX_ID_CHARS: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SplitDir {
    Row,
    Column,
}

/// A panel, or a split of parts side by side (`row`) or one under the other (`column`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export)]
pub enum PanelNode {
    Panel {
        panel: String,
    },
    Split {
        dir: SplitDir,
        parts: Vec<PanelNode>,
        sizes: Vec<f32>,
    },
}

/// Drops what the UI can't lay out: unknown shapes, repeated or overlong ids, empty and
/// one-part splits, a split inside one going the same way, bad sizes. `None` when nothing
/// is left.
pub fn sanitize(node: Option<PanelNode>) -> Option<PanelNode> {
    let mut seen = HashSet::new();
    node.and_then(|n| clean(n, 0, &mut seen))
}

fn clean(node: PanelNode, depth: usize, seen: &mut HashSet<String>) -> Option<PanelNode> {
    match node {
        PanelNode::Panel { panel } => {
            let ok = !panel.is_empty() && panel.chars().count() <= MAX_ID_CHARS && seen.len() < MAX_PANELS;
            (ok && seen.insert(panel.clone())).then_some(PanelNode::Panel { panel })
        }
        PanelNode::Split { dir, parts, sizes } => {
            if depth >= MAX_DEPTH {
                return None;
            }
            let mut out: Vec<(PanelNode, f32)> = Vec::new();
            for (i, part) in parts.into_iter().enumerate() {
                let size = sizes
                    .get(i)
                    .copied()
                    .filter(|s| s.is_finite() && *s > 0.0)
                    .unwrap_or(1.0);
                let Some(part) = clean(part, depth + 1, seen) else {
                    continue;
                };
                match part {
                    // The same way round: its parts join this split.
                    PanelNode::Split {
                        dir: d,
                        parts: inner,
                        sizes: inner_sizes,
                    } if d == dir => {
                        out.extend(inner.into_iter().zip(inner_sizes).map(|(p, s)| (p, s * size)));
                    }
                    other => out.push((other, size)),
                }
            }
            match out.len() {
                0 => None,
                1 => out.pop().map(|(p, _)| p),
                _ => {
                    let total: f32 = out.iter().map(|(_, s)| s).sum();
                    let (parts, sizes) = out.into_iter().map(|(p, s)| (p, s / total)).unzip();
                    Some(PanelNode::Split { dir, parts, sizes })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str) -> PanelNode {
        PanelNode::Panel { panel: id.into() }
    }

    fn s(dir: SplitDir, parts: Vec<PanelNode>, sizes: Vec<f32>) -> PanelNode {
        PanelNode::Split { dir, parts, sizes }
    }

    #[test]
    fn reads_the_ui_shape() {
        let n: PanelNode = serde_json::from_str(
            r#"{"dir":"row","parts":[{"panel":"stream"},{"dir":"column","parts":[{"panel":"modview"},{"panel":"chat"}],"sizes":[0.5,0.5]}],"sizes":[0.7,0.3]}"#,
        )
        .unwrap();
        let PanelNode::Split { dir, parts, .. } = &n else {
            panic!()
        };
        assert_eq!((*dir, parts.len()), (SplitDir::Row, 2));
        let back = serde_json::to_string(&p("chat")).unwrap();
        assert_eq!(back, r#"{"panel":"chat"}"#);
    }

    #[test]
    fn repairs_what_the_ui_cant_use() {
        // One-part splits fold away, empty ones go, repeats are dropped.
        let n = s(
            SplitDir::Row,
            vec![
                s(SplitDir::Column, vec![p("a")], vec![1.0]),
                s(SplitDir::Row, vec![], vec![]),
                p("a"),
            ],
            vec![0.5, 0.5],
        );
        assert_eq!(sanitize(Some(n)), Some(p("a")));
        assert_eq!(sanitize(Some(p(""))), None);
        // Bad and missing sizes become even; sizes add up to one.
        let n = sanitize(Some(s(
            SplitDir::Row,
            vec![p("a"), p("b"), p("c")],
            vec![f32::NAN, -1.0],
        )))
        .unwrap();
        let PanelNode::Split { sizes, .. } = n else {
            panic!()
        };
        assert!(sizes.iter().all(|x| (x - 1.0 / 3.0).abs() < 1e-6));
    }

    #[test]
    fn a_split_inside_one_going_the_same_way_joins_it() {
        let n = s(
            SplitDir::Row,
            vec![p("a"), s(SplitDir::Row, vec![p("b"), p("c")], vec![0.5, 0.5])],
            vec![0.5, 0.5],
        );
        let Some(PanelNode::Split { parts, sizes, .. }) = sanitize(Some(n)) else {
            panic!()
        };
        assert_eq!(parts, vec![p("a"), p("b"), p("c")]);
        assert_eq!(sizes, vec![0.5, 0.25, 0.25]);
    }

    #[test]
    fn depth_and_count_are_capped() {
        let mut n = p("deep");
        for i in 0..20 {
            let dir = if i % 2 == 0 {
                SplitDir::Row
            } else {
                SplitDir::Column
            };
            n = s(dir, vec![p(&format!("x{i}")), n], vec![0.5, 0.5]);
        }
        let ids = |n: &PanelNode| -> usize {
            fn count(n: &PanelNode) -> usize {
                match n {
                    PanelNode::Panel { .. } => 1,
                    PanelNode::Split { parts, .. } => parts.iter().map(count).sum(),
                }
            }
            count(n)
        };
        assert!(ids(&sanitize(Some(n)).unwrap()) <= MAX_DEPTH + 1);
        let many = s(
            SplitDir::Row,
            (0..50).map(|i| p(&i.to_string())).collect(),
            vec![],
        );
        assert_eq!(ids(&sanitize(Some(many)).unwrap()), MAX_PANELS);
    }
}
