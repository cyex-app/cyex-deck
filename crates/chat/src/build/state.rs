//! ROOMSTATE and USERSTATE. ROOMSTATE sends every mode on join, then only what changed.

use crate::irc::Line;
use crate::model::{Badge, RoomState, SelfState};

use super::sender::badges;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RoomStateUpdate {
    pub channel_id: Option<String>,
    pub slow: Option<u32>,
    /// `Some(None)` = followers-only turned off.
    pub followers_only: Option<Option<u32>>,
    pub subs_only: Option<bool>,
    pub emote_only: Option<bool>,
    pub unique: Option<bool>,
}

impl RoomStateUpdate {
    pub fn from_line(line: &Line<'_>) -> Self {
        let flag = |key| line.raw_tag(key).map(|v| v == "1");
        Self {
            channel_id: line.tag_nonempty("room-id").map(str::to_owned),
            slow: line.tag_num("slow"),
            followers_only: line
                .tag_num::<i64>("followers-only")
                .map(|m| u32::try_from(m).ok()),
            subs_only: flag("subs-only"),
            emote_only: flag("emote-only"),
            unique: flag("r9k"),
        }
    }

    pub fn apply(self, state: &mut RoomState) {
        if self.channel_id.is_some() {
            state.channel_id = self.channel_id;
        }
        if let Some(v) = self.slow {
            state.slow = v;
        }
        if let Some(v) = self.followers_only {
            state.followers_only = v;
        }
        if let Some(v) = self.subs_only {
            state.subs_only = v;
        }
        if let Some(v) = self.emote_only {
            state.emote_only = v;
        }
        if let Some(v) = self.unique {
            state.unique = v;
        }
    }
}

/// Our own badges in a room. Arrives on join and after each message we send.
#[derive(Debug, PartialEq, Eq)]
pub struct UserState {
    pub badges: Vec<Badge>,
    pub color: Option<String>,
    pub display: Option<String>,
    /// `mod=1`
    pub moderator: bool,
    /// Id of the message we just sent, when this follows a send.
    pub sent_id: Option<String>,
}

impl UserState {
    pub fn from_line(line: &Line<'_>) -> Self {
        Self {
            badges: badges(line.raw_tag("badges"), line.raw_tag("badge-info")),
            color: line.tag_nonempty("color").map(str::to_owned),
            display: line.tag("display-name").map(|d| d.into_owned()).filter(|d| !d.is_empty()),
            moderator: line.tag_flag("mod"),
            sent_id: line.tag_nonempty("id").map(str::to_owned),
        }
    }

    /// `owner`: the room is the account's own channel (V102-21).
    pub fn to_self(&self, login: &str, owner: bool) -> SelfState {
        SelfState::new(login, self.badges.clone(), self.moderator, owner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModRole;

    #[test]
    fn roomstate_full_then_partial() {
        let mut s = RoomState::default();
        RoomStateUpdate::from_line(
            &Line::parse("@emote-only=0;followers-only=-1;r9k=0;room-id=12;slow=0;subs-only=0 :tmi.twitch.tv ROOMSTATE #c")
                .unwrap(),
        )
        .apply(&mut s);
        assert_eq!(s.channel_id.as_deref(), Some("12"));
        assert_eq!(s.followers_only, None);
        RoomStateUpdate::from_line(&Line::parse("@followers-only=10;room-id=12 :tmi.twitch.tv ROOMSTATE #c").unwrap())
            .apply(&mut s);
        RoomStateUpdate::from_line(&Line::parse("@slow=30;room-id=12 :tmi.twitch.tv ROOMSTATE #c").unwrap())
            .apply(&mut s);
        assert_eq!((s.followers_only, s.slow), (Some(10), 30));
        RoomStateUpdate::from_line(&Line::parse("@followers-only=-1;room-id=12 :tmi.twitch.tv ROOMSTATE #c").unwrap())
            .apply(&mut s);
        assert_eq!((s.followers_only, s.slow), (None, 30));
    }

    #[test]
    fn userstate_roles() {
        let u = UserState::from_line(
            &Line::parse("@badges=vip/1;color=#FF0000;display-name=Me;id=abc;mod=0 :tmi.twitch.tv USERSTATE #c").unwrap(),
        );
        assert_eq!(u.sent_id.as_deref(), Some("abc"));
        let me = u.to_self("me", false);
        assert!(me.vip && me.role.is_none() && me.fast_lane() && !me.privileged());
        // V102-21 Lead mods, `mod=1` alone, and the channel's own account.
        let lead = UserState::from_line(
            &Line::parse("@badges=lead_moderator/1;mod=1 :tmi.twitch.tv USERSTATE #c").unwrap(),
        );
        assert_eq!(lead.to_self("me", false).role, Some(ModRole::LeadModerator));
        let tagged = UserState::from_line(&Line::parse("@badges=;mod=1 :tmi.twitch.tv USERSTATE #c").unwrap());
        assert_eq!(tagged.to_self("me", false).role, Some(ModRole::Moderator));
        let own = UserState::from_line(&Line::parse("@badges=;mod=0 :tmi.twitch.tv USERSTATE #me").unwrap());
        assert_eq!(own.to_self("me", true).role, Some(ModRole::Broadcaster));
    }
}
