//! 1.48, C14 Mod actions as short lines for chat and the mod log: "Kuni timed out x for
//! 10m: caps". `secs` is a timeout's length, worked out by the caller from its end.

use cyex_chat::build::duration_text;
use cyex_core::number::grouped;
use cyex_eventsub::events::Moderate;

pub fn describe(m: &Moderate, secs: Option<u64>) -> String {
    let by = if m.moderator.name.is_empty() { "A mod" } else { m.moderator.name.as_str() };
    let who = m.target.as_ref().map_or("someone", |t| if t.name.is_empty() { &t.login } else { &t.name });
    let why = |s: String| if m.reason.is_empty() { format!("{s}.") } else { format!("{s}: {}", m.reason) };
    let term = || m.terms.iter().map(|t| format!("“{t}”")).collect::<Vec<_>>().join(", ");
    let toggled = |what: &str, on: bool| format!("{by} turned {what} {}.", if on { "on" } else { "off" });
    let action = m.action.strip_prefix("shared_chat_").unwrap_or(&m.action);
    let line = match action {
        "ban" => why(format!("{by} banned {who}")),
        "timeout" => why(match secs {
            Some(s) => format!("{by} timed out {who} for {}", duration_text(s)),
            None => format!("{by} timed out {who}"),
        }),
        "unban" => format!("{by} unbanned {who}."),
        "untimeout" => format!("{by} lifted {who}’s timeout."),
        "delete" => match &m.message_body {
            Some(b) if !b.is_empty() => format!("{by} deleted {who}’s message: {b}"),
            _ => format!("{by} deleted a message from {who}."),
        },
        "clear" => format!("{by} cleared the chat."),
        "emoteonly" | "emoteonlyoff" => toggled("emote-only mode", action == "emoteonly"),
        "subscribers" | "subscribersoff" => toggled("subscribers-only mode", action == "subscribers"),
        "uniquechat" | "uniquechatoff" => toggled("unique-chat mode", action == "uniquechat"),
        "followers" => match m.amount {
            Some(min) if min > 0 => format!("{by} turned on followers-only mode ({}).", duration_text(min * 60)),
            _ => format!("{by} turned on followers-only mode."),
        },
        "followersoff" => toggled("followers-only mode", false),
        "slow" => format!("{by} turned on slow mode ({}).", duration_text(m.amount.unwrap_or(30))),
        "slowoff" => toggled("slow mode", false),
        "vip" => format!("{by} made {who} a VIP."),
        "unvip" => format!("{by} removed {who} as VIP."),
        "mod" => format!("{by} made {who} a mod."),
        "unmod" => format!("{by} removed {who} as mod."),
        "raid" => match m.amount {
            Some(n) => format!("{by} raided {who} with {} viewers.", grouped(n)),
            None => format!("{by} raided {who}."),
        },
        "unraid" => format!("{by} canceled the raid."),
        "add_blocked_term" => format!("{by} blocked {}.", term()),
        "remove_blocked_term" => format!("{by} unblocked {}.", term()),
        "add_permitted_term" => format!("{by} allowed {}.", term()),
        "remove_permitted_term" => format!("{by} no longer allows {}.", term()),
        "approve_unban_request" => why(format!("{by} approved {who}’s unban request")),
        "deny_unban_request" => why(format!("{by} denied {who}’s unban request")),
        "warn" => why(format!("{by} warned {who}")),
        "approved_automod_message" => format!("{by} approved a held message from {who}."),
        "denied_automod_message" => format!("{by} denied a held message from {who}."),
        "shield" | "shieldoff" => toggled("shield mode", action == "shield"),
        other => format!("{by}: {other}."),
    };
    match &m.source_channel {
        Some(c) if m.action.starts_with("shared_chat_") => format!("In {c}’s chat: {line}"),
        _ => line,
    }
}

#[cfg(test)]
mod tests {
    use cyex_eventsub::events::Who;

    use super::*;

    fn m(action: &str) -> Moderate {
        Moderate {
            moderator: Who {
                id: "2".into(),
                login: "kuni".into(),
                name: "Kuni".into(),
            },
            action: action.into(),
            target: Some(Who {
                id: "3".into(),
                login: "x".into(),
                name: "X".into(),
            }),
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
    fn lines() {
        let mut t = m("timeout");
        t.reason = "caps".into();
        assert_eq!(describe(&t, Some(600)), "Kuni timed out X for 10m: caps");
        assert_eq!(describe(&m("ban"), None), "Kuni banned X.");
        let mut s = m("slow");
        s.amount = Some(30);
        s.target = None;
        assert_eq!(describe(&s, None), "Kuni turned on slow mode (30s).");
        let mut d = m("delete");
        d.message_body = Some("bad".into());
        assert_eq!(describe(&d, None), "Kuni deleted X’s message: bad");
        let mut b = m("add_blocked_term");
        b.terms = vec!["foo".into()];
        assert_eq!(describe(&b, None), "Kuni blocked “foo”.");
        let mut sc = m("shared_chat_ban");
        sc.source_channel = Some("other".into());
        assert_eq!(describe(&sc, None), "In other’s chat: Kuni banned X.");
    }
}
