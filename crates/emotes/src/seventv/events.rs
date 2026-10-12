//! 7TV EventAPI frames (research/providers.md): what arrives, turned into plain events.
//! Emotes in these frames come in v3's shape.

use cyex_chat::model::Provider;
use serde::Deserialize;
use serde_json::Value;

use super::paint::Paint;
use crate::cosmetics::BadgeDef;
use crate::model::Emote;

/// One frame from the server.
#[derive(Debug, PartialEq)]
pub enum Frame {
    Hello {
        heartbeat_ms: u64,
    },
    Heartbeat,
    /// Reconnect now (op 4 or 7); subscriptions have to be sent again.
    Reconnect,
    Error(String),
    Event(Box<Event>),
    /// ACKs and anything this client doesn't use.
    Other,
}

#[derive(Debug, PartialEq)]
pub enum Event {
    Paint(Paint),
    Badge(BadgeDef),
    /// A user got (or lost) a paint, badge or emote set.
    Entitlement {
        added: bool,
        kind: Entitled,
        ref_id: String,
        /// Twitch user ids of the 7TV user's connections.
        twitch_ids: Vec<String>,
    },
    /// Emotes added, removed or renamed in a set.
    SetChanged(SetChange),
    /// A 7TV user switched a connection's active emote set.
    ActiveSet {
        twitch_id: String,
        set_id: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entitled {
    Paint,
    Badge,
    EmoteSet,
}

#[derive(Debug, Default, PartialEq)]
pub struct SetChange {
    pub set_id: String,
    /// Who did it, display name. `None` for sets filled by the server.
    pub actor: Option<String>,
    pub added: Vec<Emote>,
    /// Ids of emotes taken out.
    pub removed: Vec<(String, String)>,
    /// (id, old name, new emote).
    pub renamed: Vec<(String, String, Emote)>,
}

pub fn parse(text: &str) -> Frame {
    let Ok(raw) = serde_json::from_str::<RawFrame>(text) else {
        return Frame::Other;
    };
    match raw.op {
        0 => raw
            .d
            .and_then(dispatch)
            .map_or(Frame::Other, |e| Frame::Event(Box::new(e))),
        1 => Frame::Hello {
            heartbeat_ms: raw
                .d
                .as_ref()
                .and_then(|d| d["heartbeat_interval"].as_u64())
                .unwrap_or(45_000),
        },
        2 => Frame::Heartbeat,
        4 | 7 => Frame::Reconnect,
        6 => Frame::Error(
            raw.d
                .as_ref()
                .and_then(|d| d["message"].as_str())
                .unwrap_or("error")
                .to_owned(),
        ),
        _ => Frame::Other,
    }
}

fn dispatch(d: Value) -> Option<Event> {
    let kind = d["type"].as_str()?.to_owned();
    let body = d.get("body")?.clone();
    match kind.as_str() {
        "cosmetic.create" => {
            let c: RawCosmetic = serde_json::from_value(body["object"].clone()).ok()?;
            match c.kind.as_str() {
                "PAINT" => Some(Event::Paint(serde_json::from_value(c.data).ok()?)),
                "BADGE" => {
                    let b: RawBadge = serde_json::from_value(c.data).ok()?;
                    let (url, still) = b.host.pick("1x")?;
                    let (url2, _) = b.host.pick("2x").unwrap_or_else(|| (url.clone(), url.clone()));
                    let (url4, _) = b.host.pick("4x").unwrap_or_else(|| (url2.clone(), url2.clone()));
                    Some(Event::Badge(BadgeDef {
                        provider: Provider::Seventv,
                        id: b.id,
                        title: b.tooltip,
                        image1x: url,
                        image2x: url2,
                        image4x: url4,
                        still: Some(still),
                        color: None,
                        replaces: None,
                    }))
                }
                _ => None,
            }
        }
        "entitlement.create" | "entitlement.delete" => {
            let e: RawEntitlement = serde_json::from_value(body["object"].clone()).ok()?;
            let kind = match e.kind.as_str() {
                "PAINT" => Entitled::Paint,
                "BADGE" => Entitled::Badge,
                "EMOTE_SET" => Entitled::EmoteSet,
                _ => return None,
            };
            Some(Event::Entitlement {
                added: d["type"] == "entitlement.create",
                kind,
                ref_id: e.ref_id,
                twitch_ids: e.user.map(|u| u.twitch_ids()).unwrap_or_default(),
            })
        }
        "emote_set.update" => {
            let b: RawSetUpdate = serde_json::from_value(body).ok()?;
            Some(Event::SetChanged(b.into()))
        }
        "user.update" => active_set(&body),
        _ => None,
    }
}

/// `user.update`: `updated[{key:"connections", value:[{key:"emote_set", value:{id}}]}]`.
fn active_set(body: &Value) -> Option<Event> {
    let twitch_id = body
        .pointer("/actor/connections")
        .or_else(|| body.pointer("/object/connections"))
        .and_then(Value::as_array)
        .and_then(|cs| cs.iter().find(|c| c["platform"] == "TWITCH"))
        .and_then(|c| c["id"].as_str());
    for u in body["updated"].as_array()? {
        if u["key"] != "connections" {
            continue;
        }
        for v in u["value"].as_array().into_iter().flatten() {
            if v["key"] == "emote_set" {
                let set_id = v.pointer("/value/id").and_then(Value::as_str).map(str::to_owned);
                return Some(Event::ActiveSet {
                    twitch_id: twitch_id?.to_owned(),
                    set_id,
                });
            }
        }
    }
    None
}

#[derive(Deserialize)]
struct RawFrame {
    op: u8,
    d: Option<Value>,
}

#[derive(Deserialize)]
struct RawCosmetic {
    kind: String,
    #[serde(default)]
    data: Value,
}

#[derive(Deserialize)]
struct RawBadge {
    id: String,
    tooltip: String,
    host: RawHost,
}

#[derive(Deserialize)]
pub(crate) struct RawHost {
    url: String,
    #[serde(default)]
    files: Vec<RawFile>,
}

#[derive(Deserialize)]
struct RawFile {
    name: String,
    static_name: Option<String>,
    width: Option<u16>,
    height: Option<u16>,
    #[serde(default)]
    frame_count: u32,
}

impl RawHost {
    fn base(&self) -> String {
        match self.url.strip_prefix("//") {
            Some(rest) => format!("https://{rest}"),
            None => self.url.clone(),
        }
    }

    /// The `scale` WebP (`2x`) and its still frame.
    pub(crate) fn pick(&self, scale: &str) -> Option<(String, String)> {
        let f = self
            .files
            .iter()
            .find(|f| f.name == format!("{scale}.webp"))
            .or_else(|| self.files.iter().find(|f| f.name.starts_with(scale)))?;
        let base = self.base();
        let url = format!("{base}/{}", f.name);
        let still = match &f.static_name {
            Some(s) if f.frame_count > 1 => format!("{base}/{s}"),
            _ => url.clone(),
        };
        Some((url, still))
    }

    fn size_1x(&self) -> Option<(u16, u16)> {
        let f = self.files.iter().find(|f| f.name.starts_with("1x"))?;
        Some((f.width?, f.height?))
    }
}

#[derive(Deserialize)]
struct RawEntitlement {
    kind: String,
    ref_id: String,
    user: Option<RawUser>,
}

#[derive(Deserialize)]
struct RawUser {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    connections: Vec<RawConnection>,
}

#[derive(Deserialize)]
struct RawConnection {
    id: String,
    platform: String,
}

impl RawUser {
    fn twitch_ids(&self) -> Vec<String> {
        self.connections
            .iter()
            .filter(|c| c.platform == "TWITCH")
            .map(|c| c.id.clone())
            .collect()
    }
}

#[derive(Deserialize)]
struct RawSetUpdate {
    id: String,
    actor: Option<RawUser>,
    #[serde(default)]
    pushed: Vec<RawChange>,
    #[serde(default)]
    pulled: Vec<RawChange>,
    #[serde(default)]
    updated: Vec<RawChange>,
}

#[derive(Deserialize)]
struct RawChange {
    key: String,
    value: Option<RawActive>,
    old_value: Option<RawActive>,
}

/// v3 `ActiveEmote`.
#[derive(Deserialize)]
pub(crate) struct RawActive {
    id: String,
    name: String,
    #[serde(default)]
    flags: u32,
    actor_id: Option<String>,
    data: Option<RawEmoteData>,
}

#[derive(Deserialize)]
struct RawEmoteData {
    name: String,
    #[serde(default = "yes")]
    listed: bool,
    #[serde(default)]
    animated: bool,
    owner: Option<RawUser>,
    host: Option<RawHost>,
}

fn yes() -> bool {
    true
}

impl RawActive {
    pub(crate) fn emote(self) -> Emote {
        let mut e = Emote::new(Provider::Seventv, self.id, self.name);
        e.zero_width = self.flags & 1 != 0;
        e.added_by = self.actor_id;
        if let Some(d) = self.data {
            e.original = (d.name != e.name).then_some(d.name);
            e.unlisted = !d.listed;
            e.animated = d.animated;
            e.size = d.host.as_ref().and_then(RawHost::size_1x);
            e.owner = d.owner.and_then(|o| o.display_name);
        }
        e
    }
}

impl From<RawSetUpdate> for SetChange {
    fn from(u: RawSetUpdate) -> Self {
        let mut c = SetChange {
            set_id: u.id,
            actor: u.actor.and_then(|a| a.display_name),
            ..SetChange::default()
        };
        for p in u.pushed.into_iter().filter(|p| p.key == "emotes") {
            // The server only fills personal sets with emotes approved for that.
            if let Some(v) = p.value {
                c.added.push(v.emote());
            }
        }
        for p in u.pulled.into_iter().filter(|p| p.key == "emotes") {
            if let Some(v) = p.old_value {
                c.removed.push((v.id, v.name));
            }
        }
        for p in u.updated.into_iter().filter(|p| p.key == "emotes") {
            if let (Some(old), Some(new)) = (p.old_value, p.value) {
                c.renamed.push((old.id, old.name, new.emote()));
            }
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(name: &str) -> Frame {
        let path = format!("{}/tests/fixtures/7tv/{name}", env!("CARGO_MANIFEST_DIR"));
        parse(&std::fs::read_to_string(path).unwrap())
    }

    fn event(name: &str) -> Event {
        match frame(name) {
            Frame::Event(e) => *e,
            f => panic!("{f:?}"),
        }
    }

    #[test]
    fn control_frames() {
        assert_eq!(
            frame("eventapi_hello.json"),
            Frame::Hello { heartbeat_ms: 45_701 }
        );
        assert_eq!(frame("eventapi_heartbeat.json"), Frame::Heartbeat);
        assert_eq!(frame("eventapi_ack_subscribe.json"), Frame::Other);
        assert_eq!(parse(r#"{"op":7,"d":{"code":4012}}"#), Frame::Reconnect);
        assert_eq!(parse("nonsense"), Frame::Other);
    }

    #[test]
    fn badges_and_paints() {
        let Event::Badge(b) = event("eventapi_cosmetic_badge.json") else {
            panic!()
        };
        assert_eq!(b.title, "7TV Subscriber (3 Years)");
        assert!(b.image1x.starts_with("https://cdn.7tv.app/badge/"));
        assert!(b.still.unwrap().ends_with("1x_static.webp"));
        let Event::Paint(p) = event("eventapi_cosmetic_paint.json") else {
            panic!()
        };
        assert_eq!(p.name, "Ice Cold");
    }

    #[test]
    fn entitlements_name_twitch_users() {
        let Event::Entitlement {
            added,
            kind,
            ref_id,
            twitch_ids,
        } = event("eventapi_entitlement_paint.json")
        else {
            panic!()
        };
        assert!(added);
        assert_eq!(kind, Entitled::Paint);
        assert_eq!(ref_id, "01GEWY836G000EZEK94KRK9Y6T");
        assert_eq!(twitch_ids, ["100816422"]);
        let Event::Entitlement { kind, .. } = event("eventapi_entitlement_emote_set.json") else {
            panic!()
        };
        assert_eq!(kind, Entitled::EmoteSet);
    }

    #[test]
    fn set_changes() {
        let Event::SetChanged(c) = event("eventapi_emote_set_update_channel.synthetic.json") else {
            panic!()
        };
        assert_eq!(c.actor.as_deref(), Some("xQc"));
        assert_eq!(c.added[0].name, "notL");
        assert!(c.added[0].zero_width);
        assert_eq!(c.removed[0].1, "GAMBA");
        assert_eq!(c.renamed.len(), 1);
        let Event::SetChanged(c) = event("eventapi_emote_set_update_presence.json") else {
            panic!()
        };
        assert!(c.actor.is_none());
        assert_eq!(c.added[0].name, "nnysClap");
    }
}
