//! 1.41 The emote card's flags, in the order the card shows them: scope, Twitch type,
//! then what the emote is (animated, overlaying, private, ...). The UI words and explains them.

use cyex_emotes::{Emote, Scope};
use cyex_twitch::gql::{EmoteKind, TwitchEmoteInfo};
use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CardFlag {
    Global,
    Channel,
    Personal,
    /// BTTV: another channel's emote this one added.
    Shared,
    Tier1,
    Tier2,
    Tier3,
    Follower,
    Bits,
    Prime,
    Turbo,
    HypeTrain,
    TwoFactor,
    LimitedTime,
    Points,
    Smiley,
    ArchivedSet,
    Event,
    Animated,
    /// Zero-width: drawn over the emote before it.
    Overlaying,
    Private,
    Unlisted,
    PersonalApproved,
    PersonalDenied,
    Nsfw,
    /// Renamed in this channel.
    Aliased,
    /// FFZ: changes the emote before it.
    Modifier,
}

/// A Twitch emote: channel or global, then its type.
pub fn twitch(info: &TwitchEmoteInfo) -> Vec<CardFlag> {
    let mut out = vec![if info.owner.is_some() {
        CardFlag::Channel
    } else {
        CardFlag::Global
    }];
    out.extend(twitch_type(info));
    if info.emote.animated {
        out.push(CardFlag::Animated);
    }
    out
}

fn twitch_type(info: &TwitchEmoteInfo) -> Option<CardFlag> {
    match info.tier {
        Some(1) => return Some(CardFlag::Tier1),
        Some(2) => return Some(CardFlag::Tier2),
        Some(3) => return Some(CardFlag::Tier3),
        _ => {}
    }
    if info.bits.is_some() {
        return Some(CardFlag::Bits);
    }
    match info.raw_type.as_deref() {
        Some("TWO_FACTOR") => return Some(CardFlag::TwoFactor),
        Some("SMILIES") => return Some(CardFlag::Smiley),
        Some("ARCHIVE") => return Some(CardFlag::ArchivedSet),
        Some("MEGA_COMMERCE") => return Some(CardFlag::Event),
        _ => {}
    }
    Some(match info.emote.kind {
        EmoteKind::Follower => CardFlag::Follower,
        EmoteKind::Bits => CardFlag::Bits,
        EmoteKind::Prime => CardFlag::Prime,
        EmoteKind::Turbo => CardFlag::Turbo,
        EmoteKind::HypeTrain => CardFlag::HypeTrain,
        EmoteKind::Points => CardFlag::Points,
        EmoteKind::LimitedTime => CardFlag::LimitedTime,
        EmoteKind::TwoFactor => CardFlag::TwoFactor,
        _ => return None,
    })
}

/// A 7TV, BTTV or FFZ emote, with the kind of set it's in (`None`: not in a loaded set).
pub fn third_party(e: &Emote, scope: Option<Scope>) -> Vec<CardFlag> {
    let mut out = Vec::new();
    match scope {
        _ if e.shared => out.push(CardFlag::Shared),
        Some(Scope::Global) => out.push(CardFlag::Global),
        Some(Scope::Channel) => out.push(CardFlag::Channel),
        Some(Scope::Personal) => out.push(CardFlag::Personal),
        None => {}
    }
    let rest = [
        (e.animated, CardFlag::Animated),
        (e.zero_width, CardFlag::Overlaying),
        (e.private, CardFlag::Private),
        (e.unlisted, CardFlag::Unlisted),
        (e.approved_personal, CardFlag::PersonalApproved),
        (e.denied_personal, CardFlag::PersonalDenied),
        (e.nsfw, CardFlag::Nsfw),
        (e.original.is_some(), CardFlag::Aliased),
        (e.modifier.is_some(), CardFlag::Modifier),
    ];
    out.extend(rest.into_iter().filter(|(on, _)| *on).map(|(_, f)| f));
    out
}

#[cfg(test)]
mod tests {
    use cyex_chat::model::Provider;
    use cyex_twitch::gql::{EmoteOwner, TwitchEmote};

    use super::CardFlag::*;
    use super::*;

    fn info(kind: EmoteKind, raw: Option<&str>, tier: Option<u8>, bits: Option<u32>) -> TwitchEmoteInfo {
        TwitchEmoteInfo {
            emote: TwitchEmote {
                id: "1".into(),
                name: "x".into(),
                set_id: "9".into(),
                kind,
                animated: false,
                modifiers: Vec::new(),
            },
            owner: None,
            tier,
            bits,
            artist: None,
            artist_login: None,
            raw_type: raw.map(str::to_owned),
        }
    }

    #[test]
    fn twitch_types() {
        let t = |kind, raw, tier, b| twitch_type(&info(kind, raw, tier, b));
        assert_eq!(
            t(EmoteKind::Subscription, Some("SUBSCRIPTIONS"), Some(1), None),
            Some(Tier1)
        );
        assert_eq!(
            t(EmoteKind::Subscription, Some("SUBSCRIPTIONS"), Some(2), None),
            Some(Tier2)
        );
        assert_eq!(
            t(EmoteKind::Subscription, Some("SUBSCRIPTIONS"), Some(3), None),
            Some(Tier3)
        );
        assert_eq!(
            t(EmoteKind::Subscription, Some("SUBSCRIPTIONS"), None, None),
            None
        );
        assert_eq!(t(EmoteKind::Bits, None, None, Some(1000)), Some(Bits));
        assert_eq!(
            t(EmoteKind::Bits, Some("BITS_BADGE_TIERS"), None, None),
            Some(Bits)
        );
        assert_eq!(
            t(EmoteKind::TwoFactor, Some("TWO_FACTOR"), None, None),
            Some(TwoFactor)
        );
        assert_eq!(t(EmoteKind::TwoFactor, None, None, None), Some(TwoFactor));
        assert_eq!(t(EmoteKind::Global, Some("SMILIES"), None, None), Some(Smiley));
        assert_eq!(
            t(EmoteKind::Other, Some("ARCHIVE"), None, None),
            Some(ArchivedSet)
        );
        assert_eq!(
            t(EmoteKind::LimitedTime, Some("MEGA_COMMERCE"), None, None),
            Some(Event)
        );
        assert_eq!(
            t(EmoteKind::LimitedTime, Some("LIMITED_TIME"), None, None),
            Some(LimitedTime)
        );
        assert_eq!(
            t(EmoteKind::Follower, Some("FOLLOWER"), None, None),
            Some(Follower)
        );
        assert_eq!(t(EmoteKind::Prime, None, None, None), Some(Prime));
        assert_eq!(t(EmoteKind::Turbo, None, None, None), Some(Turbo));
        assert_eq!(t(EmoteKind::HypeTrain, None, None, None), Some(HypeTrain));
        assert_eq!(t(EmoteKind::Points, None, None, None), Some(Points));
        assert_eq!(t(EmoteKind::Global, Some("GLOBALS"), None, None), None);
        assert_eq!(t(EmoteKind::Other, Some("NEW"), None, None), None);
    }

    #[test]
    fn twitch_scope_then_type_then_animated() {
        let mut i = info(EmoteKind::Subscription, Some("SUBSCRIPTIONS"), Some(2), None);
        i.emote.animated = true;
        i.owner = Some(EmoteOwner {
            id: "1".into(),
            login: "a".into(),
            name: "A".into(),
            avatar: None,
        });
        assert_eq!(twitch(&i), [Channel, Tier2, Animated]);
        assert_eq!(
            twitch(&info(EmoteKind::Global, Some("GLOBALS"), None, None)),
            [Global]
        );
    }

    #[test]
    fn seventv_flags_in_order() {
        let mut e = Emote::new(Provider::Seventv, "E", "a");
        e.animated = true;
        e.zero_width = true;
        e.private = true;
        e.unlisted = true;
        e.denied_personal = true;
        e.nsfw = true;
        e.original = Some("b".into());
        assert_eq!(
            third_party(&e, Some(Scope::Channel)),
            [
                Channel,
                Animated,
                Overlaying,
                Private,
                Unlisted,
                PersonalDenied,
                Nsfw,
                Aliased
            ]
        );
        let mut p = Emote::new(Provider::Seventv, "E", "a");
        p.approved_personal = true;
        assert_eq!(
            third_party(&p, Some(Scope::Personal)),
            [Personal, PersonalApproved]
        );
        assert_eq!(third_party(&p, None), [PersonalApproved]);
    }

    #[test]
    fn bttv_shared_and_overlaying() {
        let mut e = Emote::new(Provider::Bttv, "B", "SoSnowy");
        e.shared = true;
        e.zero_width = true;
        assert_eq!(third_party(&e, Some(Scope::Channel)), [Shared, Overlaying]);
        let g = Emote::new(Provider::Bttv, "B", "x");
        assert_eq!(third_party(&g, Some(Scope::Global)), [Global]);
    }

    #[test]
    fn ffz_modifier() {
        let mut e = Emote::new(Provider::Ffz, "1", "ffzHyper");
        e.modifier = Some(2);
        assert_eq!(third_party(&e, Some(Scope::Global)), [Global, Modifier]);
    }
}
