//! BTTV over HTTP (1.38): global and channel emotes, badges, bot commands (C13).

use cyex_chat::model::Provider;
use cyex_twitch::{Http, Result};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::cosmetics::BadgeDef;
use crate::model::{Emote, EmoteSet, Scope};

const API: &str = "https://api.betterttv.net/3";

/// Old BTTV emotes drawn on top of the one before (C7), as Chatterino lists them.
const ZERO_WIDTH: &[&str] = &[
    "SoSnowy",
    "IceCold",
    "SantaHat",
    "TopHat",
    "ReinDeer",
    "CandyCane",
    "cvMask",
    "cvHazmat",
];

pub async fn global(http: &Http) -> Result<EmoteSet> {
    let list: Vec<RawEmote> = http.get_json(&format!("{API}/cached/emotes/global")).await?;
    Ok(EmoteSet::new(
        "bttv-global",
        Provider::Bttv,
        Scope::Global,
        list.into_iter()
            .filter(|e| !e.modifier)
            .map(RawEmote::emote)
            .collect(),
    ))
}

/// The channel's own and shared emotes. `None` without a BTTV account.
pub async fn channel(http: &Http, twitch_id: &str) -> Result<Option<EmoteSet>> {
    let res: Result<RawUser> = http
        .get_json(&format!("{API}/cached/users/twitch/{twitch_id}"))
        .await;
    let user = match res {
        Ok(u) => u,
        Err(cyex_twitch::Error::Api { status: 404, .. }) => return Ok(None),
        Err(e) => return Err(e),
    };
    let emotes = user
        .channel_emotes
        .into_iter()
        .map(RawEmote::emote)
        .chain(user.shared_emotes.into_iter().map(RawEmote::shared))
        .collect();
    Ok(Some(EmoteSet::new(
        format!("bttv-{twitch_id}"),
        Provider::Bttv,
        Scope::Channel,
        emotes,
    )))
}

/// BTTV's staff and helper badges, and who has which (Twitch user ids).
pub async fn badges(http: &Http) -> Result<(Vec<BadgeDef>, Vec<(String, String)>)> {
    let list: Vec<RawBadgeUser> = http.get_json(&format!("{API}/cached/badges/twitch")).await?;
    Ok(split_badges(list))
}

fn split_badges(list: Vec<RawBadgeUser>) -> (Vec<BadgeDef>, Vec<(String, String)>) {
    let mut defs: Vec<BadgeDef> = Vec::new();
    let mut users = Vec::new();
    for u in list {
        let id = u.badge.kind.to_string();
        if !defs.iter().any(|d| d.id == id) {
            defs.push(badge(&id, &u.badge.description, &u.badge.svg));
        }
        users.push((u.provider_id, id));
    }
    (defs, users)
}

/// A BTTV badge drawn from one picture at every size.
pub fn badge(id: &str, title: &str, url: &str) -> BadgeDef {
    BadgeDef {
        provider: Provider::Bttv,
        id: id.to_owned(),
        title: title.to_owned(),
        image1x: url.to_owned(),
        image2x: url.to_owned(),
        image4x: url.to_owned(),
        still: None,
        color: None,
        replaces: None,
    }
}

/// Who uploaded an emote, for its card: display name and BTTV user id.
pub async fn uploader(http: &Http, emote_id: &str) -> Result<Option<(String, String)>> {
    #[derive(Deserialize)]
    struct Detail {
        user: Option<RawUploader>,
    }
    let d: Detail = http.get_json(&format!("{API}/emotes/{emote_id}")).await?;
    Ok(d.user.map(|u| (u.display_name, u.id)))
}

/// C13 A chat bot's command in this channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BotCommand {
    /// `!discord`
    pub name: String,
    /// nightbot, fossabot, moobot, streamelements.
    pub bot: String,
    /// Who may use it: 0 everyone, 1 subs, 2 regulars, 3 VIPs, 4 mods, 5 the streamer.
    pub level: u8,
    /// Argument names, e.g. `User`.
    pub args: Vec<String>,
}

/// C13 Nightbot, Fossabot, StreamElements and Moobot commands, through BTTV.
pub async fn bot_commands(http: &Http, twitch_id: &str) -> Result<Vec<BotCommand>> {
    let res: Result<Vec<RawCommand>> = http
        .get_json(&format!("{API}/cached/users/twitch/{twitch_id}/autocomplete"))
        .await;
    let raw = match res {
        Ok(r) => r,
        Err(cyex_twitch::Error::Api { status: 404, .. }) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out: Vec<BotCommand> = raw.into_iter().filter_map(RawCommand::command).collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name == b.name);
    Ok(out)
}

fn level(name: &str) -> u8 {
    match name {
        "everyone" => 0,
        "subscriber" => 1,
        "regular" => 2,
        "twitch_vip" | "vip" => 3,
        "moderator" => 4,
        "owner" | "broadcaster" => 5,
        _ => 0,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEmote {
    id: String,
    code: String,
    #[serde(default)]
    animated: bool,
    #[serde(default)]
    modifier: bool,
    width: Option<u16>,
    height: Option<u16>,
    code_original: Option<String>,
    user: Option<RawUploader>,
    /// A channel's own emotes name their uploader by id only.
    user_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUploader {
    id: String,
    display_name: String,
}

impl RawEmote {
    fn emote(self) -> Emote {
        let mut e = Emote::new(Provider::Bttv, self.id, self.code);
        e.zero_width = ZERO_WIDTH.contains(&e.name.as_str());
        e.animated = self.animated;
        e.size = Some((self.width.unwrap_or(28), self.height.unwrap_or(28)));
        e.original = self.code_original.filter(|o| *o != e.name);
        e.owner_key = self.user.as_ref().map(|u| u.id.clone()).or(self.user_id);
        e.owner = self.user.map(|u| u.display_name);
        e
    }

    /// 1.41 Another channel's emote this one added.
    fn shared(self) -> Emote {
        let mut e = self.emote();
        e.shared = true;
        e
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    #[serde(default)]
    channel_emotes: Vec<RawEmote>,
    #[serde(default)]
    shared_emotes: Vec<RawEmote>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBadgeUser {
    provider_id: String,
    badge: RawBadge,
}

#[derive(Deserialize)]
struct RawBadge {
    #[serde(rename = "type")]
    kind: u32,
    description: String,
    svg: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCommand {
    name: String,
    provider: String,
    #[serde(default)]
    user_level: serde_json::Value,
    #[serde(default)]
    arguments: Vec<RawArg>,
}

#[derive(Deserialize)]
struct RawArg {
    name: String,
}

impl RawCommand {
    fn command(self) -> Option<BotCommand> {
        if !self.name.starts_with('!') || self.name.len() < 2 {
            return None;
        }
        // Fossabot lists every level allowed; the lowest one counts.
        let level = match &self.user_level {
            serde_json::Value::String(s) => level(s),
            serde_json::Value::Array(a) => a.iter().filter_map(|v| v.as_str()).map(level).min().unwrap_or(0),
            _ => 0,
        };
        Some(BotCommand {
            name: self.name,
            bot: self.provider,
            level,
            args: self.arguments.into_iter().map(|a| a.name).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture<T: serde::de::DeserializeOwned>(name: &str) -> T {
        let path = format!("{}/tests/fixtures/bttv/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn global_skips_modifier_words() {
        let list: Vec<RawEmote> = fixture("global_emotes.json");
        let emotes: Vec<Emote> = list
            .into_iter()
            .filter(|e| !e.modifier)
            .map(RawEmote::emote)
            .collect();
        assert!(!emotes.iter().any(|e| e.name == "w!"));
        let hazmat = emotes.iter().find(|e| e.name == "cvHazmat").unwrap();
        assert!(hazmat.zero_width);
        let angel = emotes.iter().find(|e| e.name == "AngelThump").unwrap();
        assert_eq!(angel.size, Some((84, 28)));
        assert!(emotes.iter().find(|e| e.name == "SourPls").unwrap().animated);
    }

    #[test]
    fn channel_has_own_and_shared() {
        let u: RawUser = fixture("user.json");
        let emotes: Vec<Emote> = u
            .channel_emotes
            .into_iter()
            .map(RawEmote::emote)
            .chain(u.shared_emotes.into_iter().map(RawEmote::shared))
            .collect();
        assert_eq!(emotes.len(), 4);
        let duur = emotes.iter().find(|e| e.name == "DUUR").unwrap();
        assert_eq!(duur.original.as_deref(), Some("FeelsTastyMan"));
        assert_eq!(duur.owner.as_deref(), Some("TastyMcPanda"));
        // 1.41 Shared, with the uploader's BTTV id; own emotes by userId.
        assert!(duur.shared);
        assert_eq!(duur.owner_key.as_deref(), Some("561c219aef87f8db05aca668"));
        let own = emotes.iter().find(|e| e.name == "fuBaldi").unwrap();
        assert!(!own.shared);
        assert_eq!(own.owner_key.as_deref(), Some("574e63ce66a2002d3bcc6f92"));
    }

    #[test]
    fn badges_by_kind() {
        let (defs, users) = split_badges(fixture("badges.json"));
        assert_eq!(defs.len(), 3);
        assert_eq!(users[0], ("11785491".to_owned(), "1".to_owned()));
        assert!(defs[0].image1x.ends_with("developer.svg"));
    }

    #[test]
    fn bot_commands_keep_bang_names_and_lowest_level() {
        let raw: Vec<RawCommand> = fixture("autocomplete.json");
        let cmds: Vec<BotCommand> = raw.into_iter().filter_map(RawCommand::command).collect();
        assert!(!cmds.iter().any(|c| c.name == "~wut"));
        let reset = cmds.iter().find(|c| c.name == "!resetdead").unwrap();
        assert_eq!(reset.level, 4);
        assert_eq!(cmds.iter().find(|c| c.name == "!webcam").unwrap().args, ["User"]);
    }
}
