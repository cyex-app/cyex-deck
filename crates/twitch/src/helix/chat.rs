//! 1.40 For accounts with only a Helix sign-in: the emotes they can use
//! (`user:read:emotes`) and their name color (`user:manage:chat_color`).

use std::collections::HashMap;

use serde::Deserialize;

use super::{Auth, get};
use crate::gql::{EmoteKind, EmoteOwner, EmoteSet, TwitchEmote};
use crate::http::{empty, json};
use crate::{Http, Result};

const MAX_PAGES: usize = 30;

#[derive(Deserialize)]
struct Page {
    data: Vec<RawEmote>,
    #[serde(default)]
    pagination: Pagination,
}

#[derive(Default, Deserialize)]
struct Pagination {
    cursor: Option<String>,
}

#[derive(Deserialize)]
struct RawEmote {
    id: String,
    name: String,
    emote_type: String,
    emote_set_id: String,
    #[serde(default)]
    owner_id: String,
    #[serde(default)]
    format: Vec<String>,
}

/// Every emote the user can use, grouped into sets. `broadcaster_id` adds that channel's
/// follower emotes. Owners carry only their id; names are for the caller to fill in.
pub async fn user_emotes(
    http: &Http,
    auth: Auth<'_>,
    user_id: &str,
    broadcaster_id: &str,
) -> Result<Vec<EmoteSet>> {
    let mut raw = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let mut req = get(http, auth, "/chat/emotes/user").query(&[("user_id", user_id)]);
        if !broadcaster_id.is_empty() {
            req = req.query(&[("broadcaster_id", broadcaster_id)]);
        }
        if let Some(c) = &after {
            req = req.query(&[("after", c)]);
        }
        let page: Page = json(req).await?;
        let done = page.data.is_empty();
        raw.extend(page.data);
        after = page.pagination.cursor.filter(|c| !c.is_empty());
        if done || after.is_none() {
            break;
        }
    }
    Ok(group(raw))
}

fn group(raw: Vec<RawEmote>) -> Vec<EmoteSet> {
    let mut order: Vec<String> = Vec::new();
    let mut sets: HashMap<String, EmoteSet> = HashMap::new();
    for e in raw {
        let set = sets.entry(e.emote_set_id.clone()).or_insert_with(|| {
            order.push(e.emote_set_id.clone());
            EmoteSet {
                id: e.emote_set_id.clone(),
                owner: (!matches!(e.owner_id.as_str(), "" | "0" | "twitch" | "139075904")).then(|| {
                    EmoteOwner {
                        id: e.owner_id.clone(),
                        login: String::new(),
                        name: String::new(),
                        avatar: None,
                    }
                }),
                emotes: Vec::new(),
            }
        });
        set.emotes.push(TwitchEmote {
            kind: EmoteKind::classify(&e.emote_set_id, &e.emote_type),
            animated: e.format.iter().any(|f| f == "animated"),
            id: e.id,
            name: e.name,
            set_id: e.emote_set_id,
            modifiers: Vec::new(),
        });
    }
    order.into_iter().filter_map(|id| sets.remove(&id)).collect()
}

/// A named color (`blue_violet`) or `#RRGGBB` (Prime or Turbo only).
pub async fn set_chat_color(http: &Http, auth: Auth<'_>, user_id: &str, color: &str) -> Result<()> {
    let req = http
        .0
        .put(format!("{}/chat/color", super::BASE))
        .header("Client-Id", auth.client_id)
        .bearer_auth(auth.token)
        .query(&[("user_id", user_id), ("color", color)]);
    empty(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emotes_group_by_set() {
        let p: Page = serde_json::from_str(
            r#"{"data":[
              {"id":"25","name":"Kappa","emote_type":"globals","emote_set_id":"0","owner_id":"0","format":["static"]},
              {"id":"1","name":"xqcL","emote_type":"subscriptions","emote_set_id":"19503","owner_id":"71092938","format":["static","animated"]},
              {"id":"2","name":"xqcA","emote_type":"subscriptions","emote_set_id":"19503","owner_id":"71092938","format":["static"]}
            ],"template":"x","pagination":{}}"#,
        )
        .unwrap();
        let sets = group(p.data);
        assert_eq!(sets.len(), 2);
        assert!(sets[0].owner.is_none());
        assert_eq!(sets[0].emotes[0].kind, EmoteKind::Global);
        assert_eq!(sets[1].owner.as_ref().unwrap().id, "71092938");
        assert!(sets[1].emotes[0].animated);
        assert_eq!(sets[1].emotes.len(), 2);
    }
}
