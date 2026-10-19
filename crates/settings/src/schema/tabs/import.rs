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
    /// Adds the tabs after the others, joined as `account`, with their sizes (shares are
    /// scaled by `sanitize`). The shown tab and the add-tab
    /// defaults stay as they are. Like H3, only a tab's first channel plays sound.
    pub fn import(
        &mut self,
        tabs: Vec<ImportTab>,
        show: TileShow,
        account: Option<String>,
        new_id: &mut dyn FnMut() -> String,
    ) {
        for imported in tabs {
            let id = self.fresh(new_id);
            self.tabs.push(Tab {
                id,
                alias: imported.alias,
                group: None,
                columns: Vec::new(),
            });
            let at = self.tabs.len() - 1;
            let joins = account.clone();
            let mut muted = false;
            for column in imported.columns {
                self.tabs[at].columns.push(Column {
                    size: column.size,
                    tiles: Vec::new(),
                });
                for (channel, size) in column.tiles {
                    // Each id is taken before the next is drawn, so none repeat.
                    let tile = Tile {
                        id: self.fresh(new_id),
                        channel,
                        show,
                        account: joins.clone(),
                        ratio: self.last_ratio,
                        size,
                        volume: self.last_volume,
                        muted,
                        ..Tile::default()
                    };
                    muted = true;
                    if let Some(column) = self.tabs[at].columns.last_mut() {
                        column.tiles.push(tile);
                    }
                }
            }
        }
        self.sanitize();
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
}
