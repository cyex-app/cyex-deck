//! 1.26 The bell (Twitch's onsite notifications) and 2.2 Twitch's own notification
//! settings: email, push and onsite switches per category, and the live-notification
//! choice per followed channel. GQL with the account's token; shapes checked 2026/10/10,
//! mutations never run. Live bell updates come from Hermes `onsite-notifications.<id>`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, query};
use crate::{Error, Http, Result};

pub const PAGE: u32 = 20;

/// Twitch's two bell lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "BellList")]
pub enum List {
    /// What viewers get: live, uploads, gifts, drops.
    Viewer,
    /// What a channel owner gets.
    Creator,
}

impl List {
    fn gql(self) -> &'static str {
        match self {
            List::Viewer => "VIEWER",
            List::Creator => "CREATOR",
        }
    }
}

/// One bell item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "BellItem")]
pub struct Item {
    pub id: String,
    /// Twitch's kind, e.g. `streamup` or `video_upload`.
    pub kind: String,
    /// Markdown: `**name** went live`.
    pub body: String,
    /// RFC 3339.
    pub created_at: String,
    pub read: bool,
    pub thumbnail: Option<String>,
    /// Where a click goes.
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Page {
    pub items: Vec<Item>,
    pub cursor: Option<String>,
}

pub async fn list(http: &Http, auth: Auth<'_>, which: List, after: Option<&str>) -> Result<Page> {
    let q = format!(
        "query($after: Cursor) {{ currentUser {{ id notifications(first: {PAGE}, after: $after, displayType: {}) {{ \
         pageInfo {{ hasNextPage }} edges {{ cursor node {{ id type body createdAt isRead thumbnailURL \
         actions {{ label url }} }} }} }} }} }}",
        which.gql()
    );
    let data: Me<ListUser> = query(http, auth, &q, json!({ "after": after })).await?;
    let conn = data
        .current_user
        .ok_or_else(signed_out)?
        .notifications
        .unwrap_or_default();
    let more = conn.page_info.is_some_and(|p| p.has_next_page);
    let cursor = more
        .then(|| conn.edges.last().map(|e| e.cursor.clone()))
        .flatten();
    Ok(Page {
        items: conn
            .edges
            .into_iter()
            .filter_map(|e| e.node.map(RawItem::into_item))
            .collect(),
        cursor,
    })
}

/// Bell items not seen yet, for the badge.
pub async fn unseen(http: &Http, auth: Auth<'_>) -> Result<u32> {
    let q = "query { currentUser { id notifications { summary { unseenCount } } } }";
    let data: Me<SummaryUser> = query(http, auth, q, json!({})).await?;
    Ok(data
        .current_user
        .ok_or_else(signed_out)?
        .notifications
        .and_then(|n| n.summary)
        .map_or(0, |s| s.unseen_count))
}

/// The bell was opened: clears the badge.
pub async fn seen(http: &Http, auth: Auth<'_>) -> Result<()> {
    let q = "mutation { viewedNotifications { user { id } } }";
    query::<Value>(http, auth, q, json!({})).await.map(drop)
}

pub async fn read(http: &Http, auth: Auth<'_>, ids: &[String]) -> Result<()> {
    let q = "mutation($ids: [ID!]!) { readNotifications(input: {ids: $ids}) { count } }";
    query::<Value>(http, auth, q, json!({ "ids": ids }))
        .await
        .map(drop)
}

pub async fn read_all(http: &Http, auth: Auth<'_>, which: List, user_id: &str) -> Result<()> {
    let field = match which {
        List::Viewer => "markAllViewerNotificationsAsRead",
        List::Creator => "markAllCreatorNotificationsAsRead",
    };
    let q = format!("mutation($id: ID!) {{ {field}(input: {{userID: $id}}) {{ hasSucceeded }} }}");
    query::<Value>(http, auth, &q, json!({ "id": user_id }))
        .await
        .map(drop)
}

pub async fn delete(http: &Http, auth: Auth<'_>, id: &str) -> Result<()> {
    let q = "mutation($id: ID!) { deleteNotification(input: {id: $id}) { notification { id } } }";
    query::<Value>(http, auth, q, json!({ "id": id })).await.map(drop)
}

/// A Hermes `onsite-notifications.<id>` message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Live {
    /// `toast`: Twitch wants it to pop up. `list`: it also goes in the bell.
    Created {
        item: Item,
        toast: bool,
        list: bool,
        creator: bool,
    },
    Updated(Item),
    Deleted(String),
    Read(Vec<String>),
    AllRead,
    /// The badge count changed.
    Unseen(u32),
}

pub fn from_hermes(payload: &Value) -> Option<Live> {
    let kind = payload.get("type")?.as_str()?;
    let data = match payload.get("data")? {
        Value::String(s) => serde_json::from_str(s).ok()?,
        d => d.clone(),
    };
    let item = |n: &Value| -> Option<Item> {
        let actions = n["actions"].as_array();
        let url = actions.and_then(|list| {
            let pick = |label: &str| {
                list.iter()
                    .find(|a| a["label"] == label)
                    .and_then(|a| a["url"].as_str())
            };
            pick("CTA").or_else(|| list.iter().find_map(|a| a["url"].as_str()))
        });
        Some(Item {
            id: n["id"].as_str()?.to_owned(),
            kind: n["type"].as_str().unwrap_or_default().to_owned(),
            body: n["body_md"]
                .as_str()
                .or(n["body"].as_str())
                .unwrap_or_default()
                .to_owned(),
            created_at: n["created_at"].as_str().unwrap_or_default().to_owned(),
            read: n["read"].as_bool().unwrap_or(false),
            thumbnail: n["thumbnail_url"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            url: url.filter(|s| !s.is_empty()).map(str::to_owned),
        })
    };
    match kind {
        "create-notification" => Some(Live::Created {
            item: item(&data["notification"])?,
            toast: data["toast"].as_bool().unwrap_or(false),
            list: data["persistent"].as_bool().unwrap_or(true),
            creator: data["display_type"] == "CREATOR" || data["notification"]["display_type"] == "CREATOR",
        }),
        "update-notification" => Some(Live::Updated(item(&data["notification"])?)),
        "delete-notification" => Some(Live::Deleted(data["notification_id"].as_str()?.to_owned())),
        "read-notifications" => Some(Live::Read(
            data["notification_ids"]
                .as_array()?
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect(),
        )),
        "read-all-notifications" => Some(Live::AllRead),
        "update-summary" => {
            let s = &data["summary"];
            let n = s["unseen_view_count"].as_u64().or(s["unseen_count"].as_u64())?;
            Some(Live::Unseen(n as u32))
        }
        _ => None,
    }
}

/// 2.2 One of Twitch's notification switches: a category on one platform.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TwitchNotifySwitch")]
pub struct Switch {
    /// e.g. `LIVE`, `CHAT`, `PROMOTIONS`.
    pub category: String,
    /// `email`, `push` or `onsite`.
    pub platform: String,
    pub on: bool,
}

/// 2.2 A followed channel and Twitch's live-notification choice for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TwitchChannelNotify")]
pub struct ChannelNotify {
    pub id: String,
    pub login: String,
    pub name: String,
    pub avatar: Option<String>,
    /// `ALWAYS`, `LIVE_UP_ONLY`, `PERSONALIZED` or `NEVER`.
    pub state: String,
}

/// The four choices Twitch offers per channel.
pub const CHANNEL_STATES: [&str; 4] = ["ALWAYS", "LIVE_UP_ONLY", "PERSONALIZED", "NEVER"];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub switches: Vec<Switch>,
    pub channels: Vec<ChannelNotify>,
    pub cursor: Option<String>,
}

/// Twitch's switches and a page of followed channels with their live-notification choice.
pub async fn settings(http: &Http, auth: Auth<'_>, after: Option<&str>) -> Result<Settings> {
    let q = "query($after: Cursor) { currentUser { id \
        notificationSettings { category platforms { platformName settingState isEnabled } } \
        notificationEnrollmentFollows(first: 100, after: $after, filters: [], order: DESC) { \
        edges { cursor notificationSettings { followsSettingState } \
        node { id login displayName profileImageURL(width: 70) } } pageInfo { hasNextPage } } } }";
    let data: Me<SettingsUser> = query(http, auth, q, json!({ "after": after })).await?;
    let me = data.current_user.ok_or_else(signed_out)?;
    let switches = me
        .notification_settings
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|c| {
            let category = c.category;
            c.platforms.into_iter().flatten().map(move |p| Switch {
                category: category.clone(),
                on: p.setting_state.as_deref().map_or(p.is_enabled, |s| s == "on"),
                platform: p.platform_name,
            })
        })
        .collect();
    let conn = me.notification_enrollment_follows.unwrap_or_default();
    let more = conn.page_info.is_some_and(|p| p.has_next_page);
    let cursor = more
        .then(|| conn.edges.last().map(|e| e.cursor.clone()))
        .flatten();
    let channels = conn
        .edges
        .into_iter()
        .filter_map(|e| {
            let n = e.node?;
            Some(ChannelNotify {
                id: n.id,
                login: n.login,
                name: n.display_name,
                avatar: n.profile_image_url,
                state: e
                    .notification_settings
                    .and_then(|s| s.follows_setting_state)
                    .unwrap_or_else(|| "ALWAYS".into()),
            })
        })
        .collect();
    Ok(Settings {
        switches,
        channels,
        cursor,
    })
}

pub async fn set_switch(http: &Http, auth: Auth<'_>, category: &str, platform: &str, on: bool) -> Result<()> {
    let q = "mutation($c: String!, $p: String!, $s: String!) { \
        setNotificationSetting(input: {category: $c, platform: $p, settingState: $s}) { settingState } }";
    let vars = json!({ "c": category, "p": platform, "s": if on { "on" } else { "off" } });
    query::<Value>(http, auth, q, vars).await.map(drop)
}

/// `state` is one of [`CHANNEL_STATES`].
pub async fn set_channel(http: &Http, auth: Auth<'_>, channel_id: &str, state: &str) -> Result<()> {
    if !CHANNEL_STATES.contains(&state) {
        return Err(Error::Api {
            status: 400,
            message: format!("unknown notification choice {state}"),
        });
    }
    let q = "mutation($id: ID!, $s: FollowsChannelNotificationSettingState!) { \
        setLiveNotificationsEnrollment(input: {channelID: $id, settingState: $s}) { error } }";
    let data: Value = query(http, auth, q, json!({ "id": channel_id, "s": state })).await?;
    match data["setLiveNotificationsEnrollment"]["error"].as_str() {
        Some(code) => Err(Error::Api {
            status: 200,
            message: code.to_owned(),
        }),
        None => Ok(()),
    }
}

fn signed_out() -> Error {
    Error::Api {
        status: 200,
        message: "unauthenticated".into(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Me<T> {
    current_user: Option<T>,
}

#[derive(Deserialize)]
struct ListUser {
    notifications: Option<Connection<RawItem>>,
}

#[derive(Deserialize)]
struct SummaryUser {
    notifications: Option<SummaryHolder>,
}

#[derive(Deserialize)]
struct SummaryHolder {
    summary: Option<Summary>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    #[serde(default)]
    unseen_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsUser {
    notification_settings: Option<Vec<Option<Category>>>,
    notification_enrollment_follows: Option<Connection<RawChannel, FollowEdgeExtra>>,
}

#[derive(Deserialize)]
struct Category {
    category: String,
    #[serde(default)]
    platforms: Vec<Option<Platform>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Platform {
    platform_name: String,
    setting_state: Option<String>,
    #[serde(default)]
    is_enabled: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct FollowEdgeExtra {
    follows_setting_state: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannel {
    id: String,
    login: String,
    #[serde(default)]
    display_name: String,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Connection<T, X = ()> {
    #[serde(default = "Vec::new")]
    edges: Vec<Edge<T, X>>,
    page_info: Option<PageInfo>,
}

impl<T, X> Default for Connection<T, X> {
    fn default() -> Self {
        Self {
            edges: Vec::new(),
            page_info: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Edge<T, X> {
    #[serde(default)]
    cursor: String,
    node: Option<T>,
    notification_settings: Option<X>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    #[serde(default)]
    has_next_page: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawItem {
    id: String,
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    is_read: bool,
    #[serde(rename = "thumbnailURL")]
    thumbnail_url: Option<String>,
    #[serde(default)]
    actions: Vec<Option<Action>>,
}

#[derive(Deserialize)]
struct Action {
    label: Option<String>,
    url: Option<String>,
}

impl RawItem {
    fn into_item(self) -> Item {
        let actions: Vec<Action> = self.actions.into_iter().flatten().collect();
        let url = actions
            .iter()
            .find(|a| a.label.as_deref() == Some("CTA"))
            .or_else(|| actions.iter().find(|a| a.url.is_some()))
            .and_then(|a| a.url.clone())
            .filter(|u| !u.is_empty());
        Item {
            id: self.id,
            kind: self.kind,
            body: self.body,
            created_at: self.created_at,
            read: self.is_read,
            thumbnail: self.thumbnail_url.filter(|s| !s.is_empty()),
            url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_parses() {
        let d: Me<ListUser> = serde_json::from_str(
            r#"{"currentUser":{"id":"1","notifications":{"pageInfo":{"hasNextPage":false},"edges":[{"cursor":"c",
              "node":{"id":"n1","type":"streamup","body":"**xqc** went live","createdAt":"2026-10-10T10:00:00Z",
              "isRead":false,"thumbnailURL":"t","actions":[{"label":"LEFT_CTA","url":"x"},{"label":"CTA","url":"https://www.twitch.tv/xqc"}]}}]}}}"#,
        )
        .unwrap();
        let conn = d.current_user.unwrap().notifications.unwrap();
        let item = conn.edges.into_iter().next().unwrap().node.unwrap().into_item();
        assert_eq!(item.url.as_deref(), Some("https://www.twitch.tv/xqc"));
        assert_eq!((item.kind.as_str(), item.read), ("streamup", false));
    }

    #[test]
    fn hermes_payloads() {
        let p = json!({"type":"create-notification","data":{"notification":{"id":"n2","body_md":"**a** went live",
            "type":"streamup","created_at":"x","read":false,"thumbnail_url":"",
            "actions":[{"label":"CTA","url":"https://www.twitch.tv/a"}]},"persistent":true,"toast":true,"display_type":"VIEWER"}});
        let Some(Live::Created {
            item,
            toast,
            list,
            creator,
        }) = from_hermes(&p)
        else {
            panic!()
        };
        assert_eq!(
            (item.body.as_str(), item.thumbnail, toast, list, creator),
            ("**a** went live", None, true, true, false)
        );
        let s = json!({"type":"update-summary","data":"{\"summary\":{\"unseen_view_count\":4}}"});
        assert_eq!(from_hermes(&s), Some(Live::Unseen(4)));
        let r = json!({"type":"read-notifications","data":{"notification_ids":["a","b"]}});
        assert_eq!(from_hermes(&r), Some(Live::Read(vec!["a".into(), "b".into()])));
        assert_eq!(
            from_hermes(&json!({"type":"delete-notification","data":{"notification_id":"z"}})),
            Some(Live::Deleted("z".into()))
        );
    }

    #[test]
    fn settings_parse() {
        let d: Me<SettingsUser> = serde_json::from_str(
            r#"{"currentUser":{"id":"1","notificationSettings":[{"category":"LIVE","platforms":[
                {"platformName":"email","settingState":"off","isEnabled":true},{"platformName":"push","settingState":null,"isEnabled":true}]}],
              "notificationEnrollmentFollows":{"edges":[{"cursor":"c","notificationSettings":{"followsSettingState":"NEVER"},
                "node":{"id":"9","login":"a","displayName":"A","profileImageURL":null}}],"pageInfo":{"hasNextPage":true}}}}"#,
        )
        .unwrap();
        let me = d.current_user.unwrap();
        let cats = me.notification_settings.unwrap();
        let live = cats[0].as_ref().unwrap();
        assert_eq!(live.platforms.len(), 2);
        let edge = &me.notification_enrollment_follows.unwrap().edges[0];
        assert_eq!(
            edge.notification_settings
                .as_ref()
                .unwrap()
                .follows_setting_state
                .as_deref(),
            Some("NEVER")
        );
    }
}
