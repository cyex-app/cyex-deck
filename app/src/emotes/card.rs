//! 1.41 What an emote card shows: where the emote comes from, who made and added it,
//! and a link to its page. Local facts first, the rest fetched when the card opens.

use std::sync::LazyLock;
use std::time::Duration;

use cyex_chat::model::Provider;
use cyex_emotes::{Emote, Scope, seventv};
use cyex_twitch::gql::{self, EmoteKind};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::Emotes;
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
    /// "Channel emote", "Global emote", "Personal emote", "Tier 1", "Follower", "Bits".
    pub kind: Option<String>,
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
    /// 7TV: the owner's page.
    pub owner_url: Option<String>,
    /// Twitch: "Inactive", "Archived", "Pending", "Deleted"; `None` when active or unknown.
    pub state: Option<String>,
}

/// 1.41 Cards that loaded, per provider, emote and chat (Twitch: any chat), for an hour.
static CARDS: LazyLock<Ttl<(Provider, String, String), EmoteCard>> =
    LazyLock::new(|| Ttl::new(Duration::from_secs(60 * 60)));

/// Facts about an emote. `login` is the chat it was clicked in.
pub async fn card(
    app: &AppHandle,
    login: &str,
    provider: Provider,
    id: &str,
    name: &str,
) -> Result<EmoteCard, String> {
    let http = app.state::<Auth>().http.clone();
    let chat = if provider == Provider::Twitch { "" } else { login };
    let key = (provider, id.to_owned(), chat.to_owned());
    let mut card = match CARDS.get(&key) {
        Some(c) => c,
        None => {
            let (c, complete) = if provider == Provider::Twitch {
                (twitch(&http, id, name).await?, true)
            } else {
                third_party(app, &http, login, provider, id, name).await
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
) -> (EmoteCard, bool) {
    let (local, scope) =
        find(app, login, provider, id).unwrap_or_else(|| (Emote::new(provider, id, name), None));
    let mut card = EmoteCard {
        provider,
        id: id.to_owned(),
        name: name.to_owned(),
        original: local.original.clone(),
        kind: scope.map(|s| {
            match s {
                Scope::Channel => "Channel emote",
                Scope::Global => "Global emote",
                Scope::Personal => "Personal emote",
            }
            .to_owned()
        }),
        channel: None,
        owner: local.owner.clone(),
        added_by: None,
        added_at: local.added_at.clone(),
        zero_width: local.zero_width,
        unlisted: local.unlisted,
        animated: local.animated,
        page: page(provider, id, local.original.as_deref().unwrap_or(&local.name)),
        channel_login: None,
        owner_url: local
            .owner_id
            .as_deref()
            .filter(|_| provider == Provider::Seventv)
            .map(seventv_user),
        state: None,
    };
    let complete = match provider {
        Provider::Seventv => {
            let (owner, names) = tokio::join!(
                async {
                    match &card.owner {
                        Some(o) => Ok(Some(o.clone())),
                        None => seventv::api::owner_name(http, id).await,
                    }
                },
                async {
                    let ids: Vec<String> = local.added_by.iter().cloned().collect();
                    seventv::api::user_names(http, &ids).await
                }
            );
            let complete = owner.is_ok() && names.is_ok();
            card.owner = owner.ok().flatten();
            card.added_by = local.added_by.as_ref().and_then(|a| names.ok()?.get(a).cloned());
            complete
        }
        _ => super::providers::card(http, &mut card).await,
    };
    (card, complete)
}

async fn twitch(http: &cyex_twitch::Http, id: &str, name: &str) -> Result<EmoteCard, String> {
    let info = gql::emote_info(http, id).await.map_err(|e| e.to_string())?;
    let mut card = EmoteCard {
        provider: Provider::Twitch,
        id: id.to_owned(),
        name: name.to_owned(),
        original: None,
        kind: None,
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
    };
    let Some(info) = info else { return Ok(card) };
    card.animated = info.emote.animated;
    card.kind = Some(twitch_kind(&info));
    card.owner = info.artist;
    if let Some(o) = info.owner {
        card.channel_login = Some(o.login);
        card.channel = Some(o.name);
    }
    Ok(card)
}

fn twitch_kind(info: &gql::TwitchEmoteInfo) -> String {
    if let Some(t) = info.tier {
        return format!("Tier {t}");
    }
    if let Some(b) = info.bits {
        return bits(b);
    }
    let by_type = match info.raw_type.as_deref() {
        Some("TWO_FACTOR") => Some("Two-factor emote"),
        Some("SMILIES") => Some("Smiley"),
        Some("ARCHIVE") => Some("Archived set"),
        Some("MEGA_COMMERCE") => Some("Event emote"),
        _ => None,
    };
    by_type
        .unwrap_or(match info.emote.kind {
            EmoteKind::Follower => "Follower",
            EmoteKind::Global => "Global emote",
            EmoteKind::Prime => "Prime",
            EmoteKind::Turbo => "Turbo",
            EmoteKind::HypeTrain => "Hype train",
            EmoteKind::Points => "Unlocked with points",
            EmoteKind::LimitedTime => "Limited time",
            EmoteKind::TwoFactor => "Two-factor emote",
            EmoteKind::Bits => BITS,
            _ => "Twitch emote",
        })
        .to_owned()
}

/// A bits emote whose threshold GQL didn't say.
const BITS: &str = "Bits";

fn bits(n: u32) -> String {
    format!("Bits, {} cheered", cyex_core::number::grouped(u64::from(n)))
}

/// What potat or ivr add to a Twitch card.
fn apply(card: &mut EmoteCard, f: &twitch_state::Facts) {
    card.state.clone_from(&f.state);
    if card.kind.as_deref() == Some(BITS)
        && let Some(n) = f.bit_cost
    {
        card.kind = Some(bits(n));
    }
}

/// The emote as this chat knows it, and which kind of set it's in.
fn find(app: &AppHandle, login: &str, provider: Provider, id: &str) -> Option<(Emote, Option<Scope>)> {
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
                .map(|e| ((**e).clone(), Some(s.scope)))
        })
}

fn seventv_user(id: &str) -> String {
    format!("https://7tv.app/users/{id}")
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
        Provider::Twitch => Some(format!("https://chatvau.lt/emote/twitch/{id}")),
    }
}

#[cfg(test)]
mod tests {
    use cyex_twitch::gql::{TwitchEmote, TwitchEmoteInfo};

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
            raw_type: raw.map(str::to_owned),
        }
    }

    #[test]
    fn twitch_kind_labels() {
        let k = |kind, raw, tier, b| twitch_kind(&info(kind, raw, tier, b));
        assert_eq!(
            k(EmoteKind::Subscription, Some("SUBSCRIPTIONS"), Some(2), None),
            "Tier 2"
        );
        assert_eq!(k(EmoteKind::Bits, None, None, Some(1000)), "Bits, 1.000 cheered");
        assert_eq!(k(EmoteKind::Bits, Some("BITS_BADGE_TIERS"), None, None), "Bits");
        assert_eq!(
            k(EmoteKind::TwoFactor, Some("TWO_FACTOR"), None, None),
            "Two-factor emote"
        );
        assert_eq!(k(EmoteKind::TwoFactor, None, None, None), "Two-factor emote");
        assert_eq!(k(EmoteKind::Global, Some("SMILIES"), None, None), "Smiley");
        assert_eq!(k(EmoteKind::Other, Some("ARCHIVE"), None, None), "Archived set");
        assert_eq!(
            k(EmoteKind::LimitedTime, Some("MEGA_COMMERCE"), None, None),
            "Event emote"
        );
        assert_eq!(
            k(EmoteKind::LimitedTime, Some("LIMITED_TIME"), None, None),
            "Limited time"
        );
        assert_eq!(k(EmoteKind::Global, Some("GLOBALS"), None, None), "Global emote");
        assert_eq!(k(EmoteKind::Other, Some("NEW"), None, None), "Twitch emote");
    }

    fn blank() -> EmoteCard {
        EmoteCard {
            provider: Provider::Twitch,
            id: "1".into(),
            name: "x".into(),
            original: None,
            kind: Some(BITS.into()),
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
        assert_eq!(c.kind.as_deref(), Some("Bits, 5.000 cheered"));
        let mut c = blank();
        c.kind = Some("Tier 1".into());
        apply(&mut c, &f);
        assert_eq!(c.kind.as_deref(), Some("Tier 1"));
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
}
