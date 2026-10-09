//! PRIVMSG → chat message (1.8, 1.9): replies, rewards, shared chat, power-ups.

use crate::irc::Line;
use crate::model::{ChatMessage, Flag, Fragment, Reply, RewardRef, SharedSource};
use crate::tokenize::{Input, tokenize};

use super::sender::{badges, sender};
use super::{Ctx, timestamp};

pub fn privmsg(line: &Line<'_>, ctx: &mut Ctx<'_>) -> Option<ChatMessage> {
    let raw = line.trailing()?;
    let (text, action) = match raw.strip_prefix("\u{1}ACTION ") {
        Some(rest) => (rest.strip_suffix('\u{1}').unwrap_or(rest), true),
        None => (raw, false),
    };
    let sender = sender(line, ctx.colors)?;
    let me = ctx.me;
    let tokens = tokenize(
        &Input {
            text,
            emotes: line.raw_tag("emotes").unwrap_or(""),
            gifs: line.raw_tag("gifs").unwrap_or(""),
            bits: line.tag_num("bits").unwrap_or(0),
        },
        ctx.me_names(),
        ctx.emotes,
    );
    let mut body = tokens.body;

    let reply = reply(line);
    if let Some(r) = &reply {
        strip_reply_mention(&mut body, &r.parent_login, &r.parent_display);
    }

    let own = me.is_some_and(|m| m.user_id == sender.id);
    let replies_to_me = reply
        .as_ref()
        .zip(me)
        .is_some_and(|(r, m)| r.parent_login.eq_ignore_ascii_case(&m.login));

    let mut flags = Vec::new();
    let mut flag = |on: bool, f: Flag| {
        if on {
            flags.push(f);
        }
    };
    flag(action, Flag::Action);
    flag(own, Flag::Own);
    flag(!own && (tokens.mentions_me || replies_to_me), Flag::Mention);
    flag(line.tag_flag("historical"), Flag::History);
    flag(line.tag_flag("rm-deleted"), Flag::Deleted);
    flag(line.tag_flag("first-msg"), Flag::FirstMessage);
    flag(line.tag_flag("returning-chatter"), Flag::ReturningChatter);
    flag(line.raw_tag("pinned-chat-paid-amount").is_some(), Flag::Paid);
    match line.raw_tag("msg-id") {
        Some("highlighted-message") => flag(true, Flag::Highlighted),
        Some("skip-subs-mode-message") => flag(true, Flag::SkipSubsMode),
        Some("gigantified-emote-message") => flag(true, Flag::Gigantified),
        Some("animated-message") => flag(true, Flag::Animated),
        _ => {}
    }

    let room = line.raw_tag("room-id");
    let source = line
        .tag_nonempty("source-room-id")
        .filter(|s| Some(*s) != room)
        .map(|room_id| SharedSource {
            room_id: room_id.to_owned(),
            badges: badges(line.raw_tag("source-badges"), line.raw_tag("source-badge-info")),
        });

    Some(ChatMessage {
        id: line.tag_nonempty("id").map_or_else(|| ctx.local_id(), str::to_owned),
        ts: timestamp(line, ctx.now_ms),
        sender: Some(sender),
        body,
        notice: None,
        reply,
        reward: line.tag_nonempty("custom-reward-id").map(|id| RewardRef {
            id: id.to_owned(),
            title: None,
            cost: None,
        }),
        source,
        flags,
    })
}

fn reply(line: &Line<'_>) -> Option<Reply> {
    let parent_id = line.tag_nonempty("reply-parent-msg-id")?.to_owned();
    let parent_login = line.tag("reply-parent-user-login").unwrap_or_default().into_owned();
    Some(Reply {
        thread_id: line
            .tag_nonempty("reply-thread-parent-msg-id")
            .map_or_else(|| parent_id.clone(), str::to_owned),
        parent_id,
        parent_display: line
            .tag("reply-parent-display-name")
            .filter(|d| !d.is_empty())
            .map_or_else(|| parent_login.clone(), |d| d.into_owned()),
        parent_body: line.tag("reply-parent-msg-body").unwrap_or_default().into_owned(),
        parent_login,
    })
}

/// Twitch prefixes replies with `@parent `. The reply header already says who it's for.
/// Localized display names aren't mentions to the tokenizer, so those come as text.
fn strip_reply_mention(body: &mut Vec<Fragment>, login: &str, display: &str) {
    match body.first_mut() {
        Some(Fragment::Mention { login: l, .. }) if l.eq_ignore_ascii_case(login) => {
            body.remove(0);
        }
        Some(Fragment::Text { text }) => {
            let prefix = format!("@{display} ");
            if text.starts_with(&prefix) && text.len() > prefix.len() {
                text.drain(..prefix.len());
            }
            return;
        }
        _ => return,
    }
    if let Some(Fragment::Text { text }) = body.first_mut() {
        let trimmed = text.trim_start_matches(' ');
        if trimmed.is_empty() {
            body.remove(0);
        } else {
            *text = trimmed.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Me, sender::Colors};
    use super::*;
    use crate::model::plain;

    fn build(raw: &str, me: Option<&Me>) -> ChatMessage {
        let mut colors = Colors::default();
        let mut ctx = Ctx::new(me, &mut colors, 1_000);
        privmsg(&Line::parse(raw).unwrap(), &mut ctx).unwrap()
    }

    fn me() -> Me {
        Me::new("42", "cyex", "Cyex")
    }

    #[test]
    fn plain_message() {
        let m = build(
            "@badges=;color=#0000FF;display-name=Foo;emotes=;id=m1;room-id=1;tmi-sent-ts=1700000000000;user-id=7 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :hello",
            None,
        );
        assert_eq!((m.id.as_str(), m.ts), ("m1", 1_700_000_000_000));
        assert_eq!(plain(&m.body), "hello");
        assert!(m.flags.is_empty());
        let s = m.sender.unwrap();
        assert_eq!(s.display, "Foo");
        assert_ne!(s.color.dark, "#0000ff", "dark blue gets lifted in dark mode");
    }

    #[test]
    fn action_and_mention() {
        let m = build(
            "@id=m2;user-id=7 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :\u{1}ACTION waves at cyex\u{1}",
            Some(&me()),
        );
        assert_eq!(plain(&m.body), "waves at cyex");
        assert!(m.has(Flag::Action) && m.has(Flag::Mention));
    }

    #[test]
    fn own_messages_never_count_as_mentions() {
        let m = build("@id=m3;user-id=42 :cyex!cyex@cyex.tmi.twitch.tv PRIVMSG #bar :cyex here", Some(&me()));
        assert!(m.has(Flag::Own) && !m.has(Flag::Mention));
    }

    #[test]
    fn reply_drops_the_leading_mention() {
        let m = build(
            r"@id=m4;user-id=7;reply-parent-msg-id=p1;reply-parent-user-login=cyex;reply-parent-display-name=Cyex;reply-parent-msg-body=hi\sall;reply-thread-parent-msg-id=t1 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :@cyex hello back",
            Some(&me()),
        );
        assert_eq!(plain(&m.body), "hello back");
        let r = m.reply.as_ref().unwrap();
        assert_eq!((r.parent_body.as_str(), r.thread_id.as_str()), ("hi all", "t1"));
        assert!(m.has(Flag::Mention), "a reply to me is a mention");
    }

    #[test]
    fn reply_to_a_localized_name() {
        let m = build(
            "@id=m6;user-id=7;reply-parent-msg-id=p1;reply-parent-user-login=sai;reply-parent-display-name=\u{30b5}\u{30a4};reply-parent-msg-body=x :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :@\u{30b5}\u{30a4} hi",
            None,
        );
        assert_eq!(plain(&m.body), "hi");
    }

    #[test]
    fn rewards_power_ups_and_shared_chat() {
        let m = build(
            "@id=m5;user-id=7;room-id=1;source-room-id=2;source-badges=moderator/1;custom-reward-id=r1;msg-id=gigantified-emote-message;first-msg=1 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :x",
            None,
        );
        assert_eq!(m.reward.as_ref().unwrap().id, "r1");
        assert_eq!(m.source.as_ref().unwrap().room_id, "2");
        assert!(m.has(Flag::Gigantified) && m.has(Flag::FirstMessage));
    }
}
