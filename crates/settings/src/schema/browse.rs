use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Schema, TabChannel};

/// H2
const MAX_FAVORITES: usize = 300;
/// H4
const MAX_LANGUAGES: usize = 12;
/// V108-N05
const MAX_HIDDEN: usize = 300;
/// V108-N05
const MAX_HIDDEN_TAGS: usize = 100;

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
    /// V108-N05 Categories left out of Home and Discover, by Twitch's id; the name is what
    /// it was when hidden (the manage list shows it).
    pub hidden_categories: Vec<HiddenCategory>,
    /// V108-N05 Streams carrying one of these tags are left out (Twitch's content labels
    /// such as mature ones aren't on the lists, so tags are what can be matched). Any case.
    pub hidden_tags: Vec<String>,
}

/// V108-N05
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct HiddenCategory {
    pub id: String,
    pub name: String,
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
        let mut seen = HashSet::new();
        self.hidden_categories.retain(|c| !c.id.is_empty() && seen.insert(c.id.clone()));
        self.hidden_categories.truncate(MAX_HIDDEN);
        for t in &mut self.hidden_tags {
            *t = t.trim().to_owned();
        }
        let mut seen = HashSet::new();
        self.hidden_tags
            .retain(|t| !t.is_empty() && t.chars().count() <= 40 && seen.insert(t.to_lowercase()));
        self.hidden_tags.truncate(MAX_HIDDEN_TAGS);
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
    fn hidden_lists_are_cleaned() {
        let hide = |id: &str| HiddenCategory { id: id.into(), name: id.into() };
        let mut b = Browse {
            hidden_categories: vec![hide("1"), hide(""), hide("1"), hide("2")],
            hidden_tags: vec![" NSFW ".into(), "nsfw".into(), "".into(), "x".repeat(41), "18+".into()],
            ..Browse::default()
        };
        b.sanitize();
        assert_eq!(b.hidden_categories.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["1", "2"]);
        assert_eq!(b.hidden_tags, ["NSFW", "18+"]);
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
