//! Emotes and sets as every provider maps them.

use std::sync::Arc;

use cyex_chat::model::{EmoteLook, Provider};
use cyex_chat::tokenize::Found;
use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Emote {
    pub provider: Provider,
    /// The provider's id, also the picture's.
    pub id: String,
    /// What's typed in chat.
    pub name: String,
    /// 7TV: the emote's own name when the set renamed it.
    pub original: Option<String>,
    /// C7
    pub zero_width: bool,
    /// C7 FFZ modifier emote: these flags go to the emote before it.
    pub modifier: Option<u32>,
    /// 2.4
    pub unlisted: bool,
    pub animated: bool,
    /// At 1x, when the provider says.
    pub size: Option<(u16, u16)>,
    /// Who made it: 7TV owner, BTTV or FFZ uploader. Display name.
    pub owner: Option<String>,
    /// When it was added to the set (ISO), 7TV.
    pub added_at: Option<String>,
    /// Who added it, 7TV user id (names are looked up when a card opens).
    #[serde(skip)]
    pub added_by: Option<String>,
}

impl Emote {
    pub fn new(provider: Provider, id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            provider,
            id: id.into(),
            name: name.into(),
            original: None,
            zero_width: false,
            modifier: None,
            unlisted: false,
            animated: false,
            size: None,
            owner: None,
            added_at: None,
            added_by: None,
        }
    }

    pub fn found(&self) -> Found {
        Found {
            provider: self.provider,
            id: self.id.clone(),
            zero_width: self.zero_width,
            modifier: self.modifier,
            unlisted: self.unlisted,
            look: EmoteLook {
                animated: self.animated.then_some(true),
                size: self.size,
            },
        }
    }
}

/// What a set is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Scope {
    Global,
    Channel,
    /// C9 Only its owner's messages.
    Personal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmoteSet {
    pub id: String,
    pub provider: Provider,
    pub scope: Scope,
    pub emotes: Vec<Arc<Emote>>,
}

impl EmoteSet {
    pub fn new(id: impl Into<String>, provider: Provider, scope: Scope, emotes: Vec<Emote>) -> Self {
        Self {
            id: id.into(),
            provider,
            scope,
            emotes: emotes.into_iter().map(Arc::new).collect(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Arc<Emote>> {
        self.emotes.iter().find(|e| e.name == name)
    }
}
