//! 1.49, 1.50, M3 Twitch's mod and broadcaster commands, read from a chat line into an
//! action. Running them (Helix) is the app's job.

use cyex_twitch::helix::modes::{ANNOUNCE_COLORS, MAX_FOLLOW_MINUTES, ModeChange};
use cyex_twitch::helix::moderation::{MAX_TIMEOUT_SECS, PIN_SECS};
use cyex_twitch::helix::roles::Role;

use cyex_chat::model::ModRole;

use crate::duration;

/// Default timeout, like Twitch's: 10 minutes.
pub const DEFAULT_TIMEOUT: u32 = 600;
const DEFAULT_SLOW: u32 = 30;

/// M3 How Twitch treats a suspicious user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Treatment {
    /// Messages show to mods with a marker.
    Monitor,
    /// Messages show to mods only.
    Restrict,
    /// Back to normal.
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModCommand {
    Ban { login: String, reason: String },
    Timeout { login: String, secs: u32, reason: String },
    /// Lifts a ban or a timeout.
    Unban { login: String },
    Delete { msg_id: String },
    Clear,
    Mode(ModeChange),
    Announce { color: &'static str, text: String },
    Shoutout { login: String },
    Raid { login: String },
    Unraid,
    Warn { login: String, reason: String },
    Shield(bool),
    Treat { login: String, how: Treatment },
    SetRole { login: String, role: Role, on: bool },
    ListRole(Role),
    /// V107-R28 Gives (`on`) or takes the lead moderator role.
    SetLead { login: String, on: bool },
    Commercial { secs: u32 },
    Marker { text: String },
    /// 1.9 Pins a message, for `secs` or until the stream ends.
    Pin { msg_id: String, secs: Option<u32> },
    /// M11 Sends a message and pins it, like Twitch's own `/pin <text>`. Twitch's usual
    /// 20 minutes when `secs` isn't given.
    PinSend { text: String, secs: Option<u32> },
    /// Takes the current pin down.
    Unpin,
}

/// Who may run a command (1.57). Checked against the account's role by `Need::allows`,
/// the one check every mod-only action goes through (V102-21).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Need {
    /// Moderators, lead moderators and the broadcaster.
    Mod,
    /// 1.50 Giving and taking mod and VIP: the broadcaster or a lead moderator.
    Lead,
    Broadcaster,
}

impl Need {
    pub fn allows(self, role: Option<ModRole>) -> bool {
        let least = match self {
            Need::Mod => ModRole::Moderator,
            Need::Lead => ModRole::LeadModerator,
            Need::Broadcaster => ModRole::Broadcaster,
        };
        role.is_some_and(|r| r >= least)
    }

    /// What a role that falls short is told.
    pub fn refusal(self) -> &'static str {
        match self {
            Need::Mod => "You’re not a moderator here.",
            Need::Lead => "Only the broadcaster or a lead mod can do that.",
            Need::Broadcaster => "Only the broadcaster can do that.",
        }
    }
}

impl ModCommand {
    /// What the app checks before trying. V107 Role changes are open to every mod: a lead
    /// mod may wear the plain mod badge, so Twitch decides (`roles`).
    pub fn needs(&self) -> Need {
        match self {
            ModCommand::SetRole { .. } => Need::Mod,
            _ => self.offered(),
        }
    }

    /// Who it's offered to (Tab, menus, `/help`): role changes only to lead mods and the
    /// broadcaster, as Twitch allows.
    pub fn offered(&self) -> Need {
        match self {
            ModCommand::SetRole { .. } => Need::Lead,
            ModCommand::ListRole(_)
            | ModCommand::SetLead { .. }
            | ModCommand::Raid { .. }
            | ModCommand::Unraid
            | ModCommand::Commercial { .. }
            | ModCommand::Marker { .. } => Need::Broadcaster,
            _ => Need::Mod,
        }
    }
}

/// Command names this reads; `/nuke` is handled on its own (M1).
pub const NAMES: &[&str] = &[
    "ban", "unban", "timeout", "untimeout", "delete", "clear", "slow", "slowoff", "followers", "followersoff",
    "subscribers", "subscribersoff", "emoteonly", "emoteonlyoff", "uniquechat", "uniquechatoff", "r9kbeta",
    "r9kbetaoff", "announce", "announceblue", "announcegreen", "announceorange", "announcepurple", "shoutout", "so",
    "raid", "unraid", "warn", "shield", "shieldoff", "monitor", "unmonitor", "restrict", "unrestrict", "mod", "unmod",
    "vip", "unvip", "mods", "vips", "commercial", "marker", "pin", "unpin", "leadmod", "unleadmod",
];

/// `None` when `name` isn't one of these commands; otherwise the action or a usage hint.
pub fn parse(name: &str, args: &str) -> Option<Result<ModCommand, String>> {
    if !NAMES.contains(&name) {
        return None;
    }
    Some(read(name, args.trim()))
}

/// V1041-01 What a command name needs, for Tab completion; `None` when it isn't one of
/// these. Read from what the name parses to, so [`ModCommand::offered`] stays the one place
/// that says it: a name and a word fit every command here, and lengths take nothing.
pub fn need(name: &str) -> Option<Need> {
    ["x y", ""]
        .into_iter()
        .find_map(|args| parse(name, args)?.ok())
        .map(|c| c.offered())
}

/// Twitch's message ids: a UUID in lowercase hex.
fn is_message_id(s: &str) -> bool {
    s.len() == 36
        && s.char_indices()
            .all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { c == '-' } else { c.is_ascii_hexdigit() })
}

fn read(name: &str, args: &str) -> Result<ModCommand, String> {
    let (first, rest) = split(args);
    let login = || login(first).ok_or_else(|| format!("Usage: /{name} name"));
    use ModCommand as C;
    Ok(match name {
        "ban" => C::Ban {
            login: login()?,
            reason: rest.to_owned(),
        },
        "timeout" => {
            let login = login().map_err(|_| "Usage: /timeout name [10m] [reason]".to_owned())?;
            let (len, reason) = split(rest);
            match len {
                "" => C::Timeout {
                    login,
                    secs: DEFAULT_TIMEOUT,
                    reason: String::new(),
                },
                _ if duration::looks_like(len) => {
                    let secs = duration::parse(len, 1).unwrap_or(0);
                    if secs == 0 || secs > u64::from(MAX_TIMEOUT_SECS) {
                        return Err("Timeouts go from 1 second to 2 weeks.".into());
                    }
                    C::Timeout {
                        login,
                        secs: secs as u32,
                        reason: reason.to_owned(),
                    }
                }
                _ => C::Timeout {
                    login,
                    secs: DEFAULT_TIMEOUT,
                    reason: rest.to_owned(),
                },
            }
        }
        "unban" | "untimeout" => C::Unban { login: login()? },
        "delete" => {
            if first.is_empty() {
                return Err("Usage: /delete message-id".into());
            }
            C::Delete { msg_id: first.to_owned() }
        }
        "clear" => C::Clear,
        "slow" => {
            let secs = match first {
                "" => DEFAULT_SLOW,
                t => duration::parse(t, 1).ok_or("Usage: /slow [30]")? as u32,
            };
            C::Mode(ModeChange::Slow(Some(secs)))
        }
        "slowoff" => C::Mode(ModeChange::Slow(None)),
        "followers" => {
            let mins = match first {
                "" => 0,
                t => duration::parse(t, 60).ok_or("Usage: /followers [10m]")? / 60,
            };
            if mins > u64::from(MAX_FOLLOW_MINUTES) {
                return Err("Followers-only goes up to 3 months.".into());
            }
            C::Mode(ModeChange::Followers(Some(mins as u32)))
        }
        "followersoff" => C::Mode(ModeChange::Followers(None)),
        "subscribers" => C::Mode(ModeChange::Subs(true)),
        "subscribersoff" => C::Mode(ModeChange::Subs(false)),
        "emoteonly" => C::Mode(ModeChange::Emote(true)),
        "emoteonlyoff" => C::Mode(ModeChange::Emote(false)),
        "uniquechat" | "r9kbeta" => C::Mode(ModeChange::Unique(true)),
        "uniquechatoff" | "r9kbetaoff" => C::Mode(ModeChange::Unique(false)),
        _ if name.starts_with("announce") => {
            if args.is_empty() {
                return Err(format!("Usage: /{name} message"));
            }
            let color = name.strip_prefix("announce").filter(|c| !c.is_empty()).unwrap_or("primary");
            C::Announce {
                color: ANNOUNCE_COLORS.iter().find(|c| **c == color).copied().unwrap_or("primary"),
                text: args.to_owned(),
            }
        }
        "shoutout" | "so" => C::Shoutout { login: login()? },
        "raid" => C::Raid { login: login()? },
        "unraid" => C::Unraid,
        "warn" => {
            let login = login().map_err(|_| "Usage: /warn name reason".to_owned())?;
            if rest.is_empty() {
                return Err("A warning needs a reason: /warn name reason".into());
            }
            C::Warn {
                login,
                reason: rest.to_owned(),
            }
        }
        "shield" => C::Shield(true),
        "shieldoff" => C::Shield(false),
        "monitor" => C::Treat {
            login: login()?,
            how: Treatment::Monitor,
        },
        "restrict" => C::Treat {
            login: login()?,
            how: Treatment::Restrict,
        },
        "unmonitor" | "unrestrict" => C::Treat {
            login: login()?,
            how: Treatment::None,
        },
        "mod" | "unmod" | "vip" | "unvip" => C::SetRole {
            login: login()?,
            role: if name.ends_with("mod") { Role::Mod } else { Role::Vip },
            on: !name.starts_with("un"),
        },
        "leadmod" | "unleadmod" => C::SetLead {
            login: login()?,
            on: name == "leadmod",
        },
        "mods" => C::ListRole(Role::Mod),
        "vips" => C::ListRole(Role::Vip),
        "commercial" => {
            let secs = match first {
                "" => 30,
                t => t.parse::<u32>().map_err(|_| "Usage: /commercial [30|60|90|120|150|180]")?,
            };
            C::Commercial { secs }
        }
        "marker" => C::Marker { text: args.to_owned() },
        "pin" => {
            if first.is_empty() {
                return Err("Usage: /pin message, or /pin message-id [5m]".into());
            }
            if !is_message_id(first) {
                return Ok(C::PinSend {
                    text: args.to_owned(),
                    secs: None,
                });
            }
            let secs = match rest {
                "" => None,
                t => Some(duration::parse(t, 1).ok_or("Usage: /pin message-id [5m]")?.clamp(u64::from(PIN_SECS.0), u64::from(PIN_SECS.1)) as u32),
            };
            C::Pin {
                msg_id: first.to_owned(),
                secs,
            }
        }
        "unpin" => C::Unpin,
        _ => return Err(format!("/{name} isn’t supported.")),
    })
}

/// First word and the rest, both trimmed.
fn split(s: &str) -> (&str, &str) {
    let s = s.trim();
    match s.split_once(char::is_whitespace) {
        Some((a, b)) => (a, b.trim()),
        None => (s, ""),
    }
}

/// A login from `@Name`, `name` or `#name`, lowercased. `None` if it can't be one.
pub fn login(word: &str) -> Option<String> {
    let l = word.trim_start_matches(['@', '#']).trim_end_matches(',').to_ascii_lowercase();
    ((1..=25).contains(&l.len()) && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')).then_some(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(line: &str) -> ModCommand {
        let (n, a) = line.split_once(' ').unwrap_or((line, ""));
        parse(n, a).unwrap().unwrap()
    }

    fn err(line: &str) -> String {
        let (n, a) = line.split_once(' ').unwrap_or((line, ""));
        parse(n, a).unwrap().unwrap_err()
    }

    #[test]
    fn bans_and_timeouts() {
        assert_eq!(
            ok("ban @Spammer buying followers"),
            ModCommand::Ban {
                login: "spammer".into(),
                reason: "buying followers".into()
            }
        );
        assert_eq!(
            ok("timeout x"),
            ModCommand::Timeout {
                login: "x".into(),
                secs: 600,
                reason: String::new()
            }
        );
        assert_eq!(
            ok("timeout x 1h30m caps"),
            ModCommand::Timeout {
                login: "x".into(),
                secs: 5_400,
                reason: "caps".into()
            }
        );
        // A reason without a length keeps the default.
        assert!(matches!(ok("timeout x stop that"), ModCommand::Timeout { secs: 600, ref reason, .. } if reason == "stop that"));
        assert!(err("timeout x 3w").contains("2 weeks"));
        assert!(err("ban").starts_with("Usage"));
        assert_eq!(ok("untimeout x"), ModCommand::Unban { login: "x".into() });
    }

    #[test]
    fn twitchs_own_syntax_and_its_edges() {
        // V102-11 user, optional length and reason; a warning needs a reason.
        assert_eq!(ok("ban x"), ModCommand::Ban { login: "x".into(), reason: String::new() });
        assert!(matches!(ok("timeout x 1209600"), ModCommand::Timeout { secs: 1_209_600, .. }));
        assert!(err("timeout x 1209601").contains("2 weeks"));
        assert!(err("timeout x 0").contains("1 second"));
        assert!(err("timeout").starts_with("Usage: /timeout"));
        assert_eq!(ok("unban @SomeOne"), ModCommand::Unban { login: "someone".into() });
        assert!(err("unban").starts_with("Usage"));
        assert_eq!(ok("warn x stop it"), ModCommand::Warn { login: "x".into(), reason: "stop it".into() });
        assert!(err("warn").starts_with("Usage: /warn"));
        assert!(err("ban not/a/name").starts_with("Usage"));
    }

    #[test]
    fn modes() {
        assert_eq!(ok("slow"), ModCommand::Mode(ModeChange::Slow(Some(30))));
        assert_eq!(ok("slow 10"), ModCommand::Mode(ModeChange::Slow(Some(10))));
        assert_eq!(ok("followers 10"), ModCommand::Mode(ModeChange::Followers(Some(10))));
        assert_eq!(ok("followers 1h"), ModCommand::Mode(ModeChange::Followers(Some(60))));
        assert_eq!(ok("followers"), ModCommand::Mode(ModeChange::Followers(Some(0))));
        assert_eq!(ok("r9kbeta"), ModCommand::Mode(ModeChange::Unique(true)));
        assert_eq!(ok("emoteonlyoff"), ModCommand::Mode(ModeChange::Emote(false)));
    }

    #[test]
    fn the_rest() {
        assert_eq!(
            ok("announcegreen hi all"),
            ModCommand::Announce {
                color: "green",
                text: "hi all".into()
            }
        );
        assert!(err("warn x").contains("reason"));
        assert_eq!(
            ok("vip Friend"),
            ModCommand::SetRole {
                login: "friend".into(),
                role: Role::Vip,
                on: true
            }
        );
        assert_eq!(ok("unmod x").offered(), Need::Lead);
        assert_eq!(ok("mods").needs(), Need::Broadcaster);
        assert_eq!(ok("restrict x").needs(), Need::Mod);
        assert!(parse("me", "hi").is_none());
    }

    /// V102-21 The one check: every role reaches what it should and nothing more.
    #[test]
    fn roles_meet_needs() {
        use ModRole::{Broadcaster, LeadModerator, Moderator};
        for (role, mod_, lead, owner) in [
            (None, false, false, false),
            (Some(Moderator), true, false, false),
            (Some(LeadModerator), true, true, false),
            (Some(Broadcaster), true, true, true),
        ] {
            assert_eq!(Need::Mod.allows(role), mod_, "{role:?}");
            assert_eq!(Need::Lead.allows(role), lead, "{role:?}");
            assert_eq!(Need::Broadcaster.allows(role), owner, "{role:?}");
        }
        // The ban family is open to every mod, as V102-11 asks.
        for line in ["ban x", "timeout x 10", "unban x", "warn x be nice"] {
            assert!(ok(line).needs().allows(Some(Moderator)), "{line}");
            assert!(!ok(line).needs().allows(None), "{line}");
        }
    }

    /// 1.50 Mod and VIP changes show to lead mods and the broadcaster, and any mod may try
    /// them (a lead mod in the plain mod badge); Twitch decides.
    #[test]
    fn role_changes_are_offered_to_lead_mods_and_tried_by_mods() {
        use ModRole::{Broadcaster, LeadModerator, Moderator};
        for line in ["mod x", "unmod x", "vip x", "unvip x"] {
            let c = ok(line);
            assert_eq!((c.offered(), c.needs()), (Need::Lead, Need::Mod), "{line}");
            assert!(!c.offered().allows(Some(Moderator)), "{line}");
            assert!(c.offered().allows(Some(LeadModerator)) && c.offered().allows(Some(Broadcaster)), "{line}");
            assert!(c.needs().allows(Some(Moderator)) && !c.needs().allows(None), "{line}");
        }
        // Listing stays the broadcaster's: Helix only answers their token.
        assert!(!ok("mods").needs().allows(Some(LeadModerator)));
        assert!(!ok("vips").offered().allows(Some(LeadModerator)));
        // V107-R28 Lead mod is the broadcaster's alone.
        assert_eq!(ok("leadmod @Caressz"), ModCommand::SetLead { login: "caressz".into(), on: true });
        assert_eq!(ok("unleadmod caressz"), ModCommand::SetLead { login: "caressz".into(), on: false });
        for line in ["leadmod x", "unleadmod x"] {
            assert_eq!((ok(line).offered(), ok(line).needs()), (Need::Broadcaster, Need::Broadcaster), "{line}");
        }
        assert!(err("leadmod").contains("/leadmod name"));
    }

    /// V1041-01 Every name has a need, the same one its parsed command is offered with.
    #[test]
    fn every_name_has_a_need() {
        for name in NAMES {
            assert!(need(name).is_some(), "{name}");
        }
        assert_eq!(need("ban"), Some(Need::Mod));
        assert_eq!(need("slow"), Some(Need::Mod));
        assert_eq!(need("vip"), Some(Need::Lead));
        assert_eq!(need("mods"), Some(Need::Broadcaster));
        assert_eq!(need("commercial"), Some(Need::Broadcaster));
        assert_eq!(need("me"), None);
    }

    #[test]
    fn pin_takes_a_message_id_or_text_to_send() {
        let id = "bddd8905-c104-4f7a-9c2e-0a1b2c3d4e5f";
        assert_eq!(
            ok(&format!("pin {id} 5m")),
            ModCommand::Pin {
                msg_id: id.into(),
                secs: Some(300)
            }
        );
        // V107 Any second Twitch takes, clamped to 30 s..30 min.
        let secs = |t: &str| match ok(&format!("pin {id} {t}")) {
            ModCommand::Pin { secs, .. } => secs,
            _ => None,
        };
        assert_eq!((secs("777s"), secs("10s"), secs("2h")), (Some(777), Some(30), Some(1800)));
        // M11 Anything else is a message to send and pin.
        assert_eq!(
            ok("pin Giveaway at 8, type !join"),
            ModCommand::PinSend {
                text: "Giveaway at 8, type !join".into(),
                secs: None
            }
        );
        assert!(err("pin").contains("Usage"));
    }
}
