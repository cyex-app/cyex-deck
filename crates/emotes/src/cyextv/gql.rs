//! V112-15 cyex.tv GraphQL (`POST /v1/gql`) with the app session's Bearer: own cosmetics
//! (`myCosmetics`, `selectCosmetics`) and, for the chat commands (Phase 6), the sets of a channel
//! you may change and emote adds, renames and removals. Names checked against the platform's
//! `schema/schema.graphql` (feat/app-sign-in 0b89a17). The token only ever goes to the client's
//! own URL and never into an error.

use cyex_twitch::Http;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use ts_rs::TS;

use crate::seventv::paint::PaintStyle;

/// Where pictures may come from (anything else is dropped).
const PICTURES: &str = "https://cdn.cyex.tv/";
/// Largest answer read.
const MAX: usize = 2 << 20;

/// What a call can come back with, worded for people.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GqlError {
    /// 401 or `signed_out`/`unauthorized`: the access token is gone (refresh, then sign in again).
    SignedOut,
    /// `insufficient_scope`: the app session may not do this.
    Scope,
    Forbidden,
    NotFound,
    RateLimited,
    Down,
    /// Any other refusal, with cyex.tv's own (short) message.
    Said(String),
}

impl std::fmt::Display for GqlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SignedOut => f.write_str("Sign in to cyex.tv again."),
            Self::Scope => f.write_str("cyex deck can’t do that. Use cyex.tv for it."),
            Self::Forbidden => f.write_str("You can’t change that."),
            Self::NotFound => f.write_str("That isn’t on cyex.tv anymore."),
            Self::RateLimited => f.write_str("Too many changes. Wait a minute."),
            Self::Down => f.write_str("cyex.tv isn’t answering. Try again soon."),
            Self::Said(m) => f.write_str(m),
        }
    }
}

/// A GraphQL error code (`extensions.code`) as one of ours.
fn from_code(code: &str, message: &str) -> GqlError {
    match code {
        "signed_out" | "unauthorized" | "unauthenticated" | "session_expired" => GqlError::SignedOut,
        "insufficient_scope" => GqlError::Scope,
        "forbidden" | "not_owned" => GqlError::Forbidden,
        "not_found" => GqlError::NotFound,
        "rate_limited" => GqlError::RateLimited,
        "internal" => GqlError::Down,
        _ if message.trim().is_empty() => GqlError::Down,
        _ => GqlError::Said(message.trim().chars().take(200).collect()),
    }
}

/// The status and body of a call to `data`, or the first error.
pub fn read_answer(status: u16, body: &[u8]) -> Result<Value, GqlError> {
    match status {
        401 => return Err(GqlError::SignedOut),
        403 => {
            let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
            let code = v.get("code").or_else(|| v.pointer("/error/code")).and_then(Value::as_str).unwrap_or("");
            return Err(if code == "insufficient_scope" { GqlError::Scope } else { GqlError::Forbidden });
        }
        429 => return Err(GqlError::RateLimited),
        500.. => return Err(GqlError::Down),
        _ => {}
    }
    let v: Value = serde_json::from_slice(body).map_err(|_| GqlError::Down)?;
    if let Some(e) = v.get("errors").and_then(Value::as_array).and_then(|e| e.first()) {
        let code = e.pointer("/extensions/code").and_then(Value::as_str).unwrap_or("");
        let message = e.get("message").and_then(Value::as_str).unwrap_or("");
        return Err(from_code(code, message));
    }
    if !(200..300).contains(&status) {
        return Err(GqlError::Down);
    }
    match v.get("data") {
        Some(d) if !d.is_null() => Ok(d.clone()),
        _ => Err(GqlError::Down),
    }
}

// ---- What the deck reads ----

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TvAccount")]
pub struct Account {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub avatar: Option<String>,
}

/// A picture: 1x and the `srcset`, from cdn.cyex.tv only; `still` for reduced motion.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TvPicture")]
pub struct Picture {
    pub src: String,
    pub srcset: String,
    pub still: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TvBadge")]
pub struct Badge {
    pub id: String,
    pub name: String,
    pub tooltip: String,
    pub picture: Option<Picture>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TvStop")]
pub struct Stop {
    pub at: f64,
    pub color: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TvShadow")]
pub struct Shadow {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub color: String,
}

/// A name paint. Colours are `#RRGGBB(AA)` (others dropped); at most 10 stops and 4 shadows.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TvPaint")]
pub struct Paint {
    pub id: String,
    pub name: String,
    /// `linear`, `radial` or `image`.
    pub kind: String,
    pub angle: i32,
    pub repeat: bool,
    /// `circle` or `ellipse`.
    pub shape: String,
    pub color: Option<String>,
    pub stops: Vec<Stop>,
    pub shadows: Vec<Shadow>,
    pub image: Option<String>,
    /// V112-19 The paint as chat draws it (the same conversion as the catalog's): the picker
    /// shows names with this style. `None` when there is nothing to draw.
    pub style: Option<PaintStyle>,
}

/// What you own and show (`OwnedCosmetics`).
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "TvOwned")]
pub struct Owned {
    pub account: Account,
    pub badges: Vec<Badge>,
    pub paints: Vec<Paint>,
    pub badge_id: Option<String>,
    pub paint_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFile {
    name: String,
    #[serde(default)]
    format: String,
    #[serde(default)]
    scale: u8,
    #[serde(default)]
    is_static: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawImage {
    host: String,
    #[serde(default)]
    files: Vec<RawFile>,
    #[serde(default)]
    animated: bool,
}

#[derive(Deserialize)]
struct RawBadge {
    id: String,
    name: String,
    #[serde(default)]
    tooltip: String,
    image: Option<RawImage>,
}

#[derive(Deserialize)]
struct RawStop {
    at: f64,
    color: String,
}

#[derive(Deserialize)]
struct RawShadow {
    x: f64,
    y: f64,
    radius: f64,
    color: String,
}

#[derive(Deserialize)]
struct RawPaint {
    id: String,
    name: String,
    kind: String,
    #[serde(default)]
    angle: i32,
    #[serde(default)]
    repeat: bool,
    #[serde(default)]
    shape: String,
    color: Option<String>,
    #[serde(default)]
    stops: Vec<RawStop>,
    #[serde(default)]
    shadows: Vec<RawShadow>,
    image: Option<RawImage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOwned {
    account: Account,
    #[serde(default)]
    badges: Vec<RawBadge>,
    #[serde(default)]
    paints: Vec<RawPaint>,
    badge_id: Option<String>,
    paint_id: Option<String>,
}

/// `#RRGGBB` or `#RRGGBBAA`.
pub fn color_ok(c: &str) -> bool {
    c.strip_prefix('#')
        .is_some_and(|h| matches!(h.len(), 6 | 8) && h.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// The webp files at 1x, 2x and 4x (animated, or the stills).
fn picture(img: &RawImage) -> Option<Picture> {
    let host = img.host.trim_end_matches('/');
    let safe = |n: &str| !n.is_empty() && n.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if !host.starts_with(PICTURES) || host.contains(['"', '\'', ' ', '(', ')', '\\', '?', '#']) {
        return None;
    }
    let pick = |still: bool| -> Vec<(u8, String)> {
        let mut out: Vec<(u8, String)> = img
            .files
            .iter()
            .filter(|f| f.format.eq_ignore_ascii_case("webp") && f.is_static == still && safe(&f.name) && (1..=4).contains(&f.scale))
            .map(|f| (f.scale, format!("{host}/{}", f.name)))
            .collect();
        out.sort_by_key(|(s, _)| *s);
        out.dedup_by_key(|(s, _)| *s);
        out
    };
    let moving = pick(false);
    let stills = pick(true);
    let main = if moving.is_empty() { &stills } else { &moving };
    let (_, src) = main.first()?.clone();
    let srcset = main.iter().map(|(s, u)| format!("{u} {s}x")).collect::<Vec<_>>().join(", ");
    let still = (img.animated && !moving.is_empty()).then(|| stills.first().map(|(_, u)| u.clone())).flatten();
    Some(Picture { src, srcset, still })
}

fn paint(p: RawPaint) -> Paint {
    let kind = match p.kind.to_ascii_uppercase().as_str() {
        "RADIAL_GRADIENT" => "radial",
        "IMAGE" => "image",
        _ => "linear",
    };
    Paint {
        id: p.id,
        name: p.name,
        kind: kind.to_owned(),
        angle: p.angle.rem_euclid(360),
        repeat: p.repeat,
        shape: if p.shape.eq_ignore_ascii_case("ellipse") { "ellipse" } else { "circle" }.to_owned(),
        color: p.color.filter(|c| color_ok(c)),
        stops: p
            .stops
            .into_iter()
            .filter(|s| color_ok(&s.color) && s.at.is_finite())
            .take(10)
            .map(|s| Stop { at: s.at.clamp(0.0, 1.0), color: s.color })
            .collect(),
        shadows: p
            .shadows
            .into_iter()
            .filter(|s| color_ok(&s.color) && [s.x, s.y, s.radius].iter().all(|n| n.is_finite()))
            .take(4)
            .map(|s| Shadow {
                x: s.x.clamp(-16.0, 16.0),
                y: s.y.clamp(-16.0, 16.0),
                radius: s.radius.clamp(0.0, 16.0),
                color: s.color,
            })
            .collect(),
        image: p.image.as_ref().and_then(picture).map(|pic| pic.src),
        style: None,
    }
}

/// The chat renderer's style for one GraphQL paint: its answer in the REST catalog's shape.
fn style_of(v: &Value) -> Option<PaintStyle> {
    let mut v = v.clone();
    let kind = v.get("kind").and_then(Value::as_str)?.to_ascii_lowercase();
    v["kind"] = Value::from(kind);
    if let Some(img) = v.get_mut("image").and_then(Value::as_object_mut) {
        img.insert("ready".into(), Value::Bool(true));
    }
    super::cosmetics::paint_style(&v)
}

/// An `OwnedCosmetics` object.
pub fn parse_owned(v: &Value) -> serde_json::Result<Owned> {
    let raw = RawOwned::deserialize(v)?;
    let badges: Vec<Badge> = raw
        .badges
        .into_iter()
        .map(|b| Badge {
            picture: b.image.as_ref().and_then(picture),
            id: b.id,
            name: b.name,
            tooltip: b.tooltip,
        })
        .collect();
    let styles: Vec<Option<PaintStyle>> = v
        .get("paints")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(style_of).collect())
        .unwrap_or_default();
    let paints: Vec<Paint> = raw
        .paints
        .into_iter()
        .enumerate()
        .map(|(i, p)| Paint {
            style: styles.get(i).cloned().flatten(),
            ..paint(p)
        })
        .collect();
    // Only ids of things owned count as shown.
    let badge_id = raw.badge_id.filter(|id| badges.iter().any(|b| &b.id == id));
    let paint_id = raw.paint_id.filter(|id| paints.iter().any(|p| &p.id == id));
    Ok(Owned {
        account: raw.account,
        badges,
        paints,
        badge_id,
        paint_id,
    })
}

// ---- Set writes (Phase 6) ----

/// `SetAccess`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Access {
    Owner,
    AccountEditor,
    SetEditor,
    Staff,
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSummary {
    pub id: String,
    pub name: String,
    pub capacity: u32,
    pub slots_used: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditableSet {
    pub set: SetSummary,
    pub active_here: bool,
    pub access: Access,
}

/// `channelEditableSets`: the channel's active set and the sets you may change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditableSets {
    pub active_set_id: Option<String>,
    pub login: String,
    /// `OWNER`, `ACCOUNT_EDITOR`, `SET_EDITOR`; `None` when you have no role.
    pub role: Option<String>,
    pub can_edit_active_set: bool,
    pub sets: Vec<EditableSet>,
}

/// A `SetEmote` as an add or rename answers it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEmote {
    pub name: String,
    pub zero_width: bool,
    pub emote: EmoteRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EmoteRef {
    pub id: String,
    pub name: String,
}

/// `Platform`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Twitch,
    Kick,
}

impl Platform {
    fn gql(self) -> &'static str {
        match self {
            Self::Twitch => "TWITCH",
            Self::Kick => "KICK",
        }
    }
}

fn parse_editable(v: &Value) -> Option<EditableSets> {
    let c = v.get("channel")?;
    Some(EditableSets {
        active_set_id: c.get("activeSetId").and_then(Value::as_str).map(str::to_owned),
        login: c.get("login").and_then(Value::as_str).unwrap_or_default().to_owned(),
        role: v.get("role").and_then(Value::as_str).map(str::to_owned),
        can_edit_active_set: v.get("canEditActiveSet").and_then(Value::as_bool).unwrap_or(false),
        sets: v
            .get("sets")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|s| EditableSet::deserialize(s).ok()).collect())
            .unwrap_or_default(),
    })
}

// ---- Queries ----

const PICTURE: &str = "host animated files { name format scale isStatic }";

fn owned_fields() -> String {
    format!(
        "account {{ id displayName avatar }} badgeId paintId \
         badges {{ id name tooltip image {{ {PICTURE} }} }} \
         paints {{ id name kind angle repeat shape color stops {{ at color }} \
         shadows {{ x y radius color }} image {{ {PICTURE} }} }}"
    )
}

const SET_EMOTE: &str = "name zeroWidth emote { id name }";

/// A choice for `selectCosmetics`: leave it as it is, take it off, or wear this one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice<'a> {
    Keep,
    None,
    Wear(&'a str),
}

/// The mutation with only the fields that change (a field left out is kept, ADR 0015).
pub fn select_query(badge: Choice<'_>, paint: Choice<'_>, owner: Option<&str>) -> (String, Value) {
    let mut decl = Vec::new();
    let mut args = Vec::new();
    let mut vars = Map::new();
    for (name, choice) in [("badgeId", badge), ("paintId", paint)] {
        let value = match choice {
            Choice::Keep => continue,
            Choice::None => Value::Null,
            Choice::Wear(id) => Value::from(id),
        };
        decl.push(format!("${name}: String"));
        args.push(format!("{name}: ${name}"));
        vars.insert(name.to_owned(), value);
    }
    if let Some(owner) = owner {
        decl.push("$ownerId: String".into());
        args.push("ownerId: $ownerId".into());
        vars.insert("ownerId".into(), Value::from(owner));
    }
    let head = if decl.is_empty() { String::new() } else { format!("({})", decl.join(", ")) };
    let call = if args.is_empty() { String::new() } else { format!("({})", args.join(", ")) };
    let q = format!("mutation SelectCosmetics{head} {{ selectCosmetics{call} {{ {} }} }}", owned_fields());
    (q, Value::Object(vars))
}

#[derive(Clone)]
pub struct Gql {
    http: Http,
    url: String,
}

impl Gql {
    /// `base` like `https://api.cyex.tv/v1` (or a loopback mock's `/v1`).
    pub fn new(http: Http, base: &str) -> Self {
        Self {
            http,
            url: format!("{}/gql", base.trim_end_matches('/')),
        }
    }

    async fn call(&self, bearer: &str, query: &str, vars: Value) -> Result<Value, GqlError> {
        let body = json!({ "query": query, "variables": vars });
        let bytes = serde_json::to_vec(&body).map_err(|_| GqlError::Down)?;
        let (status, out) = self
            .http
            .send("POST", &self.url, Some(bearer), Some(("application/json".into(), bytes)), MAX)
            .await
            .map_err(|_| GqlError::Down)?;
        read_answer(status, &out)
    }

    /// `myCosmetics`: what you own and show.
    pub async fn my_cosmetics(&self, bearer: &str) -> Result<Owned, GqlError> {
        let q = format!("query MyCosmetics {{ myCosmetics {{ {} }} }}", owned_fields());
        let d = self.call(bearer, &q, json!({})).await?;
        d.get("myCosmetics").and_then(|v| parse_owned(v).ok()).ok_or(GqlError::Down)
    }

    /// `selectCosmetics(badgeId, paintId, ownerId)`; answers with what you show now.
    pub async fn select_cosmetics(
        &self,
        bearer: &str,
        badge: Choice<'_>,
        paint: Choice<'_>,
        owner: Option<&str>,
    ) -> Result<Owned, GqlError> {
        let (q, vars) = select_query(badge, paint, owner);
        let d = self.call(bearer, &q, vars).await?;
        d.get("selectCosmetics").and_then(|v| parse_owned(v).ok()).ok_or(GqlError::Down)
    }

    /// `channelEditableSets(platform, id)`; `None` when cyex.tv doesn't know the channel.
    pub async fn channel_editable_sets(
        &self,
        bearer: &str,
        platform: Platform,
        id: &str,
    ) -> Result<Option<EditableSets>, GqlError> {
        let q = "query ChannelEditableSets($platform: Platform!, $id: String) { \
                 channelEditableSets(platform: $platform, id: $id) { \
                 channel { login activeSetId } role canEditActiveSet \
                 sets { set { id name capacity slotsUsed } activeHere access } } }";
        let d = self.call(bearer, q, json!({ "platform": platform.gql(), "id": id })).await?;
        match d.get("channelEditableSets") {
            None | Some(Value::Null) => Ok(None),
            Some(v) => parse_editable(v).map(Some).ok_or(GqlError::Down),
        }
    }

    /// `addSetEmote(setId, emoteId, input { name, zeroWidth })`.
    pub async fn add_set_emote(
        &self,
        bearer: &str,
        set_id: &str,
        emote_id: &str,
        name: Option<&str>,
        zero_width: Option<bool>,
    ) -> Result<SetEmote, GqlError> {
        let q = format!(
            "mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {{ \
             addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {{ {SET_EMOTE} }} }}"
        );
        let vars = json!({ "setId": set_id, "emoteId": emote_id, "input": input(name, zero_width) });
        let d = self.call(bearer, &q, vars).await?;
        set_emote(&d, "addSetEmote")
    }

    /// `updateSetEmote(setId, emoteId, input { name, zeroWidth })`: rename, or the zero-width flag.
    pub async fn update_set_emote(
        &self,
        bearer: &str,
        set_id: &str,
        emote_id: &str,
        name: Option<&str>,
        zero_width: Option<bool>,
    ) -> Result<SetEmote, GqlError> {
        let q = format!(
            "mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {{ \
             updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {{ {SET_EMOTE} }} }}"
        );
        let vars = json!({ "setId": set_id, "emoteId": emote_id, "input": input(name, zero_width) });
        let d = self.call(bearer, &q, vars).await?;
        set_emote(&d, "updateSetEmote")
    }

    /// `removeSetEmote(setId, emoteId)`; false when it wasn't in the set.
    pub async fn remove_set_emote(&self, bearer: &str, set_id: &str, emote_id: &str) -> Result<bool, GqlError> {
        let q = "mutation RemoveSetEmote($setId: String!, $emoteId: String!) { \
                 removeSetEmote(setId: $setId, emoteId: $emoteId) }";
        let d = self.call(bearer, q, json!({ "setId": set_id, "emoteId": emote_id })).await?;
        d.get("removeSetEmote").and_then(Value::as_bool).ok_or(GqlError::Down)
    }
}

/// `{ name, zeroWidth }` with only what is given.
fn input(name: Option<&str>, zero_width: Option<bool>) -> Value {
    let mut m = Map::new();
    if let Some(n) = name {
        m.insert("name".into(), Value::from(n));
    }
    if let Some(z) = zero_width {
        m.insert("zeroWidth".into(), Value::from(z));
    }
    Value::Object(m)
}

fn set_emote(d: &Value, field: &str) -> Result<SetEmote, GqlError> {
    d.get(field)
        .and_then(|v| SetEmote::deserialize(v).ok())
        .ok_or(GqlError::Down)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned() -> Value {
        json!({
            "account": { "id": "acc1", "displayName": "Ada", "avatar": null },
            "badgeId": "b1", "paintId": "gone",
            "badges": [
                { "id": "b1", "name": "Early", "tooltip": "Early supporter", "image": {
                    "host": "https://cdn.cyex.tv/cosmetics/badges/abc", "animated": true, "files": [
                        { "name": "2x.webp", "format": "WEBP", "scale": 2, "isStatic": false },
                        { "name": "1x.webp", "format": "WEBP", "scale": 1, "isStatic": false },
                        { "name": "1x.avif", "format": "AVIF", "scale": 1, "isStatic": false },
                        { "name": "1x_static.webp", "format": "WEBP", "scale": 1, "isStatic": true }
                    ] } },
                { "id": "b2", "name": "Evil", "tooltip": "", "image": {
                    "host": "https://evil.example/x", "animated": false, "files": [
                        { "name": "1x.webp", "format": "WEBP", "scale": 1, "isStatic": true } ] } }
            ],
            "paints": [
                { "id": "p1", "name": "Sunset", "kind": "LINEAR_GRADIENT", "angle": 450, "repeat": false,
                  "shape": "CIRCLE", "color": "#ff0000",
                  "stops": [ { "at": 0.0, "color": "#ff0000" }, { "at": 1.5, "color": "#0000ffcc" },
                             { "at": 0.5, "color": "red;}" } ],
                  "shadows": [ { "x": 40, "y": 1, "radius": 2, "color": "#000000" } ], "image": null },
                { "id": "p2", "name": "Pic", "kind": "IMAGE", "angle": 90, "repeat": true, "shape": "ELLIPSE",
                  "color": "url(x)", "stops": [], "shadows": [], "image": {
                    "host": "https://cdn.cyex.tv/cosmetics/paints/def", "animated": false, "files": [
                        { "name": "1x_static.webp", "format": "WEBP", "scale": 1, "isStatic": true } ] } }
            ]
        })
    }

    #[test]
    fn owned_cosmetics_parse_and_are_cleaned() {
        let o = parse_owned(&owned()).unwrap();
        assert_eq!(o.account.display_name, "Ada");
        assert_eq!(o.badge_id.as_deref(), Some("b1"));
        assert_eq!(o.paint_id, None, "not owned: not shown");
        let pic = o.badges[0].picture.clone().unwrap();
        assert_eq!(pic.src, "https://cdn.cyex.tv/cosmetics/badges/abc/1x.webp");
        assert_eq!(
            pic.srcset,
            "https://cdn.cyex.tv/cosmetics/badges/abc/1x.webp 1x, https://cdn.cyex.tv/cosmetics/badges/abc/2x.webp 2x"
        );
        assert_eq!(pic.still.as_deref(), Some("https://cdn.cyex.tv/cosmetics/badges/abc/1x_static.webp"));
        assert_eq!(o.badges[1].picture, None, "only cdn.cyex.tv");
        let p = &o.paints[0];
        assert_eq!((p.kind.as_str(), p.angle, p.shape.as_str()), ("linear", 90, "circle"));
        assert_eq!(p.stops.len(), 2, "bad colours dropped");
        assert_eq!(p.stops[1].at, 1.0);
        assert_eq!(p.shadows[0].x, 16.0);
        let p = &o.paints[1];
        assert_eq!((p.kind.as_str(), p.shape.as_str(), p.color.as_deref()), ("image", "ellipse", None));
        assert_eq!(p.image.as_deref(), Some("https://cdn.cyex.tv/cosmetics/paints/def/1x_static.webp"));
        // V112-19 The picker draws the chat's way: the same style as the catalog's conversion.
        let st = o.paints[0].style.as_ref().unwrap();
        assert_eq!(st.id, "ctvp1");
        assert!(st.css.contains("linear-gradient(") && st.css.contains("color:#ff0000;"), "{}", st.css);
        assert_eq!(o.paints[1].style, None, "nothing to draw");
    }

    #[test]
    fn colours() {
        assert!(color_ok("#a1b2c3") && color_ok("#A1B2C3FF"));
        assert!(!color_ok("a1b2c3") && !color_ok("#abc") && !color_ok("#gggggg") && !color_ok("#a1b2c3;"));
    }

    #[test]
    fn select_sends_only_what_changes() {
        let (q, v) = select_query(Choice::Wear("b1"), Choice::Keep, None);
        assert!(q.starts_with("mutation SelectCosmetics($badgeId: String) { selectCosmetics(badgeId: $badgeId) {"), "{q}");
        assert_eq!(v, json!({ "badgeId": "b1" }));
        let (q, v) = select_query(Choice::Keep, Choice::None, Some("acc2"));
        assert!(q.contains("($paintId: String, $ownerId: String)") && q.contains("(paintId: $paintId, ownerId: $ownerId)"));
        assert_eq!(v, json!({ "paintId": null, "ownerId": "acc2" }));
        let (q, v) = select_query(Choice::Keep, Choice::Keep, None);
        assert!(q.starts_with("mutation SelectCosmetics { selectCosmetics {"), "{q}");
        assert_eq!(v, json!({}));
        assert!(q.contains("badges { id name tooltip image { host animated files { name format scale isStatic } } }"));
    }

    #[test]
    fn errors_map_to_short_lines() {
        let err = |code: &str, msg: &str| {
            json!({ "data": null, "errors": [{ "message": msg, "extensions": { "code": code } }] }).to_string().into_bytes()
        };
        assert_eq!(read_answer(200, &err("insufficient_scope", "x")), Err(GqlError::Scope));
        assert_eq!(read_answer(200, &err("forbidden", "x")), Err(GqlError::Forbidden));
        assert_eq!(read_answer(200, &err("not_found", "x")), Err(GqlError::NotFound));
        assert_eq!(read_answer(200, &err("signed_out", "x")), Err(GqlError::SignedOut));
        assert_eq!(read_answer(200, &err("rate_limited", "x")), Err(GqlError::RateLimited));
        assert_eq!(read_answer(200, &err("set_full", "That set is full.")), Err(GqlError::Said("That set is full.".into())));
        assert_eq!(read_answer(401, b""), Err(GqlError::SignedOut));
        assert_eq!(read_answer(403, br#"{"code":"insufficient_scope"}"#), Err(GqlError::Scope));
        assert_eq!(read_answer(403, b"{}"), Err(GqlError::Forbidden));
        assert_eq!(read_answer(429, b""), Err(GqlError::RateLimited));
        assert_eq!(read_answer(502, b""), Err(GqlError::Down));
        assert_eq!(read_answer(200, b"<html>"), Err(GqlError::Down));
        assert_eq!(read_answer(200, br#"{"data":{"x":1}}"#), Ok(json!({ "x": 1 })));
        assert_eq!(GqlError::Scope.to_string(), "cyex deck can’t do that. Use cyex.tv for it.");
    }

    #[test]
    fn editable_sets_and_set_emotes_parse() {
        let v = json!({
            "channel": { "login": "ada", "activeSetId": "s1" }, "role": "OWNER", "canEditActiveSet": true,
            "sets": [
                { "set": { "id": "s1", "name": "Main", "capacity": 600, "slotsUsed": 12 }, "activeHere": true, "access": "OWNER" },
                { "set": { "id": "s2", "name": "Alt", "capacity": 600, "slotsUsed": 0 }, "activeHere": false, "access": "SOMETHING_NEW" }
            ]
        });
        let e = parse_editable(&v).unwrap();
        assert_eq!((e.login.as_str(), e.active_set_id.as_deref(), e.role.as_deref()), ("ada", Some("s1"), Some("OWNER")));
        assert!(e.can_edit_active_set);
        assert_eq!(e.sets.len(), 2);
        assert_eq!(e.sets[0].access, Access::Owner);
        assert_eq!(e.sets[1].access, Access::Other);
        let d = json!({ "addSetEmote": { "name": "wave", "zeroWidth": false, "emote": { "id": "e1", "name": "Wave" } } });
        assert_eq!(set_emote(&d, "addSetEmote").unwrap().emote.id, "e1");
        assert_eq!(set_emote(&d, "updateSetEmote"), Err(GqlError::Down));
        assert_eq!(input(Some("x"), None), json!({ "name": "x" }));
        assert_eq!(input(None, Some(true)), json!({ "zeroWidth": true }));
    }
}
