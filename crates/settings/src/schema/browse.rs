use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Schema, TabChannel};

/// H2
const MAX_FAVORITES: usize = 300;
/// H4
const MAX_LANGUAGES: usize = 12;

/// Home and discover (1.6, 1.7).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Browse {
    /// 1.6
    pub layout: HomeLayout,
    /// H1
    pub live_sort: LiveSort,
    /// H1
    pub offline_sort: OfflineSort,
    /// H2 Starred channels, first on home. Kept by user id (1.15); login and name are
    /// what they were when starred.
    pub favorites: Vec<TabChannel>,
    /// H4 Discover's language filter, as Twitch's codes (`EN`, `DE`). Empty shows all.
    pub languages: Vec<String>,
}

impl Schema for Browse {
    fn sanitize(&mut self) {
        let mut seen = HashSet::new();
        self.favorites
            .retain(|c| !c.id.is_empty() && seen.insert(c.id.clone()));
        self.favorites.truncate(MAX_FAVORITES);
        for l in &mut self.languages {
            *l = l.trim().to_ascii_uppercase();
        }
        let mut seen = HashSet::new();
        self.languages.retain(|l| {
            !l.is_empty()
                && l.len() <= 8
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && seen.insert(l.clone())
        });
        self.languages.truncate(MAX_LANGUAGES);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HomeLayout {
    #[default]
    Grid,
    List,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LiveSort {
    #[default]
    Viewers,
    /// Live the longest first.
    Uptime,
    Name,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum OfflineSort {
    /// Live most recently first.
    #[default]
    LastLive,
    Name,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(id: &str) -> TabChannel {
        TabChannel {
            id: id.into(),
            login: id.into(),
            name: id.into(),
        }
    }

    #[test]
    fn favorites_and_languages_are_cleaned() {
        let mut b = Browse {
            favorites: vec![channel("1"), channel(""), channel("1"), channel("2")],
            languages: vec![" en ".into(), "EN".into(), "de".into(), "".into(), "x y".into()],
            ..Browse::default()
        };
        b.sanitize();
        let ids: Vec<&str> = b.favorites.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["1", "2"]);
        assert_eq!(b.languages, ["EN", "DE"]);
    }

    #[test]
    fn reads_plain_names() {
        let b: Browse = serde_json::from_str(r#"{"layout":"list","liveSort":"uptime"}"#).unwrap();
        assert_eq!(
            (b.layout, b.live_sort, b.offline_sort),
            (HomeLayout::List, LiveSort::Uptime, OfflineSort::LastLive)
        );
    }
}
