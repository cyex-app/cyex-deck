//! 1.15 Tabs follow a channel through name changes. Twitch is asked by user id; a new
//! login renames the tile and its chat rejoins under it. Aliases stay as they are.

use super::Tabs;

/// A channel as Twitch knows it now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub id: String,
    pub login: String,
    pub name: String,
}

impl Tabs {
    /// Takes the current login and name for every tile of these channels. Returns true
    /// if anything changed.
    pub fn apply_resolved(&mut self, users: &[Resolved]) -> bool {
        let mut changed = false;
        for tab in &mut self.tabs {
            for tile in tab.columns.iter_mut().flat_map(|c| c.tiles.iter_mut()) {
                let Some(user) = users.iter().find(|u| u.id == tile.channel.id) else {
                    continue;
                };
                if user.login.is_empty() {
                    continue;
                }
                if tile.channel.login != user.login || tile.channel.name != user.name {
                    tile.channel.login = user.login.clone();
                    tile.channel.name = user.name.clone();
                    changed = true;
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::tab;
    use super::*;

    fn renamed(id: &str, login: &str, name: &str) -> Resolved {
        Resolved {
            id: id.into(),
            login: login.into(),
            name: name.into(),
        }
    }

    #[test]
    fn renamed_channels_keep_their_tab() {
        let mut tabs = Tabs {
            tabs: vec![tab("a", "oldname"), tab("b", "other")],
            ..Tabs::default()
        };
        tabs.tabs[1].alias = Some("my alias".into());
        tabs.tabs[1].columns[0].tiles[0].channel.id = "uoldname".into();

        assert!(tabs.apply_resolved(&[renamed("uoldname", "newname", "NewName")]));
        for t in &tabs.tabs {
            let c = &t.columns[0].tiles[0].channel;
            assert_eq!((c.id.as_str(), c.login.as_str(), c.name.as_str()), ("uoldname", "newname", "NewName"));
        }
        assert_eq!(tabs.tabs[0].label(), "NewName");
        assert_eq!(tabs.tabs[1].label(), "my alias");
        assert_eq!(tabs.tabs[0].id, "a");
    }

    #[test]
    fn unchanged_or_unknown_channels_change_nothing() {
        let mut tabs = Tabs {
            tabs: vec![tab("a", "xqc")],
            ..Tabs::default()
        };
        assert!(!tabs.apply_resolved(&[renamed("uxqc", "xqc", "XQC")]));
        assert!(!tabs.apply_resolved(&[renamed("someone", "else", "Else")]));
        assert!(!tabs.apply_resolved(&[renamed("uxqc", "", "")]));
        // A new display name alone (capitalization) still updates.
        assert!(tabs.apply_resolved(&[renamed("uxqc", "xqc", "xQc")]));
    }
}
