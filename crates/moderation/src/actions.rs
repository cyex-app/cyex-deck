//! V103-15 Mod actions as lines for the "Mod actions" panels of the mod view (1.48) and the
//! stream manager (1.35): who did what to whom, for how long and why. They come from
//! EventSub's `channel.moderate` (a Helix sign-in), from Hermes' `chat_moderator_actions`
//! (a GQL sign-in, the old PubSub shapes), from AutoMod's decisions, and from Twitch's
//! history (`moderationActionLogs`). Everything here is plain parsing, so it's tested.

use std::collections::VecDeque;

use cyex_eventsub::events::{Moderate, Who};
use cyex_twitch::gql::modlogs::ChannelAction;
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

/// Which filter chip a line belongs to (`ModActionFilters` in the settings).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ModGroup {
    Ban,
    Timeout,
    Warn,
    /// Deleted messages and cleared chats.
    Delete,
    /// Chat modes and shield mode.
    Mode,
    /// Mods and VIPs.
    Role,
    /// AutoMod decisions, blocked and allowed terms, AutoMod settings.
    Automod,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, rename = "ModPerson")]
pub struct Person {
    pub login: String,
    pub name: String,
}

impl Person {
    fn of(w: &Who) -> Option<Self> {
        let login = if w.login.is_empty() {
            w.name.to_ascii_lowercase()
        } else {
            w.login.clone()
        };
        if login.is_empty() {
            return None;
        }
        let name = if w.name.is_empty() {
            login.clone()
        } else {
            w.name.clone()
        };
        Some(Self { login, name })
    }
}

/// One mod action, newest first in a board.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModLine {
    pub id: String,
    /// Unix milliseconds.
    #[ts(type = "number")]
    pub ts: i64,
    pub group: ModGroup,
    /// What was done, short and lower case: "banned", "turned on slow mode".
    pub action: String,
    pub moderator: Option<Person>,
    pub target: Option<Person>,
    /// A timeout's length, slow mode's wait, followers-only's follow age.
    #[ts(type = "number | null")]
    pub secs: Option<u64>,
    /// Empty when none was given.
    pub reason: String,
    /// What it was about, if not a person: a deleted message, terms, a raid's size.
    pub about: String,
    /// Another channel's action in a shared chat.
    pub from: Option<String>,
    /// From Twitch's history, not seen live.
    pub past: bool,
}

/// `channel.moderate`'s action names (PubSub's are mapped onto them) → group and wording.
fn word(action: &str, on: bool) -> (ModGroup, String) {
    use ModGroup::*;
    let toggled = |what: &str| format!("turned {} {what}", if on { "on" } else { "off" });
    let (g, w) = match action {
        "ban" => (Ban, "banned"),
        "unban" => (Ban, "unbanned"),
        "timeout" => (Timeout, "timed out"),
        "untimeout" => (Timeout, "lifted the timeout on"),
        "warn" => (Warn, "warned"),
        "delete" => (Delete, "deleted a message from"),
        "clear" => (Delete, "cleared the chat"),
        "emoteonly" | "emoteonlyoff" => return (Mode, toggled("emote-only mode")),
        "subscribers" | "subscribersoff" => return (Mode, toggled("subscribers-only mode")),
        "uniquechat" | "uniquechatoff" => return (Mode, toggled("unique-chat mode")),
        "followers" | "followersoff" => return (Mode, toggled("followers-only mode")),
        "slow" | "slowoff" => return (Mode, toggled("slow mode")),
        "shield" | "shieldoff" => return (Mode, toggled("shield mode")),
        "vip" => (Role, "gave VIP to"),
        "unvip" => (Role, "took VIP from"),
        "mod" => (Role, "modded"),
        "unmod" => (Role, "unmodded"),
        // V107-R44 As the chat line says it ("… added “x” as a blocked term.").
        "add_blocked_term" => (Automod, "added the blocked term"),
        "remove_blocked_term" => (Automod, "removed the blocked term"),
        "add_permitted_term" => (Automod, "allowed"),
        "remove_permitted_term" => (Automod, "stopped allowing"),
        "approved_automod_message" => (Automod, "approved a held message from"),
        "denied_automod_message" => (Automod, "denied a held message from"),
        "approve_unban_request" => (Other, "approved the unban request of"),
        "deny_unban_request" => (Other, "denied the unban request of"),
        "raid" => (Other, "raided"),
        "unraid" => (Other, "canceled the raid"),
        other => return (Other, other.replace('_', " ")),
    };
    (g, w.to_owned())
}

/// V7 Twitch's "Added as a Moderator" in the live feed's words, which read before the name.
fn role_words(done: &str) -> Option<&'static str> {
    let d = done.to_ascii_lowercase();
    let added = d.starts_with("added");
    if !added && !d.starts_with("removed") {
        return None;
    }
    // Twitch also words it as Added as "Mod", with straight quotes around the role.
    let word = |w: &str| d.split(|c: char| !c.is_alphanumeric()).any(|x| x == w);
    if word("moderator") || word("mod") {
        Some(if added { "modded" } else { "unmodded" })
    } else if word("vip") {
        Some(if added { "gave VIP to" } else { "took VIP from" })
    } else {
        None
    }
}

/// Twitch's history nouns ("Message Deleted") read badly before a name; the live feed's verbs instead.
fn person_words(done: &str) -> Option<&'static str> {
    match done.trim().to_ascii_lowercase().as_str() {
        "message deleted" => Some("deleted a message from"),
        "timeout removed" => Some("lifted the timeout on"),
        _ => None,
    }
}

/// V107-N3d Twitch's history words for a term ("Added as Blocked Term", the term in the
/// title) → the live feed's, so both read alike and filter under AutoMod.
fn term_words(done: &str) -> Option<&'static str> {
    match done.trim().to_ascii_lowercase().as_str() {
        "added as blocked term" => Some("added the blocked term"),
        "removed as blocked term" => Some("removed the blocked term"),
        "added as permitted term" => Some("allowed"),
        "removed as permitted term" => Some("stopped allowing"),
        _ => None,
    }
}

impl ModLine {
    /// A live action from EventSub or Hermes. `secs` is a timeout's length.
    pub fn live(m: &Moderate, secs: Option<u64>, id: String, ts: i64) -> Self {
        let action = m.action.strip_prefix("shared_chat_").unwrap_or(&m.action);
        let on = !action.ends_with("off");
        let (group, words) = word(action, on);
        let secs = match action {
            "timeout" => secs,
            "slow" => Some(m.amount.unwrap_or(30)),
            "followers" => m.amount.filter(|min| *min > 0).map(|min| min * 60),
            _ => None,
        };
        let about = match action {
            "delete" => m.message_body.clone().unwrap_or_default(),
            // V107-R44 The mod log quotes it, once.
            a if a.ends_with("_term") => m.terms.join(", "),
            "raid" => m
                .amount
                .map(|n| format!("{} viewers", cyex_core::number::grouped(n)))
                .unwrap_or_default(),
            _ => String::new(),
        };
        Self {
            id,
            ts,
            group,
            action: words,
            moderator: Person::of(&m.moderator),
            target: m.target.as_ref().and_then(Person::of),
            secs,
            reason: m.reason.trim().to_owned(),
            about,
            from: m
                .source_channel
                .clone()
                .filter(|_| m.action.starts_with("shared_chat_")),
            past: false,
        }
    }

    /// 1.56 A held message approved or denied by a mod.
    pub fn decision(d: &Decision, id: String, ts: i64) -> Self {
        let (group, action) = word(
            if d.approved {
                "approved_automod_message"
            } else {
                "denied_automod_message"
            },
            true,
        );
        Self {
            id,
            ts,
            group,
            action,
            moderator: d.moderator.clone(),
            target: d.sender.clone(),
            secs: None,
            reason: String::new(),
            about: d.text.clone(),
            from: None,
            past: false,
        }
    }

    /// Shield mode turned on or off (`channel.shield_mode.begin/end`).
    pub fn shield(on: bool, moderator: Option<&Who>, id: String, ts: i64) -> Self {
        let (group, action) = word(if on { "shield" } else { "shieldoff" }, on);
        Self {
            id,
            ts,
            group,
            action,
            moderator: moderator.and_then(Person::of),
            target: None,
            secs: None,
            reason: String::new(),
            about: String::new(),
            from: None,
            past: false,
        }
    }

    /// A line of Twitch's history, in Twitch's own words. `ts` is its `createdAt`.
    pub fn past(a: &ChannelAction, ts: i64) -> Self {
        use ModGroup::*;
        let role = a.target.is_some().then(|| role_words(&a.done)).flatten();
        let term = a.target.is_none().then(|| term_words(&a.done)).flatten();
        let group = match (a.category.as_str(), a.icon.as_str()) {
            _ if role.is_some() => Role,
            _ if term.is_some() => Automod,
            // V107-N3g The non-mod chat delay is a chat setting, not a role.
            _ if a.title.to_ascii_lowercase().contains("chat delay") => Mode,
            (_, "TRASH" | "HALT") => Delete,
            ("BANS_AND_UNBANS", _) | (_, "BAN") => Ban,
            ("TIMEOUTS_AND_UNTIMEOUTS", _) | (_, "TIMEOUT") => Timeout,
            ("WARNINGS", _) | (_, "WARNING") => Warn,
            ("MODS_AND_VIPS", _) | (_, "SWORD" | "VIP" | "PERMISSION_ADDED" | "PERMISSION_REMOVED") => Role,
            (c, i) if c.starts_with("AUTOMOD") || i == "AUTOMOD" || i == "CHECK" => Automod,
            ("CHAT_MODE_CHANGES", _) | (_, "SHIELD_MODE") => Mode,
            _ => Other,
        };
        // Twitch's Title Case in lower case ("Clear Chat" → "clear chat"); "AutoMod" and
        // "VIP" keep their capitals.
        let lower = |s: &str| {
            s.split(' ')
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        Some(f) if f.is_uppercase() && c.clone().all(|x| !x.is_uppercase()) => {
                            f.to_lowercase().chain(c).collect()
                        }
                        _ => w.to_owned(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        let action = if let Some(words) = role.or(term) {
            words.to_owned()
        } else if let Some(words) = a.target.is_some().then(|| person_words(&a.done)).flatten() {
            words.to_owned()
        } else if a.target.is_some() || a.title.is_empty() {
            lower(&a.done)
        } else if a.done.is_empty() || a.done == "Done" {
            lower(&a.title)
        } else {
            // V107-N3g Twitch ends some with a comma ("Enabled with 2s delay,").
            format!("{}: {}", lower(a.done.trim_end_matches([',', ' '])), a.title)
        };
        // The body is a reason only for actions on a person; else it's Twitch's note.
        let reason = if !a.reason.is_empty() {
            a.reason.clone()
        } else if matches!(group, Ban | Timeout | Warn) {
            a.note.clone()
        } else {
            String::new()
        };
        let person = |p: &Option<(String, String)>| {
            p.as_ref().map(|(login, name)| Person {
                login: login.clone(),
                name: name.clone(),
            })
        };
        Self {
            id: a.id.clone(),
            ts,
            group,
            action,
            moderator: person(&a.moderator),
            target: person(&a.target),
            secs: None,
            reason,
            about: if term.is_some() { a.title.trim_matches(['"', '“', '”', ' ']).to_owned() } else { String::new() },
            from: None,
            past: true,
        }
    }

    /// The same action seen twice: by two accounts' feeds, or live and in the history.
    fn same(&self, o: &ModLine) -> bool {
        let login = |p: &Option<Person>| p.as_ref().map(|p| p.login.clone());
        self.group == o.group
            && login(&self.target) == login(&o.target)
            && login(&self.moderator) == login(&o.moderator)
            && (self.ts - o.ts).abs() <= SAME_MS
            && (self.past != o.past || self.action == o.action)
    }
}

/// How far apart the same action can be timed by two sources.
const SAME_MS: i64 = 10_000;

/// Adds a live line at the front unless it's already there; keeps at most `cap`.
pub fn push(lines: &mut VecDeque<ModLine>, line: ModLine, cap: usize) -> bool {
    if lines.iter().take(20).any(|l| l.same(&line)) {
        return false;
    }
    lines.push_front(line);
    lines.truncate(cap);
    true
}

/// Adds a page of history: lines already seen (by id, or live) are skipped, the rest go
/// in by time. Keeps at most `cap`, the newest.
pub fn merge(lines: &mut VecDeque<ModLine>, page: Vec<ModLine>, cap: usize) {
    for line in page {
        if lines.iter().any(|l| l.id == line.id || l.same(&line)) {
            continue;
        }
        let at = lines.iter().position(|l| l.ts < line.ts).unwrap_or(lines.len());
        lines.insert(at, line);
    }
    lines.truncate(cap);
}

/// 1.56 A held message a mod approved or denied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub approved: bool,
    pub moderator: Option<Person>,
    pub sender: Option<Person>,
    pub text: String,
}

impl Decision {
    /// EventSub's `automod.message.update`: `Approved`, `Denied` or `Expired` (no decision).
    pub fn from_status(
        status: &str,
        moderator: Option<&Who>,
        sender: Option<Person>,
        text: String,
    ) -> Option<Self> {
        let approved = match status.to_ascii_lowercase().as_str() {
            "approved" | "allowed" => true,
            "denied" => false,
            _ => return None,
        };
        Some(Self {
            approved,
            moderator: moderator.and_then(Person::of),
            sender,
            text,
        })
    }

    pub fn sender(w: &Who) -> Option<Person> {
        Person::of(w)
    }
}

/// Hermes' `automod-queue.<user>.<channel>`: a held message comes again once a mod decided
/// on it, with `ALLOWED` or `DENIED` and who did it.
pub fn automod_queue(p: &Value) -> Option<Decision> {
    if p.get("type")?.as_str()? != "automod_caught_message" {
        return None;
    }
    let d = p.get("data")?;
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
    let msg = d.get("message").unwrap_or(&Value::Null);
    let sender = msg.get("sender").map(|u| Who {
        id: s(u, "user_id"),
        login: s(u, "login"),
        name: s(u, "display_name"),
    });
    let text = msg
        .pointer("/content/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let resolver = Who {
        id: s(d, "resolver_id"),
        login: s(d, "resolver_login"),
        name: s(d, "resolver_login"),
    };
    Decision::from_status(
        &s(d, "status"),
        Some(&resolver),
        sender.as_ref().and_then(Person::of),
        text,
    )
}

/// V106-07 Q2b Hermes' `automod-queue`: a message AutoMod is holding (`status` PENDING). Shape
/// as Chatterino 2.4.6 read it (pubsubmessages/AutoMod.cpp); checked once on a real message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub message_id: String,
    pub sender: Who,
    pub text: String,
    pub reason: String,
}

pub fn automod_pending(p: &Value) -> Option<Pending> {
    if p.get("type")?.as_str()? != "automod_caught_message" {
        return None;
    }
    let d = p.get("data")?;
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
    if !s(d, "status").eq_ignore_ascii_case("pending") {
        return None;
    }
    let msg = d.get("message")?;
    let message_id = s(msg, "id");
    if message_id.is_empty() {
        return None;
    }
    let u = msg.get("sender").unwrap_or(&Value::Null);
    let sender = Who { id: s(u, "user_id"), login: s(u, "login"), name: s(u, "display_name") };
    let text = msg.pointer("/content/text").and_then(Value::as_str).unwrap_or_default().to_owned();
    let reason = d
        .pointer("/content_classification/category")
        .and_then(Value::as_str)
        .filter(|c| !c.is_empty())
        .unwrap_or("AutoMod")
        .to_owned();
    Some(Pending { message_id, sender, text, reason })
}

/// The same topic once a mod decided or Twitch let it go: which message, the status (lowercase) and who.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub message_id: String,
    pub status: String,
    pub by: Option<String>,
}

pub fn automod_resolved(p: &Value) -> Option<Resolved> {
    if p.get("type")?.as_str()? != "automod_caught_message" {
        return None;
    }
    let d = p.get("data")?;
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
    // A7
    let status = s(d, "status").to_ascii_lowercase();
    if !matches!(status.as_str(), "allowed" | "approved" | "denied" | "expired") {
        return None;
    }
    let message_id = d.get("message").map(|m| s(m, "id")).unwrap_or_default();
    if message_id.is_empty() {
        return None;
    }
    let by = Some(s(d, "resolver_login")).filter(|l| !l.is_empty());
    Some(Resolved { message_id, status, by })
}

/// Hermes' `chat_moderator_actions.<user>.<channel>` (PubSub's shapes, from Twitch's own
/// client) as EventSub's `channel.moderate`, with a timeout's length in seconds. `None` for
/// what isn't a mod action (a warning read, a note to the person a message was deleted from).
pub fn pubsub(p: &Value) -> Option<(Moderate, Option<u64>)> {
    let kind = p.get("type")?.as_str()?;
    let d = p.get("data")?;
    let s = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
    let who = |id: &str, login: &str| Who {
        id: s(id),
        login: s(login).to_ascii_lowercase(),
        name: s(login),
    };
    let target = || Some(who("target_user_id", "target_user_login")).filter(|w| !w.login.is_empty());
    let base = |action: &str, moderator: Who| Moderate {
        moderator,
        action: action.to_owned(),
        target: target(),
        reason: String::new(),
        expires_at: None,
        message_id: None,
        message_body: None,
        amount: None,
        terms: Vec::new(),
        source_channel: None,
    };
    let m = match kind {
        "moderation_action" => {
            if s("type") == "chat_targeted_login_moderation" {
                return None;
            }
            let args: Vec<String> = d
                .get("args")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
                .unwrap_or_default();
            let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
            let raw = s("moderation_action");
            let action = match raw.as_str() {
                "r9kbeta" => "uniquechat",
                "r9kbetaoff" => "uniquechatoff",
                "delete_blocked_term" => "remove_blocked_term",
                "delete_permitted_term" => "remove_permitted_term",
                "shield_mode_enabled" => "shield",
                "shield_mode_disabled" => "shieldoff",
                "acknowledge_warning" | "delete_notification" | "host" | "unhost" | "" => return None,
                a => a,
            };
            let mut m = base(action, who("created_by_user_id", "created_by"));
            // Actions on one person name them first in `args`.
            if m.target.is_none()
                && d.get("type").and_then(Value::as_str) == Some("chat_login_moderation")
                && !arg(0).is_empty()
            {
                m.target = Some(Who {
                    id: String::new(),
                    login: arg(0).to_ascii_lowercase(),
                    name: arg(0),
                });
            }
            let mut secs = None;
            match action {
                "ban" => m.reason = args.get(1..).unwrap_or_default().join(" "),
                "timeout" => {
                    secs = arg(1).parse().ok();
                    m.reason = args.get(2..).unwrap_or_default().join(" ");
                }
                "warn" => m.reason = args.get(1..).unwrap_or_default().join(", "),
                "delete" => {
                    m.message_body = Some(arg(1)).filter(|b| !b.is_empty());
                    m.message_id = Some(arg(2)).filter(|b| !b.is_empty());
                }
                "slow" | "followers" => m.amount = arg(0).parse().ok(),
                a if a.ends_with("_term") => {
                    m.terms = vec![args.join(" ")]
                        .into_iter()
                        .filter(|t| !t.is_empty())
                        .collect()
                }
                "raid" if m.target.is_none() && !arg(0).is_empty() => {
                    m.target = Some(Who {
                        id: String::new(),
                        login: arg(0).to_ascii_lowercase(),
                        name: arg(0),
                    })
                }
                _ => {}
            }
            return Some((m, secs));
        }
        "moderator_added" => base("mod", who("created_by_user_id", "created_by")),
        "moderator_removed" => base("unmod", who("created_by_user_id", "created_by")),
        "vip_added" => base("vip", who("created_by_user_id", "created_by")),
        "vip_removed" => base("unvip", who("created_by_user_id", "created_by")),
        "approve_unban_request" | "deny_unban_request" => {
            let mut m = base(kind, who("created_by_id", "created_by_login"));
            m.reason = s("moderator_message");
            m
        }
        "channel_terms_action" => {
            let action = match s("type").as_str() {
                "add_blocked_term" => "add_blocked_term",
                "delete_blocked_term" => "remove_blocked_term",
                "add_permitted_term" => "add_permitted_term",
                "delete_permitted_term" => "remove_permitted_term",
                _ => return None,
            };
            let mut m = base(action, who("requester_id", "requester_login"));
            m.terms = vec![s("text")].into_iter().filter(|t| !t.is_empty()).collect();
            m.target = None;
            m
        }
        _ => return None,
    };
    Some((m, None))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn who(login: &str) -> Who {
        Who {
            id: "1".into(),
            login: login.into(),
            name: login.to_uppercase(),
        }
    }

    fn moderate(action: &str) -> Moderate {
        Moderate {
            moderator: who("kuni"),
            action: action.into(),
            target: Some(who("x")),
            reason: String::new(),
            expires_at: None,
            message_id: None,
            message_body: None,
            amount: None,
            terms: Vec::new(),
            source_channel: None,
        }
    }

    #[test]
    fn live_lines_say_who_did_what_to_whom() {
        let mut t = moderate("timeout");
        t.reason = " caps ".into();
        let l = ModLine::live(&t, Some(600), "a".into(), 5);
        assert_eq!(
            (l.group, l.action.as_str(), l.secs, l.reason.as_str()),
            (ModGroup::Timeout, "timed out", Some(600), "caps")
        );
        assert_eq!(l.moderator.as_ref().map(|p| p.name.as_str()), Some("KUNI"));
        assert_eq!(l.target.as_ref().map(|p| p.login.as_str()), Some("x"));
        let mut slow = moderate("slow");
        slow.target = None;
        assert_eq!(ModLine::live(&slow, None, "b".into(), 5).secs, Some(30));
        let off = ModLine::live(&moderate("followersoff"), None, "c".into(), 5);
        assert_eq!(
            (off.group, off.action.as_str()),
            (ModGroup::Mode, "turned off followers-only mode")
        );
        let mut d = moderate("delete");
        d.message_body = Some("bad".into());
        assert_eq!(ModLine::live(&d, None, "d".into(), 5).about, "bad");
        let mut t = d.clone();
        t.action = "add_blocked_term".into();
        t.terms = vec!["foo".into(), "bar".into()];
        let l = ModLine::live(&t, None, "t".into(), 5);
        assert_eq!((l.action.as_str(), l.about.as_str()), ("added the blocked term", "foo, bar"), "V107-R44");
        let mut shared = moderate("shared_chat_ban");
        shared.source_channel = Some("other".into());
        let s = ModLine::live(&shared, None, "e".into(), 5);
        assert_eq!((s.group, s.from.as_deref()), (ModGroup::Ban, Some("other")));
        assert_eq!(
            ModLine::live(&moderate("vip"), None, "f".into(), 5).group,
            ModGroup::Role
        );
        assert_eq!(
            ModLine::live(&moderate("something_new"), None, "g".into(), 5).action,
            "something new"
        );
    }

    #[test]
    fn pubsub_shapes_read_as_eventsub_ones() {
        // Twitch's own client's shapes (v1.03 batch 2a research); not seen live.
        let p = json!({"type": "moderation_action", "data": {"type": "chat_login_moderation", "moderation_action": "timeout",
            "args": ["X", "600", "caps", "lock"], "created_by": "kuni", "created_by_user_id": "2",
            "target_user_id": "3", "target_user_login": "x", "msg_id": ""}});
        let (m, secs) = pubsub(&p).unwrap();
        assert_eq!(
            (m.action.as_str(), secs, m.reason.as_str()),
            ("timeout", Some(600), "caps lock")
        );
        assert_eq!(
            (
                m.moderator.login.as_str(),
                m.target.as_ref().map(|t| t.id.as_str())
            ),
            ("kuni", Some("3"))
        );
        let ban = json!({"type": "moderation_action", "data": {"type": "chat_login_moderation", "moderation_action": "ban",
            "args": ["Y"], "created_by": "kuni", "created_by_user_id": "2"}});
        let (m, _) = pubsub(&ban).unwrap();
        assert_eq!(
            (m.reason.as_str(), m.target.map(|t| t.login)),
            ("", Some("y".into()))
        );
        let del = json!({"type": "moderation_action", "data": {"type": "chat_login_moderation", "moderation_action": "delete",
            "args": ["x", "hello there", "m1"], "created_by": "kuni"}});
        assert_eq!(
            pubsub(&del).unwrap().0.message_body.as_deref(),
            Some("hello there")
        );
        let slow = json!({"type": "moderation_action", "data": {"type": "chat_channel_moderation", "moderation_action": "followers",
            "args": ["10"], "created_by": "kuni"}});
        let (m, _) = pubsub(&slow).unwrap();
        assert_eq!((m.amount, m.target.is_none()), (Some(10), true));
        assert_eq!(ModLine::live(&m, None, "a".into(), 1).secs, Some(600));
        let r9k = json!({"type": "moderation_action", "data": {"type": "chat_channel_moderation", "moderation_action": "r9kbetaoff", "args": null, "created_by": "kuni"}});
        assert_eq!(pubsub(&r9k).unwrap().0.action, "uniquechatoff");
        let modded = json!({"type": "moderator_added", "data": {"channel_id": "1", "moderation_action": "mod",
            "target_user_id": "3", "target_user_login": "x", "created_by": "kuni", "created_by_user_id": "2"}});
        assert_eq!(pubsub(&modded).unwrap().0.action, "mod");
        let unban = json!({"type": "approve_unban_request", "data": {"created_by_id": "2", "created_by_login": "kuni",
            "moderator_message": "ok", "target_user_id": "3", "target_user_login": "x"}});
        let (m, _) = pubsub(&unban).unwrap();
        assert_eq!(
            (m.action.as_str(), m.reason.as_str(), m.moderator.login.as_str()),
            ("approve_unban_request", "ok", "kuni")
        );
        let term = json!({"type": "channel_terms_action", "data": {"type": "delete_blocked_term", "text": "bad word",
            "requester_id": "2", "requester_login": "kuni", "from_automod": false}});
        let (m, _) = pubsub(&term).unwrap();
        assert_eq!(
            (m.action.as_str(), m.terms.clone()),
            ("remove_blocked_term", vec!["bad word".to_owned()])
        );
        // Not mod actions.
        let ack = json!({"type": "moderation_action", "data": {"type": "chat_login_moderation", "moderation_action": "acknowledge_warning", "args": null}});
        assert!(pubsub(&ack).is_none());
        let note = json!({"type": "moderation_action", "data": {"type": "chat_targeted_login_moderation", "moderation_action": "delete_notification", "args": ["hi"]}});
        assert!(pubsub(&note).is_none());
        assert!(pubsub(&json!({"type": "something_else", "data": {}})).is_none());
    }

    #[test]
    fn automod_decisions_name_the_mod_and_the_sender() {
        let p = json!({"type": "automod_caught_message", "data": {"id": "c1", "status": "DENIED",
            "resolver_id": "2", "resolver_login": "kuni",
            "message": {"id": "m1", "content": {"text": "bad"}, "sender": {"user_id": "3", "login": "x", "display_name": "X"}}}});
        let d = automod_queue(&p).unwrap();
        assert!(!d.approved);
        let l = ModLine::decision(&d, "a".into(), 1);
        assert_eq!(
            (l.group, l.action.as_str(), l.about.as_str()),
            (ModGroup::Automod, "denied a held message from", "bad")
        );
        assert_eq!(l.target.map(|t| t.name), Some("X".into()));
        assert_eq!(l.moderator.map(|t| t.login), Some("kuni".into()));
        // Still waiting, or let go by Twitch: no decision.
        let pending = json!({"type": "automod_caught_message", "data": {"status": "PENDING"}});
        assert!(automod_queue(&pending).is_none());
        assert!(Decision::from_status("Expired", None, None, String::new()).is_none());
        assert!(
            Decision::from_status("Approved", Some(&who("kuni")), None, String::new())
                .unwrap()
                .approved
        );
    }

    #[test]
    fn hermes_pending_and_resolved() {
        let held = |status: &str| {
            json!({"type": "automod_caught_message", "data": {"status": status, "resolver_login": "kuni",
                "content_classification": {"category": "aggressive", "level": 2},
                "message": {"id": "m1", "content": {"text": "hi"}, "sender": {"user_id": "3", "login": "x", "display_name": "X"}}}})
        };
        let p = automod_pending(&held("PENDING")).unwrap();
        assert_eq!((p.message_id.as_str(), p.reason.as_str(), p.text.as_str()), ("m1", "aggressive", "hi"));
        assert_eq!((p.sender.id.as_str(), p.sender.login.as_str(), p.sender.name.as_str()), ("3", "x", "X"));
        assert!(automod_pending(&held("pending")).is_some());
        let bare = json!({"type": "automod_caught_message", "data": {"status": "PENDING", "message": {"id": "m2"}}});
        assert_eq!(automod_pending(&bare).unwrap().reason, "AutoMod");
        assert!(automod_pending(&held("DENIED")).is_none());
        assert!(automod_resolved(&held("PENDING")).is_none());
        for (status, want) in [("ALLOWED", "allowed"), ("DENIED", "denied"), ("EXPIRED", "expired")] {
            let r = automod_resolved(&held(status)).unwrap();
            assert_eq!((r.message_id.as_str(), r.status.as_str(), r.by.as_deref()), ("m1", want, Some("kuni")));
        }
        assert!(automod_resolved(&json!({"type": "automod_caught_message", "data": {"status": "DENIED"}})).is_none());
        assert!(automod_queue(&held("PENDING")).is_none());
    }

    #[test]
    fn v107_n3d_history_terms_read_as_live() {
        let term = |done: &str| ChannelAction {
            icon: "SHIELD".into(),
            category: "OTHER".into(),
            moderator: Some(("kuni".into(), "Kuni".into())),
            title: "\"bad word\"".into(),
            done: done.into(),
            ..Default::default()
        };
        let l = ModLine::past(&term("Added as Blocked Term"), 9);
        assert_eq!((l.group, l.action.as_str(), l.about.as_str()), (ModGroup::Automod, "added the blocked term", "bad word"));
        let l = ModLine::past(&term("Removed as Permitted Term"), 9);
        assert_eq!((l.group, l.action.as_str(), l.about.as_str()), (ModGroup::Automod, "stopped allowing", "bad word"));
        let l = ModLine::past(&term("Something Else"), 9);
        assert_eq!((l.group, l.about.as_str()), (ModGroup::Other, ""));
        let delay = ChannelAction {
            icon: "SWORD".into(),
            title: "Non-Mod Chat Delay".into(),
            done: "Enabled with 2s delay,".into(),
            ..Default::default()
        };
        let l = ModLine::past(&delay, 9);
        assert_eq!((l.group, l.action.as_str()), (ModGroup::Mode, "enabled with 2s delay: Non-Mod Chat Delay"), "V107-N3g");
    }

    #[test]
    fn history_lines_keep_twitchs_words() {
        let ban = ChannelAction {
            id: "h1".into(),
            icon: "BAN".into(),
            category: "BANS_AND_UNBANS".into(),
            moderator: Some(("kuni".into(), "Kuni".into())),
            target: Some(("x".into(), "X".into())),
            done: "Banned".into(),
            reason: "spam".into(),
            ..Default::default()
        };
        let l = ModLine::past(&ban, 9);
        assert_eq!(
            (l.group, l.action.as_str(), l.reason.as_str(), l.past),
            (ModGroup::Ban, "banned", "spam", true)
        );
        let clear = ChannelAction {
            icon: "HALT".into(),
            category: "CHAT_MODE_CHANGES".into(),
            title: "Clear Chat".into(),
            done: "Done".into(),
            note: "Chat was cleared.".into(),
            ..Default::default()
        };
        let l = ModLine::past(&clear, 9);
        assert_eq!(
            (l.group, l.action.as_str(), l.reason.as_str()),
            (ModGroup::Delete, "clear chat", "")
        );
        let level = ChannelAction {
            icon: "AUTOMOD".into(),
            category: "AUTOMOD_LEVEL_CHANGES".into(),
            title: "Level 2".into(),
            done: "Default AutoMod level updated".into(),
            ..Default::default()
        };
        assert_eq!(
            ModLine::past(&level, 9).action,
            "default AutoMod level updated: Level 2"
        );
        let modded = ChannelAction {
            icon: "SWORD".into(),
            category: "MODS_AND_VIPS".into(),
            done: "Added as a Moderator".into(),
            target: Some(("x".into(), "X".into())),
            ..Default::default()
        };
        assert_eq!(
            (
                ModLine::past(&modded, 9).group,
                ModLine::past(&modded, 9).action.as_str()
            ),
            (ModGroup::Role, "modded")
        );
        let vip = ChannelAction {
            category: "MODS_AND_VIPS".into(),
            done: "Added as a VIP".into(),
            target: Some(("x".into(), "X".into())),
            ..Default::default()
        };
        assert_eq!(ModLine::past(&vip, 9).action, "gave VIP to");
        let gone = ChannelAction { done: "Removed as a VIP".into(), ..vip.clone() };
        assert_eq!(ModLine::past(&gone, 9).action, "took VIP from");
        // The quoted wording reads the same, never as raw text.
        let quoted = ChannelAction { done: "Added as \"Mod\"".into(), ..vip.clone() };
        assert_eq!(ModLine::past(&quoted, 9).action, "modded");
        let unquoted = ChannelAction { done: "Removed as \"Mod\"".into(), ..vip.clone() };
        assert_eq!(ModLine::past(&unquoted, 9).action, "unmodded");
        // Nouns before a name read as verbs.
        let deleted = ChannelAction {
            icon: "TRASH".into(),
            done: "Message Deleted".into(),
            target: Some(("x".into(), "X".into())),
            ..Default::default()
        };
        assert_eq!(ModLine::past(&deleted, 9).action, "deleted a message from");
        let lifted = ChannelAction {
            category: "TIMEOUTS_AND_UNTIMEOUTS".into(),
            done: "Timeout Removed".into(),
            ..deleted.clone()
        };
        assert_eq!(ModLine::past(&lifted, 9).action, "lifted the timeout on");
    }

    #[test]
    fn the_same_action_shows_once() {
        let mut lines = VecDeque::new();
        let live = ModLine::live(&moderate("ban"), None, "l1".into(), 100_000);
        assert!(push(&mut lines, live.clone(), 3));
        // A second account's feed, a moment later.
        assert!(!push(
            &mut lines,
            ModLine {
                id: "l2".into(),
                ts: 101_000,
                ..live.clone()
            },
            3
        ));
        // History: the same ban, Twitch's id and words; an older one; one seen already.
        let past = ModLine {
            id: "h1".into(),
            ts: 99_500,
            action: "Banned".into(),
            past: true,
            ..live.clone()
        };
        let older = ModLine {
            id: "h2".into(),
            ts: 50_000,
            past: true,
            ..live.clone()
        };
        merge(&mut lines, vec![past, older.clone(), older.clone()], 3);
        assert_eq!(
            lines.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(),
            ["l1", "h2"]
        );
        // Newer than everything goes first; the cap drops the oldest.
        let mut t = moderate("timeout");
        t.target = Some(who("y"));
        assert!(push(
            &mut lines,
            ModLine::live(&t, Some(60), "l3".into(), 200_000),
            2
        ));
        assert_eq!(
            lines.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(),
            ["l3", "l1"]
        );
    }
}
