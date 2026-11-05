//! V108-N06 Extra badge sets from community services: FFZ:AP, BadgeHQ, Chatsen, DankChat and
//! Homies. Each is one public list fetched at start and again slowly; who wears what is
//! kept by Twitch user id in [`crate::Cosmetics`]. Nothing is asked per message.
//!
//! Their badges are `BadgeDef`s with `provider: Twitch` ("none of the emote providers") and an id
//! `<set>:<name>`, so [`BadgeSet::of_id`] tells which switch governs them.

pub mod art;
pub mod badgehq;
pub mod chatsen;
pub mod cyex;
pub mod dankchat;
pub mod ffzap;
pub mod homies;

use std::collections::HashMap;
use std::time::Duration;

use cyex_chat::model::Provider;
use cyex_twitch::{Http, Result};

use crate::cosmetics::BadgeDef;

/// How long a loaded list is trusted before it is fetched again.
pub const REFRESH: Duration = Duration::from_secs(6 * 3600);
/// Most badges one person wears from one set.
const PER_USER: usize = 4;
/// Most people one set keeps (the biggest list has about 10.000).
const MAX_USERS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BadgeSet {
    FfzAp,
    BadgeHq,
    Chatsen,
    DankChat,
    Homies,
    /// V109-B02 cyex's own badges. The app loads them itself (with an ETag, its own switch);
    /// they draw before 7TV's, right after Twitch's.
    Cyex,
}

impl BadgeSet {
    pub const ALL: [BadgeSet; 6] = [
        BadgeSet::FfzAp,
        BadgeSet::BadgeHq,
        BadgeSet::Chatsen,
        BadgeSet::DankChat,
        BadgeSet::Homies,
        BadgeSet::Cyex,
    ];

    /// The start of every badge id of this set.
    pub fn prefix(self) -> &'static str {
        match self {
            BadgeSet::FfzAp => "ffzap",
            BadgeSet::BadgeHq => "badgehq",
            BadgeSet::Chatsen => "chatsen",
            BadgeSet::DankChat => "dankchat",
            BadgeSet::Homies => "homies",
            BadgeSet::Cyex => "cyex",
        }
    }

    /// The set a badge id (`ffzap:supporter`) belongs to.
    pub fn of_id(id: &str) -> Option<BadgeSet> {
        let (prefix, _) = id.split_once(':')?;
        BadgeSet::ALL.into_iter().find(|s| s.prefix() == prefix)
    }
}

/// One set's badges and who wears them.
#[derive(Debug, Default, PartialEq)]
pub struct Loaded {
    pub defs: Vec<BadgeDef>,
    /// Twitch user id → badge ids (prefixed), most important first.
    pub users: HashMap<String, Vec<String>>,
}

impl Loaded {
    pub(crate) fn give(&mut self, user: &str, badge_id: &str) {
        if !user_id(user) {
            return;
        }
        if !self.users.contains_key(user) && self.users.len() >= MAX_USERS {
            return;
        }
        let list = self.users.entry(user.to_owned()).or_default();
        if list.len() < PER_USER && !list.iter().any(|b| b == badge_id) {
            list.push(badge_id.to_owned());
        }
    }
}

/// Fetches one set's list.
pub async fn fetch(http: &Http, set: BadgeSet) -> Result<Loaded> {
    match set {
        BadgeSet::FfzAp => http.get_json::<ffzap::Raw>(ffzap::URL).await.map(Into::into),
        BadgeSet::BadgeHq => http.get_json::<badgehq::Raw>(badgehq::URL).await.map(Into::into),
        BadgeSet::Chatsen => http.get_json::<chatsen::Raw>(chatsen::URL).await.map(Into::into),
        BadgeSet::DankChat => http.get_json::<dankchat::Raw>(dankchat::URL).await.map(Into::into),
        BadgeSet::Homies => homies::fetch(http).await,
        BadgeSet::Cyex => {
            let kinds = cyex::fetch_kinds(http, cyex::BASE).await.ok();
            let snap = cyex::fetch_users(http, cyex::BASE, &mut cyex::Tagged::default()).await?;
            Ok(cyex::build(kinds.as_ref(), cyex::BASE, &snap.unwrap_or_default().users))
        }
    }
}

/// A badge of `set`; `sizes` are the 1x, 2x and 4x pictures (a single picture repeats).
pub(crate) fn def(set: BadgeSet, name: &str, title: &str, sizes: [&str; 3]) -> BadgeDef {
    BadgeDef {
        provider: Provider::Twitch,
        id: format!("{}:{name}", set.prefix()),
        title: title.to_owned(),
        image1x: sizes[0].to_owned(),
        image2x: sizes[1].to_owned(),
        image4x: sizes[2].to_owned(),
        still: None,
        color: None,
        replaces: None,
    }
}

/// Pictures come over https only and must be safe to put in an attribute.
pub(crate) fn picture(url: &str) -> bool {
    url.len() <= 2048
        && url.starts_with("https://")
        && !url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "\"'<>\\`".contains(c))
}

/// Twitch user ids are plain numbers.
pub(crate) fn user_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit())
}

/// A name as an id part: lowercase letters, digits and dashes.
pub(crate) fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.truncate(48);
    out.trim_end_matches('-').to_owned()
}

/// Titles come from other people's servers: trimmed and short.
pub(crate) fn title(text: &str) -> String {
    text.trim().chars().filter(|c| !c.is_control()).take(80).collect()
}

#[cfg(test)]
pub(crate) fn fixture<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let path = format!("{}/tests/fixtures/badges/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_tell_their_set() {
        for s in BadgeSet::ALL {
            assert_eq!(BadgeSet::of_id(&format!("{}:x", s.prefix())), Some(s));
        }
        assert_eq!(BadgeSet::of_id("staff"), None);
        assert_eq!(BadgeSet::of_id("other:x"), None);
    }

    #[test]
    fn users_and_pictures_are_checked() {
        assert!(user_id("4867723") && !user_id("") && !user_id("abc") && !user_id("1 2"));
        assert!(picture("https://a.b/c.png?v=3"));
        assert!(!picture("http://a.b/c.png") && !picture("https://a.b/\"x") && !picture("javascript:1"));
        assert_eq!(slug("DankChat Top  Supporter!"), "dankchat-top-supporter");
        assert_eq!(slug("!!"), "");
    }

    #[test]
    fn a_user_keeps_a_few_badges_once() {
        let mut l = Loaded::default();
        for b in ["a", "b", "a", "c", "d", "e"] {
            l.give("1", b);
        }
        l.give("x", "a");
        assert_eq!(l.users["1"], ["a", "b", "c", "d"]);
        assert!(!l.users.contains_key("x"));
    }
}
