//! CLEARCHAT, CLEARMSG and NOTICE (C14, send errors).

use crate::irc::Line;
use crate::model::{ChatMessage, NoticeKind, NoticeUser};

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
    let mut notice = system(NoticeKind::Moderation, text, ts);
    // V111-16 The name opens their usercard.
    if let Some(n) = notice.notice.as_mut().filter(|_| !login.is_empty() && line.trailing().is_some()) {
        n.links = vec![NoticeUser::new(login, login)];
    }
    Incoming::ClearUser { user_id: user_id.to_owned(), secs, notice }
}

pub fn clearmsg(line: &Line<'_>) -> Option<Incoming> {
    let id = line.tag_nonempty("target-msg-id")?;
    Some(Incoming::Delete { id: id.to_owned() })
}

pub fn notice(line: &Line<'_>, ctx: &mut Ctx<'_>) -> Incoming {
    let msg_id = line.tag_nonempty("msg-id").unwrap_or("").to_owned();
    let said = line.trailing().unwrap_or("");
    let secs = seconds_in(said);
    let text = short_notice(&msg_id, secs).unwrap_or_else(|| said.to_owned());
    let kind = if msg_id.starts_with("msg_") {
        NoticeKind::Error
    } else {
        NoticeKind::Info
    };
    let notice: ChatMessage = system(kind, text, timestamp(line, ctx.now_ms));
    Incoming::Notice { msg_id, secs, notice }
}

/// Shorter wording for the notices people see most. Others keep Twitch's text. `secs`: the
/// wait Twitch named (slow mode, a timeout). V111-30 A refused send says "Not sent".
fn short_notice(msg_id: &str, secs: Option<i64>) -> Option<String> {
    let wait = secs.filter(|s| *s > 0).map(|s| s as u64);
    Some(match msg_id {
        "msg_duplicate" => "Not sent: same message as before.".into(),
        "msg_ratelimit" => "Not sent: you’re sending too fast.".into(),
        "msg_slowmode" => match wait {
            Some(s) => format!("Not sent: slow mode, wait {s} s."),
            None => "Not sent: slow mode is on.".into(),
        },
        "msg_timedout" => match wait {
            Some(s) => format!("Not sent: you’re timed out for {}.", duration_text(s)),
            None => "Not sent: you’re timed out.".into(),
        },
        "msg_followersonly" | "msg_followersonly_zero" | "msg_followersonly_followed" => {
            "Not sent: this chat is followers-only.".into()
        }
        "msg_emoteonly" => "Not sent: emote-only mode is on.".into(),
        "msg_subsonly" => "Not sent: this chat is subscribers-only.".into(),
        "msg_r9k" => "Not sent: unique-chat mode is on and that message isn’t unique.".into(),
        "msg_rejected" => "Held by AutoMod until a mod checks it.".into(),
        "msg_rejected_mandatory" => "Not sent: the channel’s moderation settings blocked it.".into(),
        "msg_bad_characters" => "Not sent: it has characters Twitch doesn’t allow.".into(),
        "msg_banned" => "You’re banned from this chat.".into(),
        "msg_channel_suspended" => "This channel is suspended.".into(),
        "msg_verified_email" => "Not sent: this chat needs a verified email.".into(),
        "msg_requires_verified_phone_number" => "Not sent: this chat needs a verified phone number.".into(),
        _ => return None,
    })
}

/// "…talk again in 10 seconds." or "…for 600 more seconds." → the number.
pub(crate) fn seconds_in(text: &str) -> Option<i64> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let at = words.iter().position(|w| w.starts_with("second"))?;
    words[at.saturating_sub(2)..at].iter().rev().find_map(|w| w.parse().ok())
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
        let Incoming::Notice { msg_id, notice, .. } =
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

    /// V111-30 What a refused send says, by Twitch's msg-id.
    #[test]
    fn refusals_name_the_reason() {
        let said = |raw: &str| {
            let Incoming::Notice { notice, secs, .. } = run(raw) else { panic!() };
            (text(&notice).to_owned(), secs)
        };
        let n = |id: &str, t: &str| said(&format!("@msg-id={id} :tmi.twitch.tv NOTICE #c :{t}")).0;
        assert_eq!(n("msg_duplicate", "Your message was not sent because it is identical"), "Not sent: same message as before.");
        assert_eq!(n("msg_ratelimit", "Your message was not sent because you are sending messages too quickly."), "Not sent: you’re sending too fast.");
        assert_eq!(
            said("@msg-id=msg_slowmode :tmi.twitch.tv NOTICE #c :This room is in slow mode and you are sending messages too quickly. You will be able to talk again in 3 seconds."),
            ("Not sent: slow mode, wait 3 s.".to_owned(), Some(3))
        );
        assert_eq!(n("msg_slowmode", "Slow mode."), "Not sent: slow mode is on.");
        assert_eq!(n("msg_timedout", "You are timed out for 90 more seconds."), "Not sent: you’re timed out for 1m 30s.");
        assert_eq!(n("msg_followersonly", "This room is in followers-only mode."), "Not sent: this chat is followers-only.");
        assert_eq!(n("msg_followersonly_zero", "x"), "Not sent: this chat is followers-only.");
        assert_eq!(n("msg_subsonly", "x"), "Not sent: this chat is subscribers-only.");
        assert_eq!(n("msg_emoteonly", "x"), "Not sent: emote-only mode is on.");
        assert_eq!(n("msg_rejected", "Your message is being checked by mods and has not been sent."), "Held by AutoMod until a mod checks it.");
        assert_eq!(n("msg_r9k", "x"), "Not sent: unique-chat mode is on and that message isn’t unique.");
        // Unknown ids keep Twitch's words.
        assert_eq!(n("msg_something_new", "Twitch says no."), "Twitch says no.");
    }

    #[test]
    fn durations() {
        assert_eq!(duration_text(90), "1m 30s");
        assert_eq!(duration_text(86_400), "1d");
        assert_eq!(duration_text(1_209_600), "14d");
        assert_eq!(duration_text(3_661), "1h 1m");
    }
}
