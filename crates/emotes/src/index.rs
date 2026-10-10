//! What chat looks words up in (C11): channel sets before global ones, providers in the
//! user's order. Personal emotes (C9) only for their owner, and the account's own Twitch
//! emotes for its own lines, which Twitch doesn't tag.

use std::collections::HashMap;
use std::sync::Arc;

use cyex_chat::model::Provider;
use cyex_chat::tokenize::{Emotes, Found};

use crate::model::{Emote, EmoteSet, Scope};

/// Name → emote. The first set given wins a name.
#[derive(Debug, Default)]
pub struct Lookup {
    map: HashMap<Box<str>, Arc<Emote>>,
}

impl Lookup {
    pub fn build<'a>(sets: impl IntoIterator<Item = &'a EmoteSet>) -> Self {
        let mut map: HashMap<Box<str>, Arc<Emote>> = HashMap::new();
        for set in sets {
            for e in &set.emotes {
                map.entry(e.name.as_str().into()).or_insert_with(|| e.clone());
            }
        }
        Self { map }
    }

    pub fn get(&self, name: &str) -> Option<&Arc<Emote>> {
        self.map.get(name)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// C11 Channel sets first, then global ones, each by the provider order (Twitch first).
pub fn ordered<'a>(sets: &'a [Arc<EmoteSet>], priority: &[Provider]) -> Vec<&'a EmoteSet> {
    let rank = |p: Provider| priority.iter().position(|x| *x == p).unwrap_or(priority.len());
    let scope = |s: Scope| match s {
        Scope::Personal => 0,
        Scope::Channel => 1,
        Scope::Global => 2,
    };
    let mut out: Vec<&EmoteSet> = sets.iter().map(Arc::as_ref).collect();
    out.sort_by_key(|s| (scope(s.scope), rank(s.provider)));
    out
}

/// Everything one room's lines are tokenized with.
#[derive(Clone, Default)]
pub struct RoomEmotes {
    pub lookup: Arc<Lookup>,
    /// C9 By Twitch user id.
    pub personal: Arc<HashMap<String, Arc<Lookup>>>,
    /// The reading account's id and its Twitch emotes.
    pub mine: Option<(String, Arc<Lookup>)>,
}

impl Emotes for RoomEmotes {
    fn find(&self, word: &str, sender: &str) -> Option<Found> {
        if !sender.is_empty() {
            if let Some((me, twitch)) = &self.mine
                && me == sender
                && let Some(e) = twitch.get(word)
            {
                return Some(e.found());
            }
            if let Some(e) = self.personal.get(sender).and_then(|l| l.get(word)) {
                return Some(e.found());
            }
        }
        self.lookup.get(word).map(|e| e.found())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(provider: Provider, scope: Scope, names: &[&str]) -> Arc<EmoteSet> {
        let emotes = names
            .iter()
            .map(|n| Emote::new(provider, format!("{provider:?}-{n}"), *n))
            .collect();
        Arc::new(EmoteSet::new("s", provider, scope, emotes))
    }

    #[test]
    fn channel_beats_global_then_priority() {
        let sets = vec![
            set(Provider::Seventv, Scope::Global, &["a", "b", "c"]),
            set(Provider::Ffz, Scope::Channel, &["a"]),
            set(Provider::Bttv, Scope::Channel, &["a", "b"]),
        ];
        let order = ordered(&sets, &[Provider::Seventv, Provider::Bttv, Provider::Ffz]);
        let l = Lookup::build(order);
        assert_eq!(l.get("a").unwrap().provider, Provider::Bttv);
        assert_eq!(l.get("b").unwrap().provider, Provider::Bttv);
        assert_eq!(l.get("c").unwrap().provider, Provider::Seventv);
        let l = Lookup::build(ordered(
            &sets,
            &[Provider::Ffz, Provider::Bttv, Provider::Seventv],
        ));
        assert_eq!(l.get("a").unwrap().provider, Provider::Ffz);
    }

    #[test]
    fn personal_and_own_emotes_only_for_their_sender() {
        let room = RoomEmotes {
            lookup: Arc::new(Lookup::build([
                set(Provider::Seventv, Scope::Channel, &["x"]).as_ref()
            ])),
            personal: Arc::new(HashMap::from([(
                "7".to_owned(),
                Arc::new(Lookup::build([set(
                    Provider::Seventv,
                    Scope::Personal,
                    &["p", "x"],
                )
                .as_ref()])),
            )])),
            mine: Some((
                "1".into(),
                Arc::new(Lookup::build([
                    set(Provider::Twitch, Scope::Global, &["Kappa"]).as_ref()
                ])),
            )),
        };
        assert!(room.find("p", "7").is_some());
        assert!(room.find("p", "8").is_none());
        assert!(room.find("Kappa", "1").is_some());
        assert!(room.find("Kappa", "2").is_none());
        assert_eq!(room.find("x", "7").unwrap().id, "Seventv-x");
    }
}
