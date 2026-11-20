//! 1.41 What an emote card shows: where the emote comes from, who made and added it,
//! and a link to its page. Local facts first, the rest fetched when the card opens.

use std::sync::LazyLock;
use std::time::Duration;

use cyex_chat::model::Provider;
use cyex_emotes::cyextv::api as cyextv;
use cyex_emotes::{Emote, Scope, bttv, seventv};
use cyex_twitch::gql;
use cyex_twitch::pace::Lane;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::Emotes;
use super::card_flags::{self, CardFlag};
use super::ttl::Ttl;
use super::twitch_state;
use crate::auth::Auth;

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmoteCard {
    pub provider: Provider,
    pub id: String,
    pub name: String,
    /// The emote's own name, when this channel renamed it (7TV).
    pub original: Option<String>,
    /// What it is, in card order (scope, Twitch type, animated, overlaying, ...).
    pub flags: Vec<CardFlag>,
    /// Twitch bits emotes: bits cheered to unlock it, when known.
    pub bits: Option<u32>,
    /// Twitch: the channel it belongs to.
    pub channel: Option<String>,
    /// Who made it.
    pub owner: Option<String>,
    pub added_by: Option<String>,
    /// ISO date.
    pub added_at: Option<String>,
    pub zero_width: bool,
    pub unlisted: bool,
    pub animated: bool,
    pub page: Option<String>,
    /// Twitch: the owning channel's login, opened in the app.
    pub channel_login: Option<String>,
    /// The owner's page: 7TV, BTTV or FFZ user.
    pub owner_url: Option<String>,
    /// Twitch: "Inactive", "Archived", "Pending", "Deleted"; `None` when active or unknown.
    pub state: Option<String>,
    /// Twitch: the owning channel's picture.
    pub channel_avatar: Option<String>,
    /// Twitch: the artist's login, opened in the app.
    pub artist_login: Option<String>,
    /// 7TV: the page of the set it came from, when a loaded set has it.
    pub set_url: Option<String>,
    /// V112-09 cyex.tv: the set's name, shown on its link.
    pub set_name: Option<String>,
    /// V112-08 cyex.tv: when the emote was made (ISO). `added_at` is when a set got it.
    pub created_at: Option<String>,
    /// V112-08 cyex.tv: its tags.
    pub tags: Vec<String>,
    /// V112-08 cyex.tv: how many channels use it.
    pub channels: Option<u32>,
    /// V112-08 cyex.tv: the owner's picture.
    pub owner_avatar: Option<String>,
}

/// 1.41 Cards that loaded, per provider, emote and chat (Twitch: any chat), for an hour.
type CardKey = (Provider, String, String, Option<Scope>);

static CARDS: LazyLock<Ttl<CardKey, EmoteCard>> =
    LazyLock::new(|| Ttl::new(Duration::from_secs(60 * 60)));

/// Facts about an emote. `login` is the chat it was clicked in; `sender` the id of the
/// message's author, whose personal set may be where the emote came from.
pub async fn card(
    app: &AppHandle,
    login: &str,
    provider: Provider,
    id: &str,
    name: &str,
    sender: Option<&str>,
) -> Result<EmoteCard, String> {
    let http = app.state::<Auth>().http.clone();
    let chat = if provider == Provider::Twitch { "" } else { login };
    let personal = sender.and_then(|u| in_personal(app, u, provider, id));
    let key = (provider, id.to_owned(), chat.to_owned(), personal);
    let mut card = match CARDS.get(&key) {
        Some(c) => c,
        None => {
            let (c, complete) = if provider == Provider::Twitch {
                (twitch(app, &http, id, name).await?, true)
            } else {
                third_party(app, &http, login, provider, id, name, personal).await
            };
            if complete {
                CARDS.put(key, c.clone());
            }
            c
        }
    };
    card.name = name.to_owned();
    if provider == Provider::Twitch
        && let Some(f) = twitch_state::facts(&http, id).await
    {
        apply(&mut card, &f);
    }
    Ok(card)
}

/// 7TV, BTTV, FFZ. `false` when a lookup failed, so the card isn't kept.
async fn third_party(
    app: &AppHandle,
    http: &cyex_twitch::Http,
    login: &str,
    provider: Provider,
    id: &str,
    name: &str,
    personal: Option<Scope>,
) -> (EmoteCard, bool) {
    let found = find(app, login, provider, id);
    let set_id = found.as_ref().map(|f| f.2.clone());
    let set_name = found.as_ref().and_then(|f| f.3.clone());
    let (mut local, scope) = found
        .map(|(e, s, _, _)| (e, Some(s)))
        .unwrap_or_else(|| (Emote::new(provider, id, name), None));
    let mut extra = None;
    let mut set_url = None;
    let complete = match provider {
        Provider::Seventv => {
            let ask = local.owner.is_none();
            let (detail, names) = tokio::join!(
                async {
                    if ask {
                        seventv::api::emote(http, id, Lane::Front).await
                    } else {
                        Ok(None)
                    }
                },
                async {
                    let ids: Vec<String> = local.added_by.iter().cloned().collect();
                    seventv::api::user_names(http, &ids, Lane::Front).await
                }
            );
            let complete = detail.is_ok() && names.is_ok();
            if let Some(d) = detail.ok().flatten() {
                merge(&mut local, d, scope.is_none());
            }
            local.added_by = local.added_by.as_ref().and_then(|a| names.ok()?.get(a).cloned());
            complete
        }
        Provider::Bttv if local.owner.is_none() => match bttv::api::uploader(http, id).await {
            Ok(o) => {
                if let Some((who, user)) = o {
                    local.owner = Some(who);
                    local.owner_key = local.owner_key.take().or(Some(user));
                }
                true
            }
            Err(_) => false,
        },
        Provider::Cyextv => {
            // V112-08, V112-09 Detail and the set's page id, both lazily, both cached with the card.
            let (detail, page) = tokio::join!(
                cyextv::emote(http, cyextv::emote_id(id), Lane::Front),
                cyextv_set_page(http, scope, set_id.as_deref()),
            );
            // The set's adder is an account id with no name to look up here.
            local.added_by = None;
            let complete = detail.is_ok() && page.is_ok();
            // A 404 is an answer: only what the set gave.
            if let Ok(Some(d)) = detail {
                cyextv_merge(&mut local, &d, scope.is_none());
                extra = Some(d);
            }
            set_url = page.ok().flatten().map(|s| format!("https://cyex.tv/sets/{s}"));
            complete
        }
        _ => true,
    };
    let mut card = third_party_card(&local, scope, personal, set_id.as_deref(), name);
    if let Some(d) = extra {
        cyextv_fill(&mut card, &d);
    }
    if provider == Provider::Cyextv {
        card.set_url = set_url;
        card.set_name = set_name.filter(|_| card.set_url.is_some());
    }
    (card, complete)
}

/// V112-09 The id of the set's page on cyex.tv: a personal set's own id; the channel's active
/// set (the deck's id holds the Twitch id); the global set's page isn't known.
async fn cyextv_set_page(
    http: &cyex_twitch::Http,
    scope: Option<Scope>,
    set_id: Option<&str>,
) -> cyex_twitch::Result<Option<String>> {
    let Some(rest) = set_id.and_then(|s| s.strip_prefix("cyextv-")) else {
        return Ok(None);
    };
    match scope {
        Some(Scope::Personal) => Ok(Some(rest.to_owned())),
        Some(Scope::Channel) => cyextv::active_set_id(http, rest, Lane::Front).await,
        _ => Ok(None),
    }
}

/// V112-08 cyex.tv's own facts for an emote no loaded set has: its flags and name; the owner
/// always.
fn cyextv_merge(local: &mut Emote, d: &cyextv::EmoteDetail, unknown: bool) {
    local.owner = d.owner.clone().or(local.owner.take());
    local.owner_id = d.owner_id.clone().or(local.owner_id.take());
    if unknown {
        local.original = (d.name != local.name && !d.name.is_empty()).then(|| d.name.clone());
        local.animated = d.animated;
        local.zero_width = d.zero_width;
        local.nsfw = d.nsfw;
        local.private = d.private;
    }
}

/// V112-08 The card lines only cyex.tv's lookup has.
fn cyextv_fill(card: &mut EmoteCard, d: &cyextv::EmoteDetail) {
    card.created_at.clone_from(&d.created_at);
    card.tags.clone_from(&d.tags);
    card.channels = d.channels;
    card.owner_avatar.clone_from(&d.owner_avatar);
}

/// 1.41 What 7TV's own lookup adds: the owner always; its flags and name when no
/// loaded set had the emote.
fn merge(local: &mut Emote, d: Emote, unknown: bool) {
    let owner_id = local.owner_id.take().or(d.owner_id.clone());
    if unknown {
        let shown = std::mem::take(&mut local.name);
        *local = Emote {
            original: (d.name != shown).then(|| d.name.clone()),
            name: shown,
            ..d
        };
    } else {
        local.owner = d.owner;
    }
    local.owner_id = owner_id;
}

/// `local.added_by` holds the adder's name by now; `name` is what chat shows.
fn third_party_card(
    local: &Emote,
    scope: Option<Scope>,
    personal: Option<Scope>,
    set_id: Option<&str>,
    name: &str,
) -> EmoteCard {
    let provider = local.provider;
    EmoteCard {
        provider,
        id: local.id.clone(),
        name: name.to_owned(),
        original: local.original.clone(),
        flags: card_flags::third_party(local, scope, personal),
        bits: None,
        channel: None,
        owner: local.owner.clone(),
        added_by: local.added_by.clone(),
        added_at: local.added_at.clone(),
        zero_width: local.zero_width,
        unlisted: local.unlisted,
        animated: local.animated,
        page: page(
            provider,
            &local.id,
            local.original.as_deref().unwrap_or(&local.name),
        ),
        channel_login: None,
        owner_url: owner_url(provider, local),
        state: None,
        channel_avatar: None,
        artist_login: None,
        set_url: set_id
            .filter(|_| provider == Provider::Seventv)
            .map(|s| format!("https://7tv.app/emote-sets/{s}")),
        set_name: None,
        created_at: None,
        tags: Vec::new(),
        channels: None,
        owner_avatar: None,
    }
}

async fn twitch(
    app: &AppHandle,
    http: &cyex_twitch::Http,
    id: &str,
    name: &str,
) -> Result<EmoteCard, String> {
    let info = gql::emote_info(http, id).await.map_err(|e| e.to_string())?;
    let mut card = EmoteCard {
        provider: Provider::Twitch,
        id: id.to_owned(),
        name: name.to_owned(),
        original: None,
        flags: Vec::new(),
        bits: None,
        channel: None,
        owner: None,
        added_by: None,
        added_at: None,
        zero_width: false,
        unlisted: false,
        animated: false,
        page: page(Provider::Twitch, id, name),
        channel_login: None,
        owner_url: None,
        state: None,
        channel_avatar: None,
        artist_login: None,
        set_url: None,
        set_name: None,
        created_at: None,
        tags: Vec::new(),
        channels: None,
        owner_avatar: None,
    };
    let Some(mut info) = info else { return Ok(card) };
    // fc-1 / fa-1 The card's own answer lacks the tier for animated tier 2 and 3 emotes (GQL says null);
    // the channel's products have it. Cached for the menu, fetched when cold.
    if info.tier.is_none()
        && info.emote.kind == gql::EmoteKind::Subscription
        && let Some(o) = &info.owner
        && let Some(c) = super::menu::channel_emotes(app, &o.id).await
    {
        info.tier = tier_in(&c, id);
    }
    card.animated = info.emote.animated;
    card.flags = card_flags::twitch(&info);
    card.bits = info.bits;
    card.owner = info.artist;
    card.artist_login = info.artist_login;
    if let Some(o) = info.owner {
        card.channel_login = Some(o.login);
        card.channel = Some(o.name);
        card.channel_avatar = o.avatar;
    }
    Ok(card)
}

/// fc-1 The sub tier a channel lists this emote under.
fn tier_in(c: &gql::ChannelEmotes, id: &str) -> Option<u8> {
    c.tiers
        .iter()
        .find(|t| t.emotes.iter().any(|e| e.id == id))
        .map(|t| t.tier)
}

/// What potat or ivr add to a Twitch card.
fn apply(card: &mut EmoteCard, f: &twitch_state::Facts) {
    card.state.clone_from(&f.state);
    if card.flags.contains(&CardFlag::Bits) && card.bits.is_none() {
        card.bits = f.bit_cost;
    }
}

/// 7TV, BTTV, FFZ: the kind of this user's own granted set that has the emote (personal or event).
fn in_personal(app: &AppHandle, user: &str, provider: Provider, id: &str) -> Option<Scope> {
    let state = app.state::<Emotes>();
    let inner = state.lock();
    inner
        .user_sets
        .get(user)
        .into_iter()
        .flatten()
        .filter_map(|sid| inner.personal_sets.get(sid))
        .filter(|s| s.provider == provider && s.emotes.iter().any(|e| e.id == id))
        // A personal set wins over an event set holding the same emote.
        .map(|s| s.scope)
        .min_by_key(|s| u8::from(*s != Scope::Personal))
}

/// The emote as this chat knows it, the kind of set it's in, and that set's id and name.
fn find(
    app: &AppHandle,
    login: &str,
    provider: Provider,
    id: &str,
) -> Option<(Emote, Scope, String, Option<String>)> {
    let state = app.state::<Emotes>();
    let inner = state.lock();
    let channel = inner
        .channels
        .get(login)
        .map(|c| c.sets.as_slice())
        .unwrap_or_default();
    channel
        .iter()
        .chain(inner.globals.iter())
        .chain(inner.personal_sets.values())
        .filter(|s| s.provider == provider)
        .find_map(|s| {
            s.emotes
                .iter()
                .find(|e| e.id == id)
                .map(|e| ((**e).clone(), s.scope, s.id.clone(), s.name.clone()))
        })
}

fn seventv_user(id: &str) -> String {
    format!("https://7tv.app/users/{id}")
}

/// 1.41 The maker's page: 7TV user, BTTV user, FFZ uploader.
fn owner_url(provider: Provider, e: &Emote) -> Option<String> {
    match provider {
        Provider::Seventv => e.owner_id.as_deref().map(seventv_user),
        Provider::Bttv => e
            .owner_key
            .as_deref()
            .filter(|k| k.chars().all(|c| c.is_ascii_alphanumeric()))
            .map(|k| format!("https://betterttv.com/users/{k}")),
        Provider::Ffz => {
            let key = e.owner_key.as_deref()?;
            let mut url = url::Url::parse("https://www.frankerfacez.com/emoticons/user/").ok()?;
            url.path_segments_mut().ok()?.pop_if_empty().push(key);
            Some(url.into())
        }
        // V112-08 cyex.tv's user page, by account id.
        Provider::Cyextv => e
            .owner_id
            .as_deref()
            .filter(|k| k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
            .map(|k| format!("https://cyex.tv/users/{k}")),
        Provider::Twitch => None,
    }
}

/// `name` is the emote's own name (FFZ's page has it in the slug).
fn page(provider: Provider, id: &str, name: &str) -> Option<String> {
    match provider {
        Provider::Seventv => Some(format!("https://7tv.app/emotes/{id}")),
        Provider::Bttv => Some(format!("https://betterttv.com/emotes/{id}")),
        Provider::Ffz => {
            let mut url = url::Url::parse("https://www.frankerfacez.com/emoticon/").ok()?;
            url.path_segments_mut()
                .ok()?
                .pop_if_empty()
                .push(&format!("{id}-{name}"));
            Some(url.into())
        }
        Provider::Cyextv => Some(format!("https://cyex.tv/emotes/{}", cyex_emotes::cyextv::api::emote_id(id))),
        Provider::Twitch => Some(format!("https://chatvau.lt/emote/twitch/{id}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> EmoteCard {
        EmoteCard {
            provider: Provider::Twitch,
            id: "1".into(),
            name: "x".into(),
            original: None,
            flags: vec![CardFlag::Channel, CardFlag::Bits],
            bits: None,
            channel: None,
            owner: None,
            added_by: None,
            added_at: None,
            zero_width: false,
            unlisted: false,
            animated: false,
            page: None,
            channel_login: None,
            owner_url: None,
            state: None,
            channel_avatar: None,
            artist_login: None,
            set_url: None,
            set_name: None,
            created_at: None,
            tags: Vec::new(),
            channels: None,
            owner_avatar: None,
        }
    }

    #[test]
    fn facts_fill_state_and_bits() {
        let mut c = blank();
        let f = twitch_state::Facts {
            state: Some("Archived".into()),
            bit_cost: Some(5000),
        };
        apply(&mut c, &f);
        assert_eq!(c.state.as_deref(), Some("Archived"));
        assert_eq!(c.bits, Some(5000));
        // GQL's threshold wins; a sub emote gets no bits.
        let mut c = blank();
        c.bits = Some(100);
        apply(&mut c, &f);
        assert_eq!(c.bits, Some(100));
        let mut c = blank();
        c.flags = vec![CardFlag::Channel, CardFlag::Tier1];
        apply(&mut c, &f);
        assert_eq!(c.bits, None);
    }

    #[test]
    fn pages() {
        assert_eq!(
            page(Provider::Twitch, "emotesv2_ab", "x").as_deref(),
            Some("https://chatvau.lt/emote/twitch/emotesv2_ab")
        );
        assert_eq!(
            page(Provider::Ffz, "720507", "monkaHmm").as_deref(),
            Some("https://www.frankerfacez.com/emoticon/720507-monkaHmm")
        );
        assert_eq!(
            page(Provider::Ffz, "1", "a/b c").as_deref(),
            Some("https://www.frankerfacez.com/emoticon/1-a%2Fb%20c")
        );
        assert_eq!(seventv_user("01ABC"), "https://7tv.app/users/01ABC");
    }

    fn with_key(provider: Provider, key: &str) -> Emote {
        let mut e = Emote::new(provider, "1", "x");
        e.owner_key = Some(key.into());
        e
    }

    #[test]
    fn owner_pages() {
        let mut s = Emote::new(Provider::Seventv, "1", "x");
        s.owner_id = Some("01U".into());
        assert_eq!(
            owner_url(Provider::Seventv, &s).as_deref(),
            Some("https://7tv.app/users/01U")
        );
        assert_eq!(
            owner_url(
                Provider::Bttv,
                &with_key(Provider::Bttv, "561c219aef87f8db05aca668")
            )
            .as_deref(),
            Some("https://betterttv.com/users/561c219aef87f8db05aca668")
        );
        assert_eq!(owner_url(Provider::Bttv, &with_key(Provider::Bttv, "../x")), None);
        assert_eq!(
            owner_url(Provider::Ffz, &with_key(Provider::Ffz, "sadgw")).as_deref(),
            Some("https://www.frankerfacez.com/emoticons/user/sadgw")
        );
        assert_eq!(
            owner_url(Provider::Ffz, &Emote::new(Provider::Ffz, "1", "x")),
            None
        );
    }

    #[test]
    fn third_party_cards() {
        let mut e = Emote::new(Provider::Seventv, "E", "a");
        e.original = Some("b".into());
        e.zero_width = true;
        e.owner_id = Some("U".into());
        let c = third_party_card(&e, Some(Scope::Channel), None, Some("S1"), "a");
        assert_eq!(
            c.flags,
            [CardFlag::Overlaying, CardFlag::Aliased]
        );
        let p = third_party_card(&e, Some(Scope::Channel), Some(Scope::Personal), None, "a");
        assert_eq!(p.flags[0], CardFlag::Personal);
        let ev = third_party_card(&e, Some(Scope::Channel), Some(Scope::Special), None, "a");
        assert_eq!(ev.flags[0], CardFlag::EventSet);
        assert_eq!(c.set_url.as_deref(), Some("https://7tv.app/emote-sets/S1"));
        assert_eq!(c.page.as_deref(), Some("https://7tv.app/emotes/E"));
        let b = third_party_card(
            &with_key(Provider::Bttv, "abc"),
            Some(Scope::Channel),
            None,
            Some("bttv-1"),
            "x",
        );
        assert_eq!(b.set_url, None);
        assert_eq!(b.owner_url.as_deref(), Some("https://betterttv.com/users/abc"));
    }

    #[test]
    fn seventv_lookup_fills_unknown_emotes() {
        let mut d = Emote::new(Provider::Seventv, "E", "own");
        d.owner = Some("Maker".into());
        d.owner_id = Some("U".into());
        d.nsfw = true;
        let mut local = Emote::new(Provider::Seventv, "E", "shown");
        merge(&mut local, d.clone(), true);
        assert_eq!(local.name, "shown");
        assert_eq!(local.original.as_deref(), Some("own"));
        assert!(local.nsfw);
        assert_eq!(local.owner.as_deref(), Some("Maker"));
        // A loaded emote keeps its own flags and gets the owner name only.
        let mut local = Emote::new(Provider::Seventv, "E", "shown");
        local.owner_id = Some("L".into());
        merge(&mut local, d, false);
        assert!(!local.nsfw && local.original.is_none());
        assert_eq!(local.owner.as_deref(), Some("Maker"));
        assert_eq!(local.owner_id.as_deref(), Some("L"));
    }

    #[test]
    fn tier_comes_from_the_channel_products() {
        let e = |id: &str| gql::TwitchEmote {
            id: id.into(),
            name: id.into(),
            set_id: "9".into(),
            kind: gql::EmoteKind::Subscription,
            animated: false,
            modifiers: Vec::new(),
        };
        let c = gql::ChannelEmotes {
            owner: gql::EmoteOwner {
                id: "1".into(),
                login: "a".into(),
                name: "A".into(),
                avatar: None,
            },
            tiers: vec![
                gql::SubTier { tier: 1, emotes: vec![e("a")] },
                gql::SubTier { tier: 2, emotes: vec![e("b")] },
            ],
            follower: Vec::new(),
            bits: Vec::new(),
        };
        assert_eq!(tier_in(&c, "b"), Some(2));
        assert_eq!(tier_in(&c, "a"), Some(1));
        assert_eq!(tier_in(&c, "z"), None);
    }

    #[test]
    fn cyextv_card_takes_the_lookup() {
        let d = cyextv::EmoteDetail {
            name: "Own".into(),
            owner_id: Some("01U".into()),
            owner: Some("Maker".into()),
            owner_avatar: Some("https://cdn.cyex.tv/avatars/01U.webp".into()),
            created_at: Some("2026-11-18T05:34:34.251Z".into()),
            tags: vec!["a".into()],
            channels: Some(1234),
            nsfw: true,
            animated: true,
            ..Default::default()
        };
        // Not in a loaded set: the lookup gives the flags and the original name.
        let mut local = Emote::new(Provider::Cyextv, "01E.key", "shown");
        cyextv_merge(&mut local, &d, true);
        assert_eq!(local.original.as_deref(), Some("Own"));
        assert!(local.nsfw && local.animated);
        let mut card = third_party_card(&local, None, None, None, "shown");
        cyextv_fill(&mut card, &d);
        assert_eq!(card.owner.as_deref(), Some("Maker"));
        assert_eq!(card.owner_url.as_deref(), Some("https://cyex.tv/users/01U"));
        assert_eq!(card.page.as_deref(), Some("https://cyex.tv/emotes/01E"));
        assert_eq!(card.channels, Some(1234));
        assert_eq!(card.tags, ["a"]);
        assert!(card.flags.contains(&CardFlag::Nsfw) && card.flags.contains(&CardFlag::Aliased));
        // In a loaded set: the set's own name and zero-width stay.
        let mut local = Emote::new(Provider::Cyextv, "01E.key", "shown");
        local.zero_width = true;
        cyextv_merge(&mut local, &d, false);
        assert!(local.zero_width && !local.nsfw && local.original.is_none());
        assert_eq!(local.owner.as_deref(), Some("Maker"));
    }
}
