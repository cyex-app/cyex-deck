//! V107-R38 Moderation settings for your own channel, the way Twitch's dashboard runs them
//! (Settings › Moderation): AutoMod levels, chat delay, email and phone verification, chat
//! rules, unban requests, blocked and permitted terms. Shapes checked against the live
//! schema on 2026/10/21 (docs/research/twitch-api.md). Follower, slow, sub and emote modes
//! stay in `modes` (1.35).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::mutate;
use crate::gql::{Auth, query};
use crate::helix::moderation::BlockedTerm;
use crate::{Error, Http, Result};

/// AutoMod's categories, in Twitch's order.
pub const CATEGORIES: [&str; 10] = [
    "ableism",
    "aggression",
    "homophobia",
    "misogyny",
    "nameCalling",
    "profanity",
    "racism",
    "sexualHarassment",
    "sexuality",
    "smartDetection",
];

/// Twitch's presets, levels 0 to 4, per category in `CATEGORIES` order (its dashboard's table).
pub const PRESETS: [[u8; 10]; 5] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 1, 0, 0, 2],
    [2, 2, 2, 2, 1, 0, 2, 2, 2, 2],
    [3, 3, 3, 3, 2, 0, 3, 3, 3, 4],
    [4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
];

/// Chat delays Twitch offers, in seconds.
pub const DELAYS: [u32; 4] = [0, 2, 4, 6];
const READ: &str = "query($id: ID!) {
  user(id: $id) { chatSettings { chatDelayMs rules accountVerificationOptions {
    emailVerificationMode partialEmailVerificationConfig { ...partial }
    phoneVerificationMode partialPhoneVerificationConfig { ...partial }
    isSubscriberExempt isVIPExempt isModeratorExempt } } }
  channel(id: $id) {
    autoModConfiguration { overallLevel detailedLevels { ableismLevel aggressionLevel homophobiaLevel
      misogynyLevel nameCallingLevel profanityLevel racismLevel sexualHarassmentLevel sexualityLevel
      smartDetectionLevel } }
    unbanRequestsSettings { isEnabled cooldownMinutes }
    blockedTerms(first: 100) { edges { node { id phrase createdAt expiresAt } } }
    permittedTerms(first: 100) { edges { node { id phrase createdAt expiresAt } } }
  }
}
fragment partial on ChatPartialVerificationConfig { minimumAccountAgeInMinutes minimumFollowerAgeInMinutes
  shouldRestrictBasedOnAccountAge shouldRestrictFirstTimeChatters shouldRestrictBasedOnFollowerAge }";
const CHAT: &str =
    "mutation($input: UpdateChatSettingsInput!) { updateChatSettings(input: $input) { error { code } } }";
const AUTOMOD: &str = "mutation($input: UpdateAutoModLevelsInput!) { \
    updateAutoModLevels(input: $input) { autoModConfiguration { overallLevel } } }";
const UNBAN: &str = "mutation($input: SetUnbanRequestsSettingsInput!) { \
    setUnbanRequestsSettings(input: $input) { error { code } } }";

/// Who must verify an email or phone before chatting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export, rename = "ModsetVerifyMode")]
pub enum VerifyMode {
    #[default]
    None,
    All,
    /// Only who matches `Partial`.
    Partial,
}

/// Which chatters must verify when the mode is `Partial`. Ages in minutes; `None` = not a rule.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ModsetPartial")]
pub struct Partial {
    pub first_time: bool,
    pub account_younger_than: Option<u32>,
    pub following_less_than: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ModsetVerifyRule")]
pub struct VerifyRule {
    pub mode: VerifyMode,
    pub partial: Partial,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ModsetVerification")]
pub struct Verification {
    pub email: VerifyRule,
    pub phone: VerifyRule,
    pub exempt_subs: bool,
    pub exempt_vips: bool,
    pub exempt_mods: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ModsetAutoMod")]
pub struct AutoMod {
    /// 0 to 4; `None` = custom (no preset matches).
    pub overall: Option<u8>,
    /// Per category, in `CATEGORIES` order.
    pub levels: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ModsetSettings")]
pub struct ModSettings {
    pub automod: AutoMod,
    /// Seconds before non-mods see messages.
    pub delay: u32,
    pub verification: Verification,
    /// Shown to first-time chatters.
    pub rules: Vec<String>,
    pub unban_requests: bool,
    pub unban_cooldown_minutes: u32,
    pub blocked: Vec<BlockedTerm>,
    pub permitted: Vec<BlockedTerm>,
}

/// One change from the panel.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, rename = "ModsetChange")]
pub enum Change {
    /// A preset, 0 to 4.
    AutoMod {
        level: u8,
    },
    /// One category, the others as they are.
    Category {
        index: u8,
        level: u8,
    },
    Delay {
        seconds: u32,
    },
    Rules {
        rules: Vec<String>,
    },
    Verification {
        verification: Verification,
    },
    UnbanRequests {
        on: bool,
        minutes: u32,
    },
}

// --- reading ---

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPartial {
    #[serde(default)]
    minimum_account_age_in_minutes: u32,
    #[serde(default)]
    minimum_follower_age_in_minutes: u32,
    #[serde(default)]
    should_restrict_based_on_account_age: bool,
    #[serde(default)]
    should_restrict_first_time_chatters: bool,
    #[serde(default)]
    should_restrict_based_on_follower_age: bool,
}

impl From<RawPartial> for Partial {
    fn from(p: RawPartial) -> Self {
        Self {
            first_time: p.should_restrict_first_time_chatters,
            account_younger_than: p
                .should_restrict_based_on_account_age
                .then_some(p.minimum_account_age_in_minutes),
            following_less_than: p
                .should_restrict_based_on_follower_age
                .then_some(p.minimum_follower_age_in_minutes),
        }
    }
}

impl Partial {
    fn input(&self) -> Value {
        json!({
            "shouldRestrictFirstTimeChatters": self.first_time,
            "shouldRestrictBasedOnAccountAge": self.account_younger_than.is_some(),
            "minimumAccountAgeInMinutes": self.account_younger_than.unwrap_or(0),
            "shouldRestrictBasedOnFollowerAge": self.following_less_than.is_some(),
            "minimumFollowerAgeInMinutes": self.following_less_than.unwrap_or(0),
        })
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVerification {
    #[serde(default)]
    email_verification_mode: Option<VerifyMode>,
    #[serde(default)]
    partial_email_verification_config: Option<RawPartial>,
    #[serde(default)]
    phone_verification_mode: Option<VerifyMode>,
    #[serde(default)]
    partial_phone_verification_config: Option<RawPartial>,
    #[serde(default, rename = "isSubscriberExempt")]
    subs: bool,
    #[serde(default, rename = "isVIPExempt")]
    vips: bool,
    #[serde(default, rename = "isModeratorExempt")]
    mods: bool,
}

impl From<RawVerification> for Verification {
    fn from(v: RawVerification) -> Self {
        Self {
            email: VerifyRule {
                mode: v.email_verification_mode.unwrap_or_default(),
                partial: v.partial_email_verification_config.unwrap_or_default().into(),
            },
            phone: VerifyRule {
                mode: v.phone_verification_mode.unwrap_or_default(),
                partial: v.partial_phone_verification_config.unwrap_or_default().into(),
            },
            exempt_subs: v.subs,
            exempt_vips: v.vips,
            exempt_mods: v.mods,
        }
    }
}

impl Verification {
    fn input(&self) -> Value {
        json!({
            "emailVerificationMode": self.email.mode,
            "partialEmailVerificationConfig": self.email.partial.input(),
            "phoneVerificationMode": self.phone.mode,
            "partialPhoneVerificationConfig": self.phone.partial.input(),
            "isSubscriberExempt": self.exempt_subs,
            "isVIPExempt": self.exempt_vips,
            "isModeratorExempt": self.exempt_mods,
        })
    }
}

#[derive(Deserialize)]
struct Data {
    user: Option<User>,
    channel: Option<Channel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    chat_settings: Option<Chat>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Chat {
    chat_delay_ms: Option<u32>,
    #[serde(default)]
    rules: Option<Vec<String>>,
    account_verification_options: Option<RawVerification>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Channel {
    auto_mod_configuration: Option<RawAutoMod>,
    unban_requests_settings: Option<Unban>,
    blocked_terms: Option<Terms>,
    permitted_terms: Option<Terms>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAutoMod {
    overall_level: Option<i32>,
    detailed_levels: Option<serde_json::Map<String, Value>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Unban {
    #[serde(default)]
    is_enabled: bool,
    #[serde(default)]
    cooldown_minutes: u32,
}

#[derive(Deserialize)]
struct Terms {
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct Edge {
    node: Option<RawTerm>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTerm {
    id: String,
    phrase: String,
    #[serde(default)]
    created_at: Option<String>,
    expires_at: Option<String>,
}

fn terms(t: Option<Terms>) -> Vec<BlockedTerm> {
    t.map(|t| t.edges)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| e.node)
        .map(|n| BlockedTerm {
            id: n.id,
            text: n.phrase,
            created_at: n.created_at.unwrap_or_default(),
            expires_at: n.expires_at,
        })
        .collect()
}

/// The preset these levels are, if any.
pub fn preset_of(levels: &[u8]) -> Option<u8> {
    PRESETS.iter().position(|p| p[..] == *levels).map(|i| i as u8)
}

fn automod(a: Option<RawAutoMod>) -> AutoMod {
    let Some(a) = a else {
        return AutoMod {
            overall: Some(0),
            levels: PRESETS[0].to_vec(),
        };
    };
    let d = a.detailed_levels.unwrap_or_default();
    let levels: Vec<u8> = CATEGORIES
        .iter()
        .map(|c| {
            d.get(&format!("{c}Level"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .min(4) as u8
        })
        .collect();
    // Twitch's own number when it has one (it says -1 for custom), else the matching preset.
    let overall = match a.overall_level {
        Some(n @ 0..=4) => Some(n as u8),
        _ => preset_of(&levels),
    };
    AutoMod { overall, levels }
}

fn settings_from(d: Data) -> ModSettings {
    let chat = d.user.and_then(|u| u.chat_settings);
    let (delay, rules, verification) = match chat {
        Some(c) => (
            c.chat_delay_ms.unwrap_or(0) / 1000,
            c.rules
                .unwrap_or_default()
                .into_iter()
                .filter(|r| !r.trim().is_empty())
                .collect(),
            c.account_verification_options.unwrap_or_default().into(),
        ),
        None => (0, Vec::new(), Verification::default()),
    };
    let ch = d.channel;
    let (am, unban, blocked, permitted) = match ch {
        Some(c) => (
            c.auto_mod_configuration,
            c.unban_requests_settings,
            c.blocked_terms,
            c.permitted_terms,
        ),
        None => (None, None, None, None),
    };
    ModSettings {
        automod: automod(am),
        delay,
        verification,
        rules,
        unban_requests: unban.as_ref().is_some_and(|u| u.is_enabled),
        unban_cooldown_minutes: unban.map_or(0, |u| u.cooldown_minutes),
        blocked: terms(blocked),
        permitted: terms(permitted),
    }
}

/// Everything on the page. Terms: the first 100 of each, as Twitch pages them.
pub async fn read(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<ModSettings> {
    let d: Data = query(http, auth, READ, json!({ "id": channel_id })).await?;
    Ok(settings_from(d))
}

// --- writing ---

/// Chat rules as Twitch keeps them: trimmed, no blanks, each ≤ 150 characters, up to 15.
pub const MAX_RULES: usize = 15;
pub const MAX_RULE_LEN: usize = 150;

pub fn clean_rules(rules: &[String]) -> std::result::Result<Vec<String>, String> {
    let out: Vec<String> = rules
        .iter()
        .map(|r| r.trim().to_owned())
        .filter(|r| !r.is_empty())
        .collect();
    if out.len() > MAX_RULES {
        return Err(format!("Up to {MAX_RULES} rules."));
    }
    if out.iter().any(|r| r.chars().count() > MAX_RULE_LEN) {
        return Err(format!("A rule can have up to {MAX_RULE_LEN} characters."));
    }
    Ok(out)
}

/// The mutation and its input for one change. `current` gives the other AutoMod levels.
pub fn plan(
    channel_id: &str,
    change: &Change,
    current: &[u8],
) -> std::result::Result<(&'static str, Value), String> {
    let chat = |k: &str, v: Value| {
        let mut m = serde_json::Map::new();
        m.insert("channelID".into(), channel_id.into());
        m.insert(k.into(), v);
        (CHAT, Value::Object(m))
    };
    let levels = |l: &[u8]| -> Value {
        let mut m = serde_json::Map::new();
        for (c, v) in CATEGORIES.iter().zip(l) {
            m.insert(format!("{c}Level"), (*v).into());
        }
        Value::Object(m)
    };
    Ok(match change {
        Change::AutoMod { level } => {
            let p = PRESETS
                .get(usize::from(*level))
                .ok_or("AutoMod goes from 0 to 4.")?;
            (
                AUTOMOD,
                json!({ "channelID": channel_id, "overallLevel": level, "detailedLevels": levels(p) }),
            )
        }
        Change::Category { index, level } => {
            let i = usize::from(*index);
            if i >= CATEGORIES.len() || current.len() != CATEGORIES.len() {
                return Err("Not an AutoMod category.".into());
            }
            let mut l = current.to_vec();
            l[i] = (*level).min(4);
            // A preset again sends its number; custom sends none, as Twitch's dashboard does.
            let mut input = json!({ "channelID": channel_id, "detailedLevels": levels(&l) });
            if let Some(p) = preset_of(&l) {
                input["overallLevel"] = p.into();
            }
            (AUTOMOD, input)
        }
        Change::Delay { seconds } => {
            let s = DELAYS
                .into_iter()
                .min_by_key(|d| d.abs_diff(*seconds))
                .unwrap_or(0);
            chat("chatDelayInMs", (s * 1000).into())
        }
        Change::Rules { rules } => {
            let mut r = clean_rules(rules)?;
            // Twitch ignores an empty list (checked live 2026/10/21): one blank rule is how
            // "no rules" gets there; reading drops blank rules.
            if r.is_empty() {
                r.push(String::new());
            }
            chat("rules", r.into())
        }
        Change::Verification { verification } => chat("accountVerificationOptions", verification.input()),
        Change::UnbanRequests { on, minutes } => (
            UNBAN,
            json!({ "channelID": channel_id, "isEnabled": on, "cooldownMinutes": minutes }),
        ),
    })
}

/// Applies one change and reads everything back.
pub async fn set(http: &Http, auth: Auth<'_>, channel_id: &str, change: &Change) -> Result<ModSettings> {
    let now = read(http, auth, channel_id).await?;
    let (text, input) = plan(channel_id, change, &now.automod.levels).map_err(Error::Refused)?;
    match text {
        AUTOMOD => {
            let _: Value = query(http, auth, text, json!({ "input": input })).await?;
        }
        CHAT => mutate(http, auth, text, "updateChatSettings", input).await?,
        _ => mutate(http, auth, text, "setUnbanRequestsSettings", input).await?,
    }
    read(http, auth, channel_id).await
}

// --- terms ---

const ADD_PERMITTED: &str = "mutation($input: AddChannelPermittedTermInput!) { \
    addChannelPermittedTerm(input: $input) { term { id } error } }";
const ADD_BLOCKED: &str = "mutation($input: AddChannelBlockedTermInput!) { \
    addChannelBlockedTerm(input: $input) { term { id } error } }";
const DEL_PERMITTED: &str = "mutation($input: DeleteChannelPermittedTermByIDInput!) { \
    deleteChannelPermittedTermByID(input: $input) { error { code } } }";
const DEL_BLOCKED: &str = "mutation($input: DeleteChannelBlockedTermByIDInput!) { \
    deleteChannelBlockedTermByID(input: $input) { error { code } } }";

/// Adds a term for good. Blocked terms stay editable by mods, like Twitch's default.
pub async fn add_term(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    permitted: bool,
    text: &str,
) -> Result<()> {
    #[derive(Deserialize)]
    struct Payload {
        /// An enum here, not `{ code }`.
        error: Option<String>,
    }
    let (q, field, input) = if permitted {
        (
            ADD_PERMITTED,
            "addChannelPermittedTerm",
            json!({ "channelID": channel_id, "phrase": text }),
        )
    } else {
        (
            ADD_BLOCKED,
            "addChannelBlockedTerm",
            json!({ "channelID": channel_id, "phrase": text, "isModEditable": true }),
        )
    };
    let data: Value = query(http, auth, q, json!({ "input": input })).await?;
    let p: Option<Payload> =
        serde_json::from_value(data.get(field).cloned().unwrap_or(Value::Null)).map_err(Error::Decode)?;
    match p.and_then(|p| p.error) {
        Some(code) => Err(Error::Refused(code)),
        None => Ok(()),
    }
}

pub async fn remove_term(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    permitted: bool,
    id: &str,
) -> Result<()> {
    let (q, field) = if permitted {
        (DEL_PERMITTED, "deleteChannelPermittedTermByID")
    } else {
        (DEL_BLOCKED, "deleteChannelBlockedTermByID")
    };
    mutate(http, auth, q, field, json!({ "channelID": channel_id, "id": id })).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIVE: &str = r#"{"user":{"chatSettings":{"chatDelayMs":4000,"rules":["Be nice",""],
      "accountVerificationOptions":{"emailVerificationMode":"ALL","isModeratorExempt":false,
      "isSubscriberExempt":true,"isVIPExempt":false,"partialEmailVerificationConfig":{"minimumAccountAgeInMinutes":0,
      "minimumFollowerAgeInMinutes":0,"shouldRestrictBasedOnAccountAge":false,"shouldRestrictBasedOnFollowerAge":false,
      "shouldRestrictFirstTimeChatters":false},"partialPhoneVerificationConfig":{"minimumAccountAgeInMinutes":1440,
      "minimumFollowerAgeInMinutes":10,"shouldRestrictBasedOnAccountAge":true,"shouldRestrictBasedOnFollowerAge":false,
      "shouldRestrictFirstTimeChatters":true},"phoneVerificationMode":"PARTIAL"}}},
      "channel":{"autoModConfiguration":{"overallLevel":2,"detailedLevels":{"ableismLevel":2,"aggressionLevel":2,
      "homophobiaLevel":2,"misogynyLevel":2,"nameCallingLevel":1,"profanityLevel":0,"racismLevel":2,
      "sexualHarassmentLevel":2,"sexualityLevel":2,"smartDetectionLevel":2}},
      "unbanRequestsSettings":{"cooldownMinutes":15,"isEnabled":true},
      "blockedTerms":{"edges":[{"node":{"createdAt":"2025-12-05T12:55:12Z","expiresAt":null,
      "id":"85eb","phrase":"forsen"}}]},"permittedTerms":{"edges":[]}}}"#;

    #[test]
    fn reads_the_live_shape() {
        let s = settings_from(serde_json::from_str(LIVE).unwrap());
        assert_eq!(s.automod.overall, Some(2));
        assert_eq!(s.automod.levels, PRESETS[2].to_vec());
        assert_eq!(s.delay, 4);
        assert_eq!(s.rules, vec!["Be nice"]);
        assert!(s.unban_requests);
        assert_eq!(s.unban_cooldown_minutes, 15);
        assert_eq!(s.blocked[0].text, "forsen");
        assert!(s.permitted.is_empty());
        let v = &s.verification;
        assert_eq!(
            (v.email.mode, v.phone.mode),
            (VerifyMode::All, VerifyMode::Partial)
        );
        assert_eq!(
            v.phone.partial,
            Partial {
                first_time: true,
                account_younger_than: Some(1440),
                following_less_than: None
            }
        );
        assert!(v.exempt_subs && !v.exempt_mods);
    }

    #[test]
    fn empty_reads_as_off() {
        let s = settings_from(serde_json::from_str(r#"{"user":null,"channel":null}"#).unwrap());
        assert_eq!(s.automod.overall, Some(0));
        assert_eq!(s.delay, 0);
        assert_eq!(s.verification.email.mode, VerifyMode::None);
    }

    #[test]
    fn custom_levels_have_no_preset() {
        let a = automod(Some(RawAutoMod {
            overall_level: Some(-1),
            detailed_levels: serde_json::from_str(r#"{"ableismLevel":4}"#).unwrap(),
        }));
        assert_eq!(a.overall, None);
        assert_eq!(a.levels[0], 4);
    }

    #[test]
    fn plans_match_twitchs_inputs() {
        let cur = PRESETS[2];
        let (q, i) = plan("7", &Change::AutoMod { level: 3 }, &cur).unwrap();
        assert_eq!(q, AUTOMOD);
        assert_eq!(i["overallLevel"], 3);
        assert_eq!(i["detailedLevels"]["smartDetectionLevel"], 4);
        assert!(plan("7", &Change::AutoMod { level: 5 }, &cur).is_err());

        let (_, i) = plan("7", &Change::Category { index: 5, level: 3 }, &cur).unwrap();
        assert!(i.get("overallLevel").is_none());
        assert_eq!(i["detailedLevels"]["profanityLevel"], 3);
        // Back to the preset: its number again.
        let mut l = cur;
        l[5] = 3;
        let (_, i) = plan("7", &Change::Category { index: 5, level: 0 }, &l).unwrap();
        assert_eq!(i["overallLevel"], 2);

        let (q, i) = plan("7", &Change::Delay { seconds: 5 }, &cur).unwrap();
        assert_eq!((q, i["chatDelayInMs"].as_u64()), (CHAT, Some(4000)));
        assert_eq!(i["channelID"], "7");

        let rules = vec!["  a ".to_owned(), String::new()];
        let (_, i) = plan("7", &Change::Rules { rules }, &cur).unwrap();
        assert_eq!(i["rules"], json!(["a"]));
        let (_, i) = plan("7", &Change::Rules { rules: vec![] }, &cur).unwrap();
        assert_eq!(i["rules"], json!([""]));

        let mut v = Verification::default();
        v.phone.mode = VerifyMode::Partial;
        v.phone.partial.following_less_than = Some(10);
        let (_, i) = plan("7", &Change::Verification { verification: v }, &cur).unwrap();
        let o = &i["accountVerificationOptions"];
        assert_eq!(o["phoneVerificationMode"], "PARTIAL");
        assert_eq!(
            o["partialPhoneVerificationConfig"]["shouldRestrictBasedOnFollowerAge"],
            true
        );
        assert_eq!(
            o["partialPhoneVerificationConfig"]["minimumFollowerAgeInMinutes"],
            10
        );
        assert_eq!(
            o["partialEmailVerificationConfig"]["shouldRestrictBasedOnAccountAge"],
            false
        );

        let (q, i) = plan(
            "7",
            &Change::UnbanRequests {
                on: false,
                minutes: 60,
            },
            &cur,
        )
        .unwrap();
        assert_eq!(
            (q, &i["isEnabled"], &i["cooldownMinutes"]),
            (UNBAN, &json!(false), &json!(60))
        );
    }

    #[test]
    fn rules_are_checked() {
        assert!(clean_rules(&vec!["x".to_owned(); 16]).unwrap_err().contains("15"));
        assert!(clean_rules(&["y".repeat(151)]).unwrap_err().contains("150"));
        assert_eq!(clean_rules(&["y".repeat(150)]).unwrap().len(), 1);
    }

    #[test]
    fn presets_match_twitchs_table() {
        assert_eq!(preset_of(&PRESETS[4]), Some(4));
        assert_eq!(preset_of(&[1; 10]), None);
    }
}
