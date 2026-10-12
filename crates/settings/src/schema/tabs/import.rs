//! V1041-02 Tabs brought in from another app (a Chatterino layout), after the others.

use super::{Column, Schema, Tab, TabChannel, Tabs, Tile, TileShow};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportTab {
    /// 1.22
    pub alias: Option<String>,
    /// U3 Left to right, each column top to bottom.
    pub columns: Vec<Vec<TabChannel>>,
}

impl Tabs {
    /// Adds the tabs after the others, joined as `account`. The shown tab and the
    /// add-tab defaults stay as they are. Like H3, only a tab's first channel plays sound.
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
            let mut muted = false;
            for channels in imported.columns {
                self.tabs[at].columns.push(Column::default());
                for channel in channels {
                    // Each id is taken before the next is drawn, so none repeat.
                    let tile = Tile {
                        id: self.fresh(new_id),
                        channel,
                        show,
                        account: account.clone(),
                        ratio: self.last_ratio,
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
                columns: vec![vec![channel("b"), channel("c")], vec![channel("d")]],
            },
            ImportTab {
                alias: None,
                columns: vec![vec![channel("e")]],
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
        assert_eq!(t.tabs[2].label(), "e");
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
}
