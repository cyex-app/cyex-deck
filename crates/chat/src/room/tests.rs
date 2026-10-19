use super::*;
use crate::model::plain;
use crate::tokenize::NoEmotes;

fn room() -> Room {
    Room::new("Chan", Some(Me::new("42", "me", "Me")))
}

fn feed(r: &mut Room, raw: &str) -> Vec<RoomEvent> {
    r.handle_line(&Line::parse(raw).unwrap(), 1_000, &NoEmotes)
}

fn msg(id: &str, user: &str) -> String {
    format!("@id={id};user-id={user};tmi-sent-ts=5 :u{user}!u@u PRIVMSG #chan :hello {id}")
}

#[test]
fn buffer_is_capped() {
    let mut r = room();
    for i in 0..BUFFER + 10 {
        feed(&mut r, &msg(&i.to_string(), "7"));
    }
    let s = r.snapshot();
    assert_eq!(s.messages.len(), BUFFER);
    assert_eq!(s.messages[0].id, "10");
}

#[test]
fn scrollback_raises_the_cap_but_never_below_the_floor() {
    let mut r = room();
    r.set_keep(1_000);
    for i in 0..1_005 {
        feed(&mut r, &msg(&i.to_string(), "7"));
    }
    assert_eq!(r.snapshot().messages.len(), 1_000);
    r.set_keep(100);
    assert_eq!(r.keep(), BUFFER);
    let s = r.snapshot();
    assert_eq!(s.messages.len(), BUFFER);
    assert_eq!(s.messages[0].id, "505");
}

#[test]
fn link_changes_leave_a_line() {
    let mut r = room();
    let text = |evs: &[RoomEvent]| -> Vec<String> {
        evs.iter()
            .filter_map(|e| match e {
                RoomEvent::Message { message } => message.notice.as_ref().map(|n| n.text.clone()),
                _ => None,
            })
            .collect()
    };
    assert_eq!(text(&r.set_status(ConnStatus::Joined, 1)), ["Joined chat"]);
    assert!(r.set_status(ConnStatus::Joined, 2).is_empty());
    assert_eq!(text(&r.set_status(ConnStatus::Reconnecting, 3)), ["Disconnected"]);
    assert_eq!(text(&r.set_status(ConnStatus::Joined, 4)), ["Reconnected"]);
    let auth = r.set_status(ConnStatus::AuthFailed, 5);
    assert!(matches!(auth.as_slice(), [RoomEvent::Status { status: ConnStatus::AuthFailed }]));
    assert_eq!(r.snapshot().messages.len(), 3);
}

fn notice(id: &str) -> String {
    format!("@msg-id={id} :tmi.twitch.tv NOTICE #chan :You can't join more chats.")
}

fn has_status(ev: &[RoomEvent], want: ConnStatus) -> bool {
    ev.iter().any(|e| matches!(e, RoomEvent::Status { status } if *status == want))
}

#[test]
fn notice_before_join_refuses() {
    let mut r = room();
    let ev = feed(&mut r, &notice("some_new_reason"));
    assert!(has_status(&ev, ConnStatus::Refused));
    assert!(ev.iter().any(|e| matches!(e, RoomEvent::Message { message }
        if message.notice.as_ref().is_some_and(|n| n.text.contains("can't join")))));
    assert_eq!(r.status, ConnStatus::Refused);
}

#[test]
fn suspended_is_unavailable_not_refused() {
    let mut r = room();
    feed(&mut r, &notice("msg_channel_suspended"));
    assert_eq!(r.status, ConnStatus::Unavailable);
    // Review: a second notice keeps it suspended, not refused.
    feed(&mut r, &notice("some_new_reason"));
    assert_eq!(r.status, ConnStatus::Unavailable);
}

#[test]
fn notice_after_join_changes_nothing() {
    let mut r = room();
    r.set_status(ConnStatus::Joined, 0);
    let ev = feed(&mut r, &notice("some_new_reason"));
    assert_eq!(r.status, ConnStatus::Joined);
    assert!(ev.iter().any(|e| matches!(e, RoomEvent::Message { .. })));
}

#[test]
fn send_answer_before_join_is_not_a_refusal() {
    let mut r = room();
    r.sending(PendingSend {
        text: "hi".into(),
        action: false,
        reply: None,
        at_ms: 900,
    });
    feed(&mut r, &notice("msg_duplicate"));
    assert_eq!(r.status, ConnStatus::Connecting);
}

#[test]
fn anonymous_rooms_are_never_refused() {
    let mut r = Room::new("Chan", None);
    feed(&mut r, &notice("some_new_reason"));
    assert_eq!(r.status, ConnStatus::Connecting);
}

#[test]
fn unanswered_join_refuses_only_while_waiting() {
    let mut r = room();
    let ev = r.join_unanswered(5);
    assert!(matches!(ev.as_slice(), [RoomEvent::Status { status: ConnStatus::Refused }]));
    let mut r = room();
    r.set_status(ConnStatus::Joined, 0);
    r.set_status(ConnStatus::Reconnecting, 1);
    assert!(has_status(&r.join_unanswered(5), ConnStatus::Refused));
    let mut r = room();
    r.set_status(ConnStatus::Joined, 0);
    assert!(r.join_unanswered(5).is_empty());
    assert_eq!(r.status, ConnStatus::Joined);
    let mut r = room();
    r.set_status(ConnStatus::Unavailable, 0);
    assert!(r.join_unanswered(5).is_empty());
    assert_eq!(r.status, ConnStatus::Unavailable);
}

#[test]
fn refused_room_joins_later() {
    let mut r = room();
    r.join_unanswered(1);
    let ev = r.set_status(ConnStatus::Joined, 2);
    assert_eq!(r.status, ConnStatus::Joined);
    assert!(ev.iter().any(|e| matches!(e, RoomEvent::Message { message }
        if message.notice.as_ref().is_some_and(|n| n.text == "Joined chat"))));
}

#[test]
fn deletions_and_timeouts_mark_messages() {
    let mut r = room();
    feed(&mut r, &msg("a", "7"));
    feed(&mut r, &msg("b", "7"));
    feed(&mut r, &msg("c", "8"));
    feed(&mut r, "@target-msg-id=c :tmi.twitch.tv CLEARMSG #chan :hello c");
    let ev = feed(&mut r, "@ban-duration=60;target-user-id=7 :tmi.twitch.tv CLEARCHAT #chan :u7");
    assert!(matches!(&ev[0], RoomEvent::ClearUser { user_id } if user_id == "7"));
    let s = r.snapshot();
    assert!(s.messages[..3].iter().all(|m| m.has(Flag::Deleted)));
    assert_eq!(s.messages[3].notice.as_ref().unwrap().kind, NoticeKind::Moderation);
}

#[test]
fn own_messages_are_echoed_from_userstate() {
    let mut r = room();
    feed(&mut r, "@badges=moderator/1;mod=1 :tmi.twitch.tv USERSTATE #chan");
    assert!(r.privileged());
    r.sending(PendingSend {
        text: "hi there".into(),
        action: false,
        reply: None,
        at_ms: 900,
    });
    let ev = feed(&mut r, "@badges=moderator/1;color=#FF0000;display-name=Me;id=sent1;mod=1 :tmi.twitch.tv USERSTATE #chan");
    let RoomEvent::Message { message } = &ev[0] else { panic!("{ev:?}") };
    assert_eq!((message.id.as_str(), plain(&message.body).as_str()), ("sent1", "hi there"));
    assert!(message.has(Flag::Own));
    assert_eq!(message.sender.as_ref().unwrap().badges[0].set, "moderator");
}

/// V105-06 A room whose rules highlight first messages.
fn first_room() -> Room {
    use crate::rules::Rules;
    use cyex_settings::schema::Highlights;
    let mut r = room();
    r.set_rules(Arc::new(Rules::new(&Highlights::default())));
    r
}

fn sent(r: &mut Room) {
    r.sending(PendingSend {
        text: "hi".into(),
        action: false,
        reply: None,
        at_ms: 900,
    });
}

/// Only the events that add or change a line.
fn lines(ev: Vec<RoomEvent>) -> Vec<RoomEvent> {
    ev.into_iter()
        .filter(|e| matches!(e, RoomEvent::Message { .. } | RoomEvent::Update { .. }))
        .collect()
}

const US: &str = "@badges=;color=#FF0000;display-name=Me;id=s1;mod=0 :tmi.twitch.tv USERSTATE #chan";
const RELAYED: &str = "@badge-info=;badges=;color=#FF0000;display-name=Me;first-msg=1;id=s1;room-id=1;tmi-sent-ts=1;user-id=42;mod=0 :me!me@me.tmi.twitch.tv PRIVMSG #chan :hi";

#[test]
fn relayed_own_line_replaces_the_echo() {
    use crate::model::HighlightKind;
    let mut r = first_room();
    sent(&mut r);
    let ev = lines(feed(&mut r, US));
    assert!(matches!(ev.as_slice(), [RoomEvent::Message { .. }]), "{ev:?}");
    let ev = lines(feed(&mut r, RELAYED));
    let [RoomEvent::Update { message }] = ev.as_slice() else { panic!("{ev:?}") };
    assert!(message.has(Flag::FirstMessage) && message.has(Flag::Own));
    assert_eq!(message.meta.highlight.as_ref().map(|h| h.kind), Some(HighlightKind::FirstMessage));
    let s = r.snapshot();
    assert_eq!(s.messages.iter().filter(|m| m.id == "s1").count(), 1);
}

#[test]
fn reader_first_then_userstate_adds_one_line() {
    let mut r = room();
    feed(&mut r, "@slow=10;room-id=1 :tmi.twitch.tv ROOMSTATE #chan");
    sent(&mut r);
    let ev = lines(feed(&mut r, RELAYED));
    assert!(matches!(ev.as_slice(), [RoomEvent::Message { .. }]), "{ev:?}");
    let ev = feed(&mut r, US);
    assert!(ev.iter().all(|e| !matches!(e, RoomEvent::Message { .. })), "{ev:?}");
    assert!(r.state.wait_until.is_some(), "slow mode wait still set");
    assert_eq!(r.snapshot().messages.iter().filter(|m| m.id == "s1").count(), 1);
}

#[test]
fn own_line_from_another_app_is_added() {
    let mut r = room();
    let ev = lines(feed(&mut r, RELAYED));
    assert!(matches!(ev.as_slice(), [RoomEvent::Message { .. }]), "{ev:?}");
}

#[test]
fn upgrade_keeps_deleted() {
    let mut r = first_room();
    sent(&mut r);
    feed(&mut r, US);
    feed(&mut r, "@target-msg-id=s1 :tmi.twitch.tv CLEARMSG #chan :hi");
    let ev = lines(feed(&mut r, RELAYED));
    let [RoomEvent::Update { message }] = ev.as_slice() else { panic!("{ev:?}") };
    assert!(message.has(Flag::Deleted));
}

#[test]
fn reply_to_a_reply_keeps_the_clicked_parent() {
    let mut r = room();
    feed(&mut r, "@id=r;user-id=7;tmi-sent-ts=5 :u7!u@u PRIVMSG #chan :root");
    feed(&mut r, "@id=c;user-id=8;tmi-sent-ts=6;reply-parent-msg-id=r;reply-thread-parent-msg-id=r;reply-parent-user-id=7;reply-parent-user-login=u7;reply-parent-display-name=u7;reply-parent-msg-body=root :u8!u@u PRIVMSG #chan :@u7 a");
    feed(&mut r, "@id=d;user-id=9;tmi-sent-ts=7;reply-parent-msg-id=c;reply-thread-parent-msg-id=r;reply-parent-user-id=8;reply-parent-user-login=u8;reply-parent-display-name=u8;reply-parent-msg-body=a :u9!u@u PRIVMSG #chan :@u8 b");
    let ids = |id: &str| {
        let x = r.reply_to(id).unwrap();
        (x.parent_id, x.thread_id)
    };
    assert_eq!(ids("d"), ("d".to_string(), "r".to_string()));
    assert_eq!(ids("c"), ("c".to_string(), "r".to_string()));
    assert_eq!(ids("r").1, "r");
}

#[test]
fn a_refused_send_is_dropped_and_explained() {
    let mut r = room();
    r.sending(PendingSend {
        text: "same".into(),
        action: false,
        reply: None,
        at_ms: 900,
    });
    let ev = feed(&mut r, "@msg-id=msg_duplicate :tmi.twitch.tv NOTICE #chan :Your message was not sent");
    assert_eq!(ev.len(), 1);
    // A later USERSTATE with an id has nothing to echo.
    let ev = feed(&mut r, "@id=x :tmi.twitch.tv USERSTATE #chan");
    assert!(ev.iter().all(|e| !matches!(e, RoomEvent::Message { .. })));
}

#[test]
fn history_goes_first_without_duplicates() {
    let mut r = room();
    feed(&mut r, &msg("live", "7"));
    let lines = [
        msg("h1", "7"),
        msg("h2", "8"),
        msg("live", "7"),
        "@ban-duration=5;target-user-id=8 :tmi.twitch.tv CLEARCHAT #chan :u8".to_owned(),
    ];
    let added = r.history(&lines, 1_000, &NoEmotes);
    assert_eq!(added.len(), 3);
    let ids: Vec<_> = r.snapshot().messages.iter().map(|m| m.id.clone()).collect();
    assert_eq!(ids[..2], ["h1", "h2"]);
    assert_eq!(ids.last().unwrap(), "live");
    let s = r.snapshot();
    assert!(s.messages[1].has(Flag::Deleted) && s.messages[1].has(Flag::History));
    assert!(!s.messages[0].has(Flag::Deleted));
}

#[test]
fn gift_subs_merge_into_their_mass_gift() {
    let mut r = room();
    let mass = r"@id=g0;login=a;msg-id=submysterygift;msg-param-mass-gift-count=2;msg-param-origin-id=77;system-msg=A\sis\sgifting\s2;user-id=9 :tmi.twitch.tv USERNOTICE #chan";
    let one = |n: &str| {
        format!(
            r"@id=g{n};login=a;msg-id=subgift;msg-param-community-gift-id=77;msg-param-origin-id=77;msg-param-recipient-display-name=R{n};system-msg=A\sgifted;user-id=9 :tmi.twitch.tv USERNOTICE #chan"
        )
    };
    feed(&mut r, mass);
    let ev = feed(&mut r, &one("1"));
    assert!(
        matches!(&ev[0], RoomEvent::Update { message } if message.notice.as_ref().unwrap().recipients == ["R1"])
    );
    feed(&mut r, &one("2"));
    // The group is full: a third one stands alone.
    let ev = feed(&mut r, &one("3"));
    assert!(matches!(&ev[0], RoomEvent::Message { .. }));
    let s = r.snapshot();
    assert_eq!(s.messages.len(), 2);
    assert_eq!(s.messages[0].notice.as_ref().unwrap().recipients, ["R1", "R2"]);
}

#[test]
fn cooldowns_from_slow_mode_and_timeouts() {
    let mut r = room();
    feed(&mut r, "@slow=10;room-id=1 :tmi.twitch.tv ROOMSTATE #chan");
    r.sending(PendingSend {
        text: "x".into(),
        action: false,
        reply: None,
        at_ms: 1_000,
    });
    feed(&mut r, "@id=s1 :tmi.twitch.tv USERSTATE #chan");
    assert_eq!(r.state.wait_until, Some(11_000));
    feed(
        &mut r,
        "@msg-id=msg_timedout :tmi.twitch.tv NOTICE #chan :You are timed out for 600 more seconds.",
    );
    assert_eq!(r.state.wait_until, Some(601_000));
    feed(&mut r, "@ban-duration=60;target-user-id=42 :tmi.twitch.tv CLEARCHAT #chan :me");
    assert_eq!(r.state.wait_until, Some(601_000), "never moves back");
    assert_eq!(seconds_in("You will be able to talk again in 3 seconds."), Some(3));
}

#[test]
fn redemptions_get_their_title_and_replies_their_header() {
    let mut r = room();
    r.set_rewards(HashMap::from([("r1".to_owned(), ("Hydrate".to_owned(), 500))]));
    feed(&mut r, "@id=m1;user-id=7;custom-reward-id=r1 :u!u@u PRIVMSG #chan :drink");
    let s = r.snapshot();
    let reward = s.messages[0].reward.as_ref().unwrap();
    assert_eq!((reward.title.as_deref(), reward.cost), (Some("Hydrate"), Some(500)));
    let reply = r.reply_to("m1").unwrap();
    assert_eq!((reply.parent_body.as_str(), reply.thread_id.as_str()), ("drink", "m1"));
    assert!(r.reply_to("nope").is_none());
}

#[test]
fn recent_messages_of_a_user() {
    let mut r = room();
    for i in 0..15 {
        feed(&mut r, &msg(&format!("m{i}"), if i % 2 == 0 { "7" } else { "8" }));
    }
    let last = r.recent_from("7", 3);
    assert_eq!(last.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["m10", "m12", "m14"]);
}

#[test]
fn the_first_pin_read_is_news_even_with_nothing_pinned() {
    let mut r = room();
    assert!(!r.snapshot().pin_known);
    assert!(r.set_pinned(None).is_some(), "V103-04 first read");
    assert!(r.snapshot().pin_known);
    assert!(r.set_pinned(None).is_none(), "unchanged");
}

#[test]
fn a_new_pin_replaces_the_old_one() {
    let mut r = room();
    feed(&mut r, &msg("a", "7"));
    feed(&mut r, &msg("b", "8"));
    let pin = |id: &str, n: usize| Pinned {
        id: id.into(),
        by: Some("Mod".into()),
        starts_at: None,
        ends_at: Some("2026-10-19T16:30:00Z".into()),
        paid: false,
        message: (*r.snapshot().messages[n]).clone(),
    };
    let (a, b) = (pin("p1", 0), pin("p2", 1));
    r.set_pinned(Some(a));
    let ev = r.set_pinned(Some(b.clone())).expect("V107 the second pin is news");
    assert!(matches!(ev, RoomEvent::Pinned { pin: Some(p) } if p.id == "p2" && p.message.id == "b"));
    assert_eq!(r.snapshot().pinned, Some(b), "only the new pin is kept");
    // V107 A delete reads the pin again only when it hit the pinned message.
    assert!(r.pins("b") && !r.pins("a"));
    r.set_pinned(None);
    assert!(!r.pins("b"));
}

#[test]
fn shared_rooms_are_the_recent_source_rooms() {
    let mut r = room();
    let shared = |id: &str, src: &str, ts: i64| {
        format!("@id={id};user-id=7;room-id=1;source-room-id={src};tmi-sent-ts={ts} :u!u@u PRIVMSG #chan :hi")
    };
    feed(&mut r, &shared("a", "2", 100));
    feed(&mut r, &shared("b", "3", 500));
    feed(&mut r, &shared("c", "1", 600));
    feed(&mut r, &shared("d", "3", 700));
    // Newest line, no source room (the buffer is time-ordered).
    feed(&mut r, "@id=e;user-id=7;tmi-sent-ts=800 :u7!u@u PRIVMSG #chan :hello e");
    assert_eq!(r.shared_rooms(0), ["3", "2"]);
    assert_eq!(r.shared_rooms(400), ["3"]);
    assert!(r.shared_rooms(800).is_empty());
}
