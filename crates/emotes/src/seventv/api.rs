//! 7TV over HTTP: emote sets through v4 GQL (a tenth of v3's size, research/providers.md),
//! presences (D21) and animated avatars (C23).

use std::collections::HashMap;

use cyex_chat::model::Provider;
use cyex_twitch::{Http, Result};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::model::{Emote, EmoteSet, Scope};

const GQL: &str = "https://api.7tv.app/v4/gql";
static BASE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
/// V106-31/32 Test runs with `CYEX_7TV_SETS=mock`: emote reads go to the local mock too.
pub fn use_mock(base: &str) {
    let _ = BASE.set(format!("{}/v4/gql", base.trim_end_matches('/')));
}
fn gql_url() -> &'static str {
    BASE.get().map_or(GQL, String::as_str)
}
const V3: &str = "https://7tv.io/v3";

const ITEM: &str = "items { id alias addedAt addedById flags { zeroWidth } \
    emote { defaultName aspectRatio ownerId flags { animated publicListed approvedPersonal private } } }";

/// Emote sets past this many aren't asked for in one go (Twitch's limit is 50).
pub const MAX_BATCH: usize = 50;

/// A channel's 7TV account.
#[derive(Debug, PartialEq)]
pub struct Channel {
    /// 7TV user id, for live updates of which set is active.
    pub user_id: String,
    pub set: Option<EmoteSet>,
}

/// `None` when the channel has no 7TV account.
pub async fn channel(http: &Http, twitch_id: &str) -> Result<Option<Channel>> {
    let q = format!(
        "query($id: String!) {{ users {{ userByConnection(platform: TWITCH, platformId: $id) {{ id \
         style {{ activeEmoteSet {{ id emotes {{ {ITEM} }} }} }} }} }} }}"
    );
    let data: Data<RawUsers> = post(http, &q, json!({ "id": twitch_id })).await?;
    Ok(data.data.users.user_by_connection.map(|u| Channel {
        user_id: u.id,
        set: u
            .style
            .and_then(|s| s.active_emote_set)
            .map(|s| s.into_set(Scope::Channel)),
    }))
}

pub async fn global(http: &Http) -> Result<EmoteSet> {
    let q = format!("query {{ emoteSets {{ global {{ id emotes {{ {ITEM} }} }} }} }}");
    let data: Data<RawGlobal> = post(http, &q, json!({})).await?;
    Ok(data.data.emote_sets.global.into_set(Scope::Global))
}

/// Sets by id, e.g. personal ones (C9). At most [`MAX_BATCH`].
pub async fn sets(http: &Http, ids: &[String]) -> Result<Vec<EmoteSet>> {
    let q = format!(
        "query($ids: [Id!]!) {{ emoteSets {{ emoteSets(ids: $ids) {{ id kind emotes {{ {ITEM} }} }} }} }}"
    );
    let ids = &ids[..ids.len().min(MAX_BATCH)];
    let data: Data<RawSets> = post(http, &q, json!({ "ids": ids })).await?;
    Ok(data
        .data
        .emote_sets
        .emote_sets
        .into_iter()
        .map(|s| {
            let scope = match s.kind.as_deref() {
                Some("PERSONAL" | "SPECIAL") => Scope::Personal,
                Some("GLOBAL") => Scope::Global,
                _ => Scope::Channel,
            };
            s.into_set(scope)
        })
        .collect())
}

/// The account's 7TV user id, if it has one (D21: presences go out under it).
pub async fn user_id(http: &Http, twitch_id: &str) -> Result<Option<String>> {
    let q = "query($id: String!) { users { userByConnection(platform: TWITCH, platformId: $id) { id } } }";
    let data: Data<RawUsers> = post(http, q, json!({ "id": twitch_id })).await?;
    Ok(data.data.users.user_by_connection.map(|u| u.id))
}

/// Display names of 7TV users by id, for "added by" on emote cards.
pub async fn user_names(http: &Http, ids: &[String]) -> Result<HashMap<String, String>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let fields: String = ids
        .iter()
        .take(20)
        .enumerate()
        .filter(|(_, id)| id.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(|(i, id)| format!("u{i}: user(id: \"{id}\") {{ id mainConnection {{ platformDisplayName }} }} "))
        .collect();
    let q = format!("query {{ users {{ {fields}}} }}");
    let data: Data<HashMap<String, HashMap<String, Option<RawNamed>>>> = post(http, &q, json!({})).await?;
    Ok(data
        .data
        .into_values()
        .flat_map(HashMap::into_values)
        .flatten()
        .filter_map(|u| Some((u.id, u.main_connection?.platform_display_name)))
        .collect())
}

/// Who made an emote, for its card.
pub async fn owner_name(http: &Http, emote_id: &str) -> Result<Option<String>> {
    let q =
        "query($id: Id!) { emotes { emote(id: $id) { owner { mainConnection { platformDisplayName } } } } }";
    let v: Value = post::<Value>(http, q, json!({ "id": emote_id })).await?;
    Ok(
        v.pointer("/data/emotes/emote/owner/mainConnection/platformDisplayName")
            .and_then(Value::as_str)
            .map(str::to_owned),
    )
}

/// D21 V106-08 The presence body, as the 7TV extension sends it (`worker.http.ts:236-245`):
/// `passive` only on joining, never a session id (7TV ignores both, `users.rs:471`).
pub fn presence_body(channel_id: &str, passive: bool) -> Value {
    let mut body = json!({ "kind": 1, "data": { "platform": "TWITCH", "id": channel_id } });
    if passive {
        body["passive"] = json!(true);
    }
    body
}

/// D21 Tells 7TV that this user is in a channel, so others see their cosmetics. No auth.
pub async fn post_presence(http: &Http, seventv_user: &str, channel_id: &str, passive: bool) -> Result<()> {
    let body = presence_body(channel_id, passive);
    let _: Value = http
        .post_json(&format!("{V3}/users/{seventv_user}/presences"), &body)
        .await?;
    Ok(())
}

/// C23 Custom 7TV profile pictures by Twitch login (lowercase). Users without one are
/// left out.
pub async fn avatars(http: &Http, logins: &[String]) -> Result<HashMap<String, Avatar>> {
    let ids: Vec<String> = logins.iter().map(|l| format!("username:{l}")).collect();
    let items: Vec<RawBridgeItem> = http
        .post_json(&format!("{V3}/bridge/event-api"), &json!({ "identifiers": ids }))
        .await?;
    Ok(items.into_iter().filter_map(RawBridgeItem::avatar).collect())
}

/// Picture URLs at 2x scale (fits a 56 px card).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Avatar {
    pub url: String,
    /// Still frame for E2; the same URL when it doesn't move.
    pub still: String,
}

async fn post<T: serde::de::DeserializeOwned>(http: &Http, query: &str, variables: Value) -> Result<T> {
    http.post_json(gql_url(), &json!({ "query": query, "variables": variables }))
        .await
}

#[derive(Deserialize)]
struct Data<T> {
    data: T,
}

#[derive(Deserialize)]
struct RawUsers {
    users: RawUserByConnection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUserByConnection {
    user_by_connection: Option<RawUser>,
}

#[derive(Deserialize)]
struct RawUser {
    id: String,
    style: Option<RawStyle>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStyle {
    active_emote_set: Option<RawSet>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGlobal {
    emote_sets: RawGlobalSets,
}

#[derive(Deserialize)]
struct RawGlobalSets {
    global: RawSet,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSets {
    emote_sets: RawSetList,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSetList {
    emote_sets: Vec<RawSet>,
}

#[derive(Deserialize)]
pub(crate) struct RawSet {
    id: String,
    kind: Option<String>,
    emotes: RawItems,
}

#[derive(Deserialize)]
struct RawItems {
    #[serde(default)]
    items: Vec<RawItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawItem {
    id: String,
    alias: String,
    added_at: Option<String>,
    added_by_id: Option<String>,
    flags: RawItemFlags,
    emote: Option<RawEmote>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawItemFlags {
    #[serde(default)]
    zero_width: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEmote {
    default_name: String,
    aspect_ratio: Option<f64>,
    #[serde(default)]
    owner_id: Option<String>,
    flags: RawEmoteFlags,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEmoteFlags {
    #[serde(default)]
    animated: bool,
    #[serde(default = "yes")]
    public_listed: bool,
    #[serde(default = "yes")]
    approved_personal: bool,
    #[serde(default)]
    private: bool,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawNamed {
    id: String,
    main_connection: Option<RawConnection>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawConnection {
    platform_display_name: String,
}

impl RawSet {
    fn into_set(self, scope: Scope) -> EmoteSet {
        let personal = scope == Scope::Personal;
        let emotes = self
            .emotes
            .items
            .into_iter()
            .filter_map(|i| {
                let e = i.emote?;
                // C9 Personal sets only lend emotes 7TV approved for that.
                if personal && !e.flags.approved_personal {
                    return None;
                }
                let mut emote = Emote::new(Provider::Seventv, i.id, i.alias);
                emote.original = (e.default_name != emote.name).then_some(e.default_name);
                emote.zero_width = i.flags.zero_width;
                emote.unlisted = !e.flags.public_listed;
                emote.private = e.flags.private;
                emote.owner_id = e.owner_id;
                emote.animated = e.flags.animated;
                emote.size = e.aspect_ratio.map(box_1x);
                emote.added_at = i.added_at;
                emote.added_by = i.added_by_id;
                Some(emote)
            })
            .collect();
        EmoteSet::new(self.id, Provider::Seventv, scope, emotes)
    }
}

/// 7TV's 1x pictures are 32 px tall and at most 96 wide.
pub(crate) fn box_1x(aspect: f64) -> (u16, u16) {
    if !(aspect.is_finite() && aspect > 0.0) {
        return (32, 32);
    }
    let w = (32.0 * aspect).round();
    if w <= 96.0 {
        (w.max(1.0) as u16, 32)
    } else {
        (96, (96.0 / aspect).round().max(1.0) as u16)
    }
}

#[derive(Deserialize)]
struct RawBridgeItem {
    body: Option<RawBridgeBody>,
}

#[derive(Deserialize)]
struct RawBridgeBody {
    object: Option<RawBridgeObject>,
}

#[derive(Deserialize)]
struct RawBridgeObject {
    kind: String,
    data: Option<RawAvatarData>,
}

#[derive(Deserialize)]
struct RawAvatarData {
    user: Option<RawAvatarUser>,
    host: Option<super::events::RawHost>,
}

#[derive(Deserialize)]
struct RawAvatarUser {
    username: Option<String>,
}

impl RawBridgeItem {
    fn avatar(self) -> Option<(String, Avatar)> {
        let obj = self.body?.object?;
        if obj.kind != "AVATAR" {
            return None;
        }
        let data = obj.data?;
        let login = data.user?.username?.to_lowercase();
        let host = data.host?;
        let (url, still) = host.pick("2x")?;
        Some((login, Avatar { url, still }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_body_is_kind_and_data() {
        assert_eq!(
            presence_body("123", false),
            json!({ "kind": 1, "data": { "platform": "TWITCH", "id": "123" } })
        );
    }

    #[test]
    fn joining_adds_only_passive() {
        assert_eq!(
            presence_body("123", true),
            json!({ "kind": 1, "passive": true, "data": { "platform": "TWITCH", "id": "123" } })
        );
    }

    #[test]
    fn presence_never_has_a_session_id() {
        for passive in [false, true] {
            assert!(presence_body("1", passive).get("session_id").is_none());
        }
    }

    fn fixture<T: serde::de::DeserializeOwned>(name: &str) -> T {
        let path = format!("{}/tests/fixtures/7tv/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn channel_set_maps_flags() {
        let d: Data<RawUsers> = fixture("v4_channel_emote_set.json");
        let u = d.data.users.user_by_connection.unwrap();
        let set = u
            .style
            .unwrap()
            .active_emote_set
            .unwrap()
            .into_set(Scope::Channel);
        let get = |n: &str| set.get(n).unwrap().clone();
        assert!(get("notL").zero_width);
        assert!(get("AlienRave").unlisted);
        assert!(get("GAMBA").animated);
        assert_eq!(get("GAMBA").size, Some((39, 32)));
        assert_eq!(get("reeferSad").original.as_deref(), Some("TriSad"));
        assert_eq!(
            get("notL").added_by.as_deref(),
            Some("01F7MSXWXR0001THV7TZX9W4ZK")
        );
    }

    #[test]
    fn private_and_owner_are_read() {
        let raw: RawSet = serde_json::from_str(
            r#"{"id":"S","emotes":{"items":[{"id":"E1","alias":"a","flags":{},
            "emote":{"defaultName":"a","ownerId":"U1","flags":{"private":true}}}]}}"#,
        )
        .unwrap();
        let set = raw.into_set(Scope::Channel);
        let e = set.get("a").unwrap();
        assert!(e.private);
        assert_eq!(e.owner_id.as_deref(), Some("U1"));
    }

    #[test]
    fn no_account_or_no_set() {
        let d: Data<RawUsers> = fixture("v4_channel_emote_set_no_account.json");
        assert!(d.data.users.user_by_connection.is_none());
        let d: Data<RawUsers> = fixture("v4_channel_emote_set_no_set.json");
        assert!(
            d.data
                .users
                .user_by_connection
                .unwrap()
                .style
                .unwrap()
                .active_emote_set
                .is_none()
        );
    }

    #[test]
    fn batches_know_personal_sets() {
        let d: Data<RawSets> = fixture("v4_emote_sets_batch.json");
        let first = d.data.emote_sets.emote_sets.into_iter().next().unwrap();
        assert_eq!(first.kind.as_deref(), Some("PERSONAL"));
        assert!(!first.into_set(Scope::Personal).emotes.is_empty());
    }

    #[test]
    fn global_set_parses() {
        let d: Data<RawGlobal> = fixture("v4_global_emote_set.json");
        let set = d.data.emote_sets.global.into_set(Scope::Global);
        assert_eq!(set.emotes.iter().filter(|e| e.zero_width).count(), 2);
    }

    #[test]
    fn boxes_are_capped() {
        assert_eq!(box_1x(1.0), (32, 32));
        assert_eq!(box_1x(4.0), (96, 24));
        assert_eq!(box_1x(f64::NAN), (32, 32));
    }

    #[test]
    fn bridge_avatars() {
        let item: RawBridgeItem = fixture("bridge_avatar_response_item.json");
        let (login, a) = item.avatar().unwrap();
        assert!(!login.is_empty());
        assert!(a.url.starts_with("https://cdn.7tv.app/"));
    }
}
