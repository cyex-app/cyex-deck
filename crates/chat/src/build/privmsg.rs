//! PRIVMSG → chat message (1.8, 1.9): replies, rewards, shared chat, power-ups, the
//! app it came from (C26).

use crate::irc::Line;
use crate::model::{ChatMessage, Client, Flag, Fragment, Gone, GoneHow, Meta, Reply, RewardRef, SharedSource};
use crate::tokenize::{Input, is_me, tokenize};

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
            sender: &sender.id,
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
    // V106-33 A reply starts with `@you`: that alone is the reply, not your name.
    let names_me = tokens.mentions_me && (!replies_to_me || names_in(&body, ctx.me_names()));

    let mut flags = Vec::new();
    let mut flag = |on: bool, f: Flag| {
        if on {
            flags.push(f);
        }
    };
    flag(action, Flag::Action);
    flag(own, Flag::Own);
    flag(!own && (tokens.mentions_me || replies_to_me), Flag::Mention);
    // V106-33 Which one, for the Your name and Replies rows (`Rules::highlight` settles `Mention`).
    flag(!own && names_me, Flag::NamesMe);
    flag(!own && replies_to_me, Flag::RepliesToMe);
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
        reward: reward_of(line),
        source,
        flags,
        meta: Meta {
            // The deck's own nonce looks like the website's: our own lines name no app.
            client: (!own).then(|| Client::from_nonce(line.raw_tag("client-nonce").unwrap_or(""))).flatten(),
            effect: (line.raw_tag("msg-id") == Some("animated-message"))
                .then(|| line.tag_nonempty("animation-id").map(str::to_owned))
                .flatten(),
            // V107-R34 A history line that was deleted; a CLEARCHAT after it says more.
            gone: line.tag_flag("rm-deleted").then(|| Gone::new(GoneHow::Deleted, None)),
            ..Meta::default()
        },
    })
}

/// Id the built-in highlight reward goes by in `RewardRef` (and the room's reward map,
/// filled in `app/src/chat/catalog.rs`).
const HIGHLIGHT_REWARD: &str = "highlighted-message";

/// V108-C28 A custom reward by its id (name and cost come from the room); a points
/// highlight has no id tag, so it gets the built-in one, cost filled in by the room.
fn reward_of(line: &Line<'_>) -> Option<RewardRef> {
    if let Some(id) = line.tag_nonempty("custom-reward-id") {
        return Some(RewardRef {
            id: id.to_owned(),
            title: None,
            cost: None,
        });
    }
    (line.raw_tag("msg-id") == Some(HIGHLIGHT_REWARD)).then(|| RewardRef {
        id: HIGHLIGHT_REWARD.to_owned(),
        title: Some("Highlight My Message".to_owned()),
        cost: None,
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
/// V106-33 Your name in the body, the way the tokenizer finds it (`@login` or the word).
/// Only run on replies to you that named you, so rare.
fn names_in(body: &[Fragment], names: &[String]) -> bool {
    body.iter().any(|f| match f {
        Fragment::Mention { login, .. } => is_me(names, login),
        Fragment::Text { text } => text.split_whitespace().any(|w| is_me(names, w)),
        _ => false,
    })
}

/// V110-F05 Drops the leading `@parent` of a reply (login or display name, any case, then a
/// space or the end) so only the content shows. A later mention stays. A body that is only
/// the mention is kept as it is, so the line isn't blank.
fn strip_reply_mention(body: &mut Vec<Fragment>, login: &str, display: &str) {
    let names = [login, display];
    let hit = |t: &str, n: &str| !n.is_empty() && t.strip_prefix('@').is_some_and(|r| r.to_lowercase() == n.to_lowercase());
    let mut next = body.clone();
    match next.first_mut() {
        Some(Fragment::Mention { text, .. }) if names.iter().any(|n| hit(text, n)) => {
            next.remove(0);
        }
        Some(Fragment::Text { text }) => {
            let cut = names.iter().filter(|n| !n.is_empty()).find_map(|n| {
                let end = 1 + n.len();
                let head = text.get(..end)?;
                let rest = &text[end..];
                (hit(head, n) && (rest.is_empty() || rest.starts_with(' '))).then_some(end)
            });
            let Some(end) = cut else { return };
            text.drain(..end);
        }
        _ => return,
    }
    if let Some(Fragment::Text { text }) = next.first_mut() {
        let trimmed = text.trim_start_matches(' ');
        if trimmed.is_empty() {
            next.remove(0);
        } else {
            *text = trimmed.to_owned();
        }
    }
    if !next.is_empty() {
        *body = next;
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

    fn strip(text: &str, login: &str, display: &str) -> String {
        let mut body = tokenize(&Input { text, sender: "7", emotes: "", gifs: "", bits: 0 }, &[], &crate::tokenize::NoEmotes).body;
        strip_reply_mention(&mut body, login, display);
        plain(&body)
    }

    #[test]
    fn strip_cases() {
        assert_eq!(strip("@sai hi there", "sai", "Sai"), "hi there", "login");
        assert_eq!(strip("@Sai hi", "sai", "Sai"), "hi", "display name");
        assert_eq!(strip("@SAI hi", "sai", "Sai"), "hi", "case");
        assert_eq!(strip("@\u{79c1}\u{306f} hi", "watashi", "\u{79c1}\u{306f}"), "hi", "CJK display");
        assert_eq!(strip("hi @sai", "sai", "Sai"), "hi @sai", "not at the start");
        assert_eq!(strip("@bob @sai hi", "sai", "Sai"), "@bob @sai hi", "another user");
        assert_eq!(strip("@sai hi @sai", "sai", "Sai"), "hi @sai", "later mention stays");
        assert_eq!(strip("@sai", "sai", "Sai"), "@sai", "only the mention is kept");
        assert_eq!(strip("@saint hi", "sai", "Sai"), "@saint hi", "longer name");
    }

    #[test]
    fn reply_with_only_an_emote_after_the_mention() {
        let m = build(
            "@id=m9;user-id=7;emotes=25:4-8;reply-parent-msg-id=p1;reply-parent-user-login=sai;reply-parent-display-name=\u{30b5}\u{30a4};reply-parent-msg-body=x :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :@\u{30b5}\u{30a4} Kappa",
            None,
        );
        assert!(matches!(m.body.first(), Some(Fragment::Emote { .. })), "{:?}", m.body);
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
        assert!(m.has(Flag::NamesMe) && !m.has(Flag::RepliesToMe));
    }

    #[test]
    fn own_messages_never_count_as_mentions() {
        let m = build("@id=m3;user-id=42 :cyex!cyex@cyex.tmi.twitch.tv PRIVMSG #bar :cyex here", Some(&me()));
        assert!(m.has(Flag::Own) && !m.has(Flag::Mention));
    }

    #[test]
    fn own_messages_name_no_app() {
        let nonce = "client-nonce=0123456789abcdef0123456789abcdef";
        let m = build(&format!("@id=m5;{nonce};user-id=42 :cyex!cyex@cyex.tmi.twitch.tv PRIVMSG #bar :hi"), Some(&me()));
        assert_eq!(m.meta.client, None);
        let m = build(&format!("@id=m6;{nonce};user-id=7 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :hi"), Some(&me()));
        assert_eq!(m.meta.client, Some(Client::Web));
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
        assert!(m.has(Flag::RepliesToMe) && !m.has(Flag::NamesMe), "the @ of a reply isn't your name");
        let named = build(
            r"@id=m5;user-id=7;reply-parent-msg-id=p1;reply-parent-user-login=cyex;reply-parent-display-name=Cyex;reply-parent-msg-body=hi;reply-thread-parent-msg-id=t1 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :@cyex hey cyex!",
            Some(&me()),
        );
        assert!(named.has(Flag::RepliesToMe) && named.has(Flag::NamesMe));
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
        let m = build("@id=m6;user-id=7;msg-id=animated-message;animation-id=simmer :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :hey", None);
        assert_eq!(m.meta.effect.as_deref(), Some("simmer"));
        assert!(m.reward.is_none(), "power-ups cost bits, not a points line");
    }

    #[test]
    fn points_highlight_is_a_reward_line() {
        let m = build("@id=m7;user-id=7;msg-id=highlighted-message :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :look", None);
        assert!(m.has(Flag::Highlighted));
        let r = m.reward.as_ref().unwrap();
        assert_eq!((r.title.as_deref(), r.cost), (Some("Highlight My Message"), None));
        let m = build("@id=m8;user-id=7 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :plain", None);
        assert!(m.reward.is_none());
    }
}
