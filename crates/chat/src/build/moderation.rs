//! CLEARCHAT, CLEARMSG and NOTICE (C14, send errors).

use crate::irc::Line;
use crate::model::{ChatMessage, NoticeKind};

use super::{Ctx, Incoming, system, timestamp};

pub fn clearchat(line: &Line<'_>, ctx: &mut Ctx<'_>) -> Incoming {
    let ts = timestamp(line, ctx.now_ms);
    let Some(user_id) = line.tag_nonempty("target-user-id") else {
        return Incoming::Clear {
            notice: system(NoticeKind::Moderation, "Chat was cleared by a moderator.", ts),
        };
    };
    let login = line.trailing().unwrap_or("Someone");
    let secs = line.tag_num::<u64>("ban-duration");
    let text = match secs {
        Some(secs) => format!("{login} was timed out for {}.", duration_text(secs)),
        None => format!("{login} was banned."),
    };
    Incoming::ClearUser {
        user_id: user_id.to_owned(),
        secs,
        notice: system(NoticeKind::Moderation, text, ts),
    }
}

pub fn clearmsg(line: &Line<'_>) -> Option<Incoming> {
    let id = line.tag_nonempty("target-msg-id")?;
    Some(Incoming::Delete { id: id.to_owned() })
}

pub fn notice(line: &Line<'_>, ctx: &mut Ctx<'_>) -> Incoming {
    let msg_id = line.tag_nonempty("msg-id").unwrap_or("").to_owned();
    let text = short_notice(&msg_id)
        .map(str::to_owned)
        .unwrap_or_else(|| line.trailing().unwrap_or("").to_owned());
    let kind = if msg_id.starts_with("msg_") {
        NoticeKind::Error
    } else {
        NoticeKind::Info
    };
    let notice: ChatMessage = system(kind, text, timestamp(line, ctx.now_ms));
    Incoming::Notice { msg_id, notice }
}

/// Shorter wording for the notices people see most. Others keep Twitch's text.
fn short_notice(msg_id: &str) -> Option<&'static str> {
    Some(match msg_id {
        "msg_duplicate" => "Not sent: same as your last message.",
        "msg_ratelimit" => "Not sent: you're sending too fast.",
        "msg_emoteonly" => "Not sent: emote-only mode is on.",
        "msg_subsonly" => "Not sent: this chat is subscribers-only.",
        "msg_r9k" => "Not sent: unique-chat mode is on and that message isn't unique.",
        "msg_banned" => "You're banned from this chat.",
        "msg_channel_suspended" => "This channel is suspended.",
        "msg_verified_email" => "Not sent: this chat needs a verified email.",
        "msg_requires_verified_phone_number" => "Not sent: this chat needs a verified phone number.",
        _ => return None,
    })
}

/// `90` → `1m 30s`, `86400` → `1d`.
pub fn duration_text(secs: u64) -> String {
    const UNITS: [(u64, &str); 4] = [(86_400, "d"), (3_600, "h"), (60, "m"), (1, "s")];
    if secs == 0 {
        return "0s".into();
    }
    let mut left = secs;
    let parts: Vec<String> = UNITS
        .iter()
        .filter_map(|&(size, unit)| {
            let n = left / size;
            left %= size;
            (n > 0).then(|| format!("{n}{unit}"))
        })
        .take(2)
        .collect();
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::super::Colors;
    use super::*;
    use crate::model::Notice;

    fn run(raw: &str) -> Incoming {
        let mut colors = Colors::default();
        let mut ctx = Ctx::new(None, &mut colors, 5);
        super::super::build(&Line::parse(raw).unwrap(), &mut ctx).unwrap()
    }

    fn text(m: &ChatMessage) -> &str {
        m.notice.as_ref().map(|n: &Notice| n.text.as_str()).unwrap()
    }

    #[test]
    fn timeouts_bans_and_clears() {
        let Incoming::ClearUser { user_id, notice, .. } =
            run("@ban-duration=600;room-id=1;target-user-id=9;tmi-sent-ts=10 :tmi.twitch.tv CLEARCHAT #c :foo")
        else {
            panic!()
        };
        assert_eq!((user_id.as_str(), text(&notice), notice.ts), ("9", "foo was timed out for 10m.", 10));
        let Incoming::ClearUser { notice, .. } = run("@room-id=1;target-user-id=9 :tmi.twitch.tv CLEARCHAT #c :foo")
        else {
            panic!()
        };
        assert_eq!(text(&notice), "foo was banned.");
        assert!(matches!(run("@room-id=1 :tmi.twitch.tv CLEARCHAT #c"), Incoming::Clear { .. }));
    }

    #[test]
    fn deleted_message() {
        let Incoming::Delete { id } = run("@login=foo;target-msg-id=abc :tmi.twitch.tv CLEARMSG #c :hi") else {
            panic!()
        };
        assert_eq!(id, "abc");
    }

    #[test]
    fn notices() {
        let Incoming::Notice { msg_id, notice } =
            run("@msg-id=msg_duplicate :tmi.twitch.tv NOTICE #c :Your message was not sent because it is identical")
        else {
            panic!()
        };
        assert_eq!(msg_id, "msg_duplicate");
        assert_eq!(notice.notice.unwrap().kind, NoticeKind::Error);
        let Incoming::Notice { notice, .. } = run("@msg-id=emote_only_on :tmi.twitch.tv NOTICE #c :This room is now in emote-only mode.")
        else {
            panic!()
        };
        assert_eq!(text(&notice), "This room is now in emote-only mode.");
    }

    #[test]
    fn durations() {
        assert_eq!(duration_text(90), "1m 30s");
        assert_eq!(duration_text(86_400), "1d");
        assert_eq!(duration_text(1_209_600), "14d");
        assert_eq!(duration_text(3_661), "1h 1m");
    }
}
