//! V1041-02 Tabs brought in from another app (a Chatterino layout), after the others.

use super::{Column, Schema, Tab, TabChannel, Tabs, Tile, TileShow};

#[derive(Clone, Debug, PartialEq)]
pub struct ImportTab {
    /// 1.22
    pub alias: Option<String>,
    /// U3 Left to right, each column top to bottom.
    pub columns: Vec<ImportColumn>,
}

/// C1 Sizes are shares, as in [`Column::size`] and [`Tile::size`].
#[derive(Clone, Debug, PartialEq)]
pub struct ImportColumn {
    pub size: f32,
    /// Channels with their height share.
    pub tiles: Vec<(TabChannel, f32)>,
}

impl Tabs {
    /// Adds the tabs after the others, joined as `account`, with their sizes. The shown
    /// tab and the add-tab defaults stay as they are. Like H3, only a tab's first channel
    /// plays sound.
    pub fn import(
        &mut self,
        tabs: Vec<ImportTab>,
        show: TileShow,
        account: Option<String>,
        new_id: &mut dyn FnMut() -> String,
    ) {
        for imported in tabs {
            let id = self.fresh(new_id);
            let mut tab = Tab {
                id,
                alias: imported.alias,
                group: None,
                columns: Vec::new(),
            };
            let mut muted = false;
            for column in imported.columns {
                let mut tiles = Vec::with_capacity(column.tiles.len());
                for (channel, size) in column.tiles {
                    // The tab isn't pushed yet, so its own ids are checked here too.
                    let id = loop {
                        let id = self.fresh(new_id);
                        let local = id == tab.id || tiles.iter().chain(tab.tiles()).any(|x: &Tile| x.id == id);
                        if !local {
                            break id;
                        }
                    };
                    tiles.push(Tile {
                        id,
                        channel,
                        show,
                        account: account.clone(),
                        ratio: self.last_ratio,
                        size,
                        volume: self.last_volume,
                        muted,
                        ..Tile::default()
                    });
                    muted = true;
                }
                tab.columns.push(Column { size: column.size, tiles });
            }
            // B3 Chatterino's shares sum to 1; `sanitize` clamps before it normalizes,
            // so a share under the minimum would grow. Mean 1 first keeps the ratio.
            to_mean_one(tab.columns.iter_mut().map(|c| &mut c.size));
            for column in &mut tab.columns {
                to_mean_one(column.tiles.iter_mut().map(|x| &mut x.size));
            }
            self.tabs.push(tab);
        }
        self.sanitize();
    }
}

/// B3 Scales finite, positive shares so they average 1; others are left to `sanitize`.
fn to_mean_one<'a>(sizes: impl Iterator<Item = &'a mut f32>) {
    let mut sizes: Vec<&mut f32> = sizes.collect();
    let good = |s: f32| s.is_finite() && s > 0.0;
    let (sum, n) = sizes.iter().filter(|s| good(***s)).fold((0.0f32, 0usize), |(a, n), s| (a + **s, n + 1));
    if n == 0 || sum <= 0.0 {
        return;
    }
    let mean = sum / n as f32;
    for s in sizes.iter_mut().filter(|s| good(***s)) {
        **s /= mean;
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{counter, order, tab};
    use super::*;

    fn channel(login: &str) -> TabChannel {
        TabChannel {
            id: format!("u{login}"),
            login: login.into(),
            name: login.into(),
        }
    }

    fn col(size: f32, tiles: &[(&str, f32)]) -> ImportColumn {
        let tiles = tiles.iter().map(|(l, s)| (channel(l), *s)).collect();
        ImportColumn { size, tiles }
    }

    #[test]
    fn imported_tabs_go_last_with_their_layout() {
        let mut t = Tabs {
            tabs: vec![tab("a", "x")],
            active: Some("a".into()),
            ..Tabs::default()
        };
        let before = t.last_show;
        let show = TileShow {
            player: true,
            chat: true,
        };
        let tabs = vec![
            ImportTab {
                alias: Some("🎮 games".into()),
                columns: vec![col(1.0, &[("b", 1.0), ("c", 1.0)]), col(1.0, &[("d", 1.0)])],
            },
            ImportTab {
                alias: None,
                columns: vec![col(1.0, &[("e", 1.0)])],
            },
        ];
        t.import(tabs, show, Some("me".into()), &mut counter());

        assert_eq!(order(&t), ["a", "n1", "n5"]);
        assert_eq!((t.active.as_deref(), t.last_show), (Some("a"), before));
        let games = &t.tabs[1];
        assert_eq!(games.label(), "🎮 games");
        let cols: Vec<Vec<&str>> = games
            .columns
            .iter()
            .map(|c| c.tiles.iter().map(|x| x.channel.login.as_str()).collect())
            .collect();
        assert_eq!(cols, [vec!["b", "c"], vec!["d"]]);
        let muted: Vec<bool> = games.tiles().map(|x| x.muted).collect();
        assert_eq!(muted, [false, true, true]);
        assert!(
            games
                .tiles()
                .all(|x| x.account.as_deref() == Some("me") && x.show == show)
        );
        let e = &t.tabs[2];
        assert_eq!(e.label(), "e");
        assert!(
            e.tiles()
                .all(|x| x.account.as_deref() == Some("me") && x.show == show && !x.muted)
        );
    }

    #[test]
    fn nothing_to_import_changes_nothing() {
        let mut t = Tabs {
            tabs: vec![tab("a", "x")],
            ..Tabs::default()
        };
        let before = t.clone();
        t.import(Vec::new(), TileShow::default(), None, &mut counter());
        assert_eq!(t, before);
    }

    /// C1 Chatterino's shares come out averaging 1, like every other tab; a channel
    /// that has a tab already goes in again, twice in one tab too.
    #[test]
    fn imported_sizes_are_kept_and_channels_can_repeat() {
        let mut t = Tabs {
            tabs: vec![tab("a", "x")],
            ..Tabs::default()
        };
        let comfi = ImportTab {
            alias: Some("Comfi".into()),
            columns: vec![
                col(0.25, &[("x", 0.8188), ("y", 0.1812)]),
                col(0.75, &[("x", 0.25), ("z", 0.25), ("w", 0.5)]),
            ],
        };
        t.import(vec![comfi], TileShow::default(), None, &mut counter());
        let got = &t.tabs[1];
        let r = |v: f32| (v * 1000.0).round() / 1000.0;
        let cols: Vec<f32> = got.columns.iter().map(|c| r(c.size)).collect();
        assert_eq!(cols, [0.5, 1.5]);
        let rows: Vec<Vec<(&str, f32)>> = got
            .columns
            .iter()
            .map(|c| c.tiles.iter().map(|x| (x.channel.login.as_str(), r(x.size))).collect())
            .collect();
        assert_eq!(rows, [vec![("x", 1.638), ("y", 0.362)], vec![("x", 0.75), ("z", 0.75), ("w", 1.5)]]);
        assert_eq!(t.tabs[0].tiles().next().unwrap().channel.login, "x");
        let ids: std::collections::HashSet<&str> = t.tabs.iter().flat_map(|t| t.tiles()).map(|x| x.id.as_str()).collect();
        assert_eq!(ids.len(), 6);
    }

    fn imported(columns: Vec<ImportColumn>) -> Tab {
        let mut t = Tabs::default();
        let one = ImportTab { alias: None, columns };
        t.import(vec![one], TileShow::default(), None, &mut counter());
        t.tabs.remove(0)
    }

    /// B3 Root horizontal with flexh 1 and 30: 1:30, not 1:19 from clamping first.
    #[test]
    fn b3_small_column_share_keeps_its_ratio() {
        let got = imported(vec![col(1.0 / 31.0, &[("a", 1.0)]), col(30.0 / 31.0, &[("b", 1.0)])]);
        let (a, b) = (got.columns[0].size, got.columns[1].size);
        assert!((b / a - 30.0).abs() < 1e-3, "{a} {b}");
        assert!((a + b - 2.0).abs() < 1e-4);
    }

    /// B3 A column holding [a, horizontal[12 splits]]: each of the 12 gets 1/24.
    #[test]
    fn b3_twelve_small_tiles_keep_their_ratio() {
        let mut tiles = vec![("a".to_string(), 0.5)];
        tiles.extend((0..12).map(|i| (format!("s{i}"), 0.5 / 12.0)));
        let tiles: Vec<(&str, f32)> = tiles.iter().map(|(l, s)| (l.as_str(), *s)).collect();
        let got = imported(vec![col(1.0, &tiles)]);
        let sizes: Vec<f32> = got.columns[0].tiles.iter().map(|x| x.size).collect();
        assert_eq!(sizes.len(), 13);
        for s in &sizes[1..] {
            assert!((sizes[0] / s - 12.0).abs() < 1e-3, "{sizes:?}");
        }
        assert!((sizes.iter().sum::<f32>() - 13.0).abs() < 1e-3);
        let ids: std::collections::HashSet<&str> = got.tiles().map(|x| x.id.as_str()).chain([got.id.as_str()]).collect();
        assert_eq!(ids.len(), 14);
    }
}
