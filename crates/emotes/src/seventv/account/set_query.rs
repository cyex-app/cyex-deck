//! V106-31/32 The set-editing documents (research-phase-5 §1, §3).

/// What an account can edit. Public: no token needed.
pub const EDITOR_FOR: &str = r#"query EditorFor($tw: String!) { users { userByConnection(platform: TWITCH, platformId: $tw) {
  id mainConnection { platformDisplayName } style { activeEmoteSetId }
  editorFor { state permissions { emoteSet { manage } }
    user { id mainConnection { platformDisplayName } style { activeEmoteSetId } } } } } }"#;

/// Target sets, ids and aliases only. 1-50 ids.
pub const EDIT_SETS: &str = r#"query EditSets($ids: [Id!]!) { emoteSets { emoteSets(ids: $ids) {
  id kind capacity ownerId emotes { items { id alias emote { id } } } } } }"#;

/// I106-3 One emote by id, for `/add <id>` when no loaded chat has it. Public.
pub const EMOTE_BRIEF: &str =
    "query EmoteBrief($id: Id!) { emotes { emote(id: $id) { id defaultName ownerId flags { private } } } }";

pub const ADD: &str = "mutation AddEmoteToSet($setId: Id!, $emote: EmoteSetEmoteId!) { emoteSets { emoteSet(id: $setId) { addEmote(id: $emote) { id } } } }";
pub const REMOVE: &str = "mutation RemoveEmoteFromSet($setId: Id!, $emote: EmoteSetEmoteId!) { emoteSets { emoteSet(id: $setId) { removeEmote(id: $emote) { id } } } }";
pub const RENAME: &str = "mutation RenameEmoteInSet($setId: Id!, $emote: EmoteSetEmoteId!, $alias: String!) { emoteSets { emoteSet(id: $setId) { updateEmoteAlias(id: $emote, alias: $alias) { id } } } }";
