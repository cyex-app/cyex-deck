//! 1.57, V102-21 Our role in a room, kept current from everything Twitch tells us:
//! USERSTATE (on join and after each send), our own lines sent from elsewhere, and
//! outside news the app passes in (the moderated-channel check, mod and unmod events).
//! The channel's own account is its broadcaster from the start.

use super::Room;
use crate::model::{Badge, MOD_SETS, ModRole, RoomEvent, SelfState};

impl Room {
    /// The room is the account's own channel.
    pub fn owner(&self) -> bool {
        self.me
            .as_ref()
            .is_some_and(|m| m.login.eq_ignore_ascii_case(&self.login))
    }

    /// What the account may do here; `None` when anonymous or not a mod.
    pub fn role(&self) -> Option<ModRole> {
        self.state.own.as_ref().and_then(|o| o.role)
    }

    /// The broadcaster's own room knows its role before Twitch says anything.
    pub(super) fn initial_own(&self) -> Option<SelfState> {
        let me = self.me.as_ref()?;
        self.owner()
            .then(|| SelfState::new(&me.login, Vec::new(), false, true))
    }

    /// Sets what we know about ourselves; an event when it changed.
    pub(super) fn set_own(&mut self, own: SelfState) -> Option<RoomEvent> {
        (self.state.own.as_ref() != Some(&own)).then(|| {
            self.state.own = Some(own);
            RoomEvent::State {
                state: self.state.clone(),
            }
        })
    }

    /// Our own line, sent from another app: its badges are as fresh as a USERSTATE.
    pub(super) fn own_line(&mut self, badges: &[Badge]) -> Option<RoomEvent> {
        let me = self.me.as_ref()?;
        let own = SelfState::new(&me.login, badges.to_vec(), false, self.owner());
        self.set_own(own)
    }

    /// Twitch says the account is a mod here now, or not anymore (the moderated-channel
    /// check, a mod or unmod event, a revoked mod subscription). A lead moderator stays
    /// one when told "mod"; the broadcaster is never changed.
    pub fn set_moderator(&mut self, on: bool) -> Option<RoomEvent> {
        let me = self.me.as_ref()?;
        if self.owner() || self.role().is_some() == on {
            return None;
        }
        let mut own = self
            .state
            .own
            .clone()
            .unwrap_or_else(|| SelfState::new(&me.login, Vec::new(), false, false));
        if on {
            own.role = Some(ModRole::Moderator);
        } else {
            own.badges.retain(|b| !MOD_SETS.contains(&b.set.as_str()));
            own.role = None;
        }
        self.set_own(own)
    }
}

#[cfg(test)]
mod tests {
    use crate::build::Me;
    use crate::irc::Line;
    use crate::model::{ModRole, RoomEvent};
    use crate::room::Room;
    use crate::tokenize::NoEmotes;

    fn feed(r: &mut Room, raw: &str) -> Vec<RoomEvent> {
        r.handle_line(&Line::parse(raw).unwrap(), 0, &NoEmotes)
    }

    fn room(channel: &str) -> Room {
        Room::new(channel, Some(Me::new("1", "me", "Me")))
    }

    #[test]
    fn userstate_sets_each_role() {
        for (badges, role) in [
            ("", None),
            ("vip/1", None),
            ("moderator/1", Some(ModRole::Moderator)),
            ("lead_moderator/1", Some(ModRole::LeadModerator)),
        ] {
            let mut r = room("chan");
            feed(
                &mut r,
                &format!("@badges={badges};mod=0 :tmi.twitch.tv USERSTATE #chan"),
            );
            assert_eq!(r.role(), role, "{badges}");
            assert_eq!(r.privileged(), role.is_some());
        }
    }

    #[test]
    fn own_channel_is_broadcaster_from_the_start() {
        let mut r = room("me");
        assert_eq!(r.role(), Some(ModRole::Broadcaster));
        feed(&mut r, "@badges=;mod=0 :tmi.twitch.tv USERSTATE #me");
        assert_eq!(r.role(), Some(ModRole::Broadcaster));
        // Nothing outside takes it away.
        assert!(r.set_moderator(false).is_none());
        assert_eq!(r.role(), Some(ModRole::Broadcaster));
        // Anonymous rooms have no role at all.
        assert_eq!(Room::new("me", None).role(), None);
    }

    #[test]
    fn modded_and_unmodded_while_open() {
        let mut r = room("chan");
        feed(
            &mut r,
            "@badges=subscriber/12;mod=0 :tmi.twitch.tv USERSTATE #chan",
        );
        assert_eq!(r.role(), None);
        // Modded: one state event, the role shows, the sub badge stays.
        let ev = r.set_moderator(true);
        assert!(matches!(ev, Some(RoomEvent::State { .. })));
        assert_eq!(r.role(), Some(ModRole::Moderator));
        assert!(r.set_moderator(true).is_none(), "no event without a change");
        // The next USERSTATE agrees and brings the badge.
        feed(
            &mut r,
            "@badges=moderator/1,subscriber/12;mod=1 :tmi.twitch.tv USERSTATE #chan",
        );
        assert_eq!(r.role(), Some(ModRole::Moderator));
        // Unmodded: the mod badge goes, the rest stays.
        assert!(r.set_moderator(false).is_some());
        assert_eq!(r.role(), None);
        let own = r.state.own.as_ref().unwrap();
        assert_eq!(
            own.badges.iter().map(|b| b.set.as_str()).collect::<Vec<_>>(),
            ["subscriber"]
        );
    }

    #[test]
    fn lead_moderators_stay_lead_and_can_lose_it() {
        let mut r = room("chan");
        feed(
            &mut r,
            "@badges=lead_moderator/1;mod=1 :tmi.twitch.tv USERSTATE #chan",
        );
        assert!(r.set_moderator(true).is_none());
        assert_eq!(r.role(), Some(ModRole::LeadModerator));
        assert!(r.set_moderator(false).is_some());
        assert_eq!(r.role(), None);
        // Promoted to lead mod, seen on a line we sent from another app.
        feed(&mut r, "@badges=moderator/1;mod=1 :tmi.twitch.tv USERSTATE #chan");
        feed(
            &mut r,
            "@badges=lead_moderator/1;display-name=Me;id=a;mod=1;room-id=9;tmi-sent-ts=1;user-id=1 :me!me@me.tmi.twitch.tv PRIVMSG #chan :hi",
        );
        assert_eq!(r.role(), Some(ModRole::LeadModerator));
    }

    #[test]
    fn a_role_before_userstate_comes_from_outside() {
        let mut r = room("chan");
        assert_eq!(r.role(), None);
        assert!(r.set_moderator(true).is_some());
        assert_eq!(r.role(), Some(ModRole::Moderator));
        assert_eq!(r.state.own.as_ref().unwrap().login, "me");
    }
}
