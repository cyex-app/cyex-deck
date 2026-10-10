//! USERNOTICE → notice line (1.9): subs, gifts, raids, announcements, milestones.
//! Twitch's `system-msg` is the line's text; the user's own message, if any, is the body.

use crate::irc::Line;
use crate::model::{ChatMessage, Flag, Meta, Notice, NoticeKind, SharedSource};
use crate::tokenize::{Input, tokenize};

use super::sender::{badges, sender};
use super::{Ctx, GiftLink, Incoming, timestamp};

pub fn usernotice(line: &Line<'_>, ctx: &mut Ctx<'_>) -> Option<Incoming> {
    // Shared chat wraps the original notice; its msg-param-* describe the inner one.
    let msg_id = match line.raw_tag("msg-id")? {
        "sharedchatnotice" => line.raw_tag("source-msg-id").unwrap_or("sharedchatnotice"),
        id => id,
    };
    let kind = kind_of(msg_id);
    let mut text = line.tag("system-msg").unwrap_or_default().trim().to_owned();
    if text.is_empty() {
        text = fallback_text(msg_id, line);
    } else if msg_id == "modiversary" {
        // Twitch leaves the name out of this one.
        text = format!("{} {text}", who(line));
    }
    let mut notice = Notice::new(kind, text);
    if msg_id == "announcement" {
        notice.color = line
            .tag_nonempty("msg-param-color")
            .map(|c| c.to_ascii_lowercase());
    }

    let tokens = line.trailing().map(|text| {
        tokenize(
            &Input {
                text,
                sender: line.raw_tag("user-id").unwrap_or(""),
                emotes: line.raw_tag("emotes").unwrap_or(""),
                gifs: "",
                bits: 0,
            },
            ctx.me_names(),
            ctx.emotes,
        )
    });
    let sender = sender(line, ctx.colors);
    let mut flags = Vec::new();
    if line.tag_flag("historical") {
        flags.push(Flag::History);
    }
    if line.tag_flag("rm-deleted") {
        flags.push(Flag::Deleted);
    }
    if tokens.as_ref().is_some_and(|t| t.mentions_me) {
        flags.push(Flag::Mention);
    }
    let room = line.raw_tag("room-id");
    let source = line
        .tag_nonempty("source-room-id")
        .filter(|s| Some(*s) != room)
        .map(|room_id| SharedSource {
            room_id: room_id.to_owned(),
            badges: badges(line.raw_tag("source-badges"), line.raw_tag("source-badge-info")),
        });
    let message = ChatMessage {
        id: line.tag_nonempty("id").map_or_else(|| ctx.local_id(), str::to_owned),
        ts: timestamp(line, ctx.now_ms),
        sender,
        body: tokens.map(|t| t.body).unwrap_or_default(),
        notice: Some(notice),
        reply: None,
        reward: None,
        source,
        flags,
        meta: Meta::default(),
    };
    let param = |key: &str| {
        line.tag(&format!("msg-param-{key}"))
            .filter(|v| !v.is_empty())
            .map(|v| v.into_owned())
    };
    let gift = match msg_id {
        "submysterygift" => param("origin-id").map(|origin| GiftLink::Mass {
            origin,
            count: param("mass-gift-count").and_then(|c| c.parse().ok()).unwrap_or(1),
        }),
        "subgift" => param("recipient-display-name")
            .or_else(|| param("recipient-user-name"))
            .map(|recipient| GiftLink::Single {
                community: param("community-gift-id"),
                recipient,
            }),
        _ => None,
    };
    Some(match gift {
        Some(gift) => Incoming::Gift { message, gift },
        None => Incoming::Message(message),
    })
}

fn who(line: &Line<'_>) -> String {
    line.tag("display-name")
        .filter(|d| !d.is_empty())
        .map(|d| d.into_owned())
        .unwrap_or_else(|| line.tag_nonempty("login").unwrap_or("Someone").to_owned())
}

fn kind_of(msg_id: &str) -> NoticeKind {
    match msg_id {
        "sub" | "resub" | "primepaidupgrade" | "giftpaidupgrade" | "anongiftpaidupgrade" => NoticeKind::Sub,
        "subgift" | "anonsubgift" | "submysterygift" | "anonsubmysterygift" | "communitypayforward"
        | "standardpayforward" | "rewardgift" | "onetapgiftredeemed" => NoticeKind::Gift,
        "raid" | "unraid" => NoticeKind::Raid,
        "announcement" => NoticeKind::Announcement,
        "bitsbadgetier" | "charitydonation" => NoticeKind::Bits,
        "viewermilestone" | "ritual" | "modiversary" | "socialsharingbadge" => NoticeKind::Milestone,
        _ => NoticeKind::Info,
    }
}

fn fallback_text(msg_id: &str, line: &Line<'_>) -> String {
    let who = who(line);
    match msg_id {
        "announcement" => "Announcement".into(),
        "raid" => format!(
            "{who} is raiding with {} viewers.",
            line.tag_nonempty("msg-param-viewerCount").unwrap_or("some")
        ),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Colors, Incoming, build};
    use super::*;
    use crate::model::plain;

    fn run_any(raw: &str) -> Incoming {
        let mut colors = Colors::default();
        let mut ctx = Ctx::new(None, &mut colors, 0);
        build(&Line::parse(raw).unwrap(), &mut ctx).unwrap()
    }

    fn run(raw: &str) -> ChatMessage {
        match run_any(raw) {
            Incoming::Message(m) => m,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn resub_with_message() {
        let m = run(
            r"@badges=subscriber/12;display-name=Foo;id=n1;login=foo;msg-id=resub;msg-param-cumulative-months=12;system-msg=Foo\ssubscribed\sat\sTier\s1.\sThey've\ssubscribed\sfor\s12\smonths!;tmi-sent-ts=5;user-id=7 :tmi.twitch.tv USERNOTICE #c :love this stream",
        );
        let n = m.notice.as_ref().unwrap();
        assert_eq!(n.kind, NoticeKind::Sub);
        assert!(n.text.starts_with("Foo subscribed at Tier 1."));
        assert_eq!(plain(&m.body), "love this stream");
        assert_eq!(m.sender.unwrap().login, "foo");
    }

    #[test]
    fn announcement_has_color_and_body() {
        let m = run(
            "@display-name=Mod;id=n2;login=mod;msg-id=announcement;msg-param-color=PURPLE;system-msg=;user-id=8 :tmi.twitch.tv USERNOTICE #c :Giveaway at 8",
        );
        let n = m.notice.unwrap();
        assert_eq!((n.kind, n.text.as_str(), n.color.as_deref()), (NoticeKind::Announcement, "Announcement", Some("purple")));
    }

    #[test]
    fn shared_chat_uses_the_inner_kind() {
        let m = run(
            r"@id=n3;login=a;msg-id=sharedchatnotice;source-msg-id=raid;room-id=1;source-room-id=2;system-msg=5\sraiders\sfrom\sA\shave\sjoined!;user-id=9 :tmi.twitch.tv USERNOTICE #c",
        );
        assert_eq!(m.source.as_ref().unwrap().room_id, "2");
        assert_eq!(m.notice.unwrap().kind, NoticeKind::Raid);
        assert!(m.body.is_empty());
    }

    #[test]
    fn gifts_carry_their_links() {
        let Incoming::Gift { gift, .. } = run_any(
            r"@id=g0;login=a;msg-id=submysterygift;msg-param-mass-gift-count=5;msg-param-origin-id=777;system-msg=A\sis\sgifting\s5;user-id=9 :tmi.twitch.tv USERNOTICE #c",
        ) else {
            panic!()
        };
        assert_eq!(
            gift,
            GiftLink::Mass {
                origin: "777".into(),
                count: 5
            }
        );
        let Incoming::Gift { gift, .. } = run_any(
            r"@id=g1;login=a;msg-id=subgift;msg-param-community-gift-id=777;msg-param-origin-id=777;msg-param-recipient-display-name=Bob;system-msg=A\sgifted;user-id=9 :tmi.twitch.tv USERNOTICE #c",
        ) else {
            panic!()
        };
        assert_eq!(
            gift,
            GiftLink::Single {
                community: Some("777".into()),
                recipient: "Bob".into()
            }
        );
    }

    #[test]
    fn modiversary_gets_the_name() {
        let m = run(
            r"@display-name=Mod;id=n4;login=mod;msg-id=modiversary;msg-param-months=12;system-msg=has\sbeen\sa\smoderator\sfor\s12\smonths!;user-id=8 :tmi.twitch.tv USERNOTICE #c",
        );
        assert_eq!(m.notice.unwrap().text, "Mod has been a moderator for 12 months!");
    }
}
