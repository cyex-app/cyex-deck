//! 1.40 The emote menu: Twitch emotes by where they come from (with the channel's
//! locked ones, E5), then 7TV, BTTV and FFZ. Favorites and most used (C12) and emoji
//! (E1) are put together in the UI.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cyex_chat::model::Provider;
use cyex_emotes::{Emote, EmoteSet, Scope};
use cyex_twitch::gql::{self, ChannelEmotes, EmoteKind, EmoteSet as TwitchSet};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{Emotes, load};
use crate::auth::Auth;

/// E5 A channel's sub products and the rest, kept for the session up to an hour.
const CHANNEL_FRESH: Duration = Duration::from_secs(3600);

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmoteMenu {
    pub twitch: Vec<MenuSection>,
    /// Why Twitch emotes are missing, e.g. signed out.
    pub twitch_note: Option<String>,
    pub seventv: Vec<MenuSection>,
    pub bttv: Vec<MenuSection>,
    pub ffz: Vec<MenuSection>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MenuSection {
    pub id: String,
    pub title: String,
    pub avatar: Option<String>,
    pub emotes: Vec<MenuEmote>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MenuEmote {
    #[serde(flatten)]
    pub emote: Emote,
    /// E5 Why you can't use it yet, e.g. "Tier 2 sub".
    pub locked: Option<String>,
    /// 1.40 Twitch sub emotes: the tier, 1 to 3.
    pub tier: Option<u8>,
}

fn open(emotes: impl IntoIterator<Item = Emote>) -> Vec<MenuEmote> {
    emotes
        .into_iter()
        .map(|emote| MenuEmote {
            emote,
            locked: None,
            tier: None,
        })
        .collect()
}

pub async fn menu(app: &AppHandle, login: &str, account: Option<&str>) -> Result<EmoteMenu, String> {
    // 1.26 Whispers have no channel: the account's own channel stands in.
    let channel_id = match super::channel_id(app, login) {
        Some(id) => id,
        None if login.is_empty() => account.ok_or("Sign in to use emotes.")?.to_owned(),
        None => return Err("This chat isn’t open.".into()),
    };
    let (mine, note) = match account {
        Some(a) => match load::twitch_sets(app, a, &channel_id, false).await {
            Ok(sets) => (Some(sets), None),
            Err(e) => (None, Some(e)),
        },
        None => (None, Some("Sign in to use your Twitch emotes.".to_owned())),
    };
    let channel = channel_emotes(app, &channel_id).await;
    let twitch = twitch_sections(
        mine.as_deref().map_or(&[], Vec::as_slice),
        channel.as_deref(),
        &channel_id,
    );

    let state = app.state::<Emotes>();
    let inner = state.lock();
    let channel_sets: Vec<Arc<EmoteSet>> = inner
        .channels
        .get(login)
        .map(|c| c.sets.clone())
        .unwrap_or_default();
    let personal: Vec<Arc<EmoteSet>> = account
        .and_then(|a| inner.user_sets.get(a))
        .into_iter()
        .flatten()
        .filter_map(|id| inner.personal_sets.get(id).cloned())
        .collect();
    let all: Vec<Arc<EmoteSet>> = personal
        .into_iter()
        .chain(channel_sets)
        .chain(inner.globals.iter().cloned())
        .collect();
    let provider = |p: Provider| provider_sections(&all, p);
    Ok(EmoteMenu {
        twitch,
        twitch_note: note,
        seventv: provider(Provider::Seventv),
        bttv: provider(Provider::Bttv),
        ffz: provider(Provider::Ffz),
    })
}

/// The channel's own emotes, public, cached for a while.
async fn channel_emotes(app: &AppHandle, channel_id: &str) -> Option<Arc<ChannelEmotes>> {
    let state = app.state::<Emotes>();
    if let Some((at, c)) = state.lock().channel_twitch.get(channel_id)
        && at.elapsed() < CHANNEL_FRESH
    {
        return Some(c.clone());
    }
    let http = app.state::<Auth>().http.clone();
    let c = Arc::new(gql::channel_emotes(&http, channel_id).await.ok()??);
    state
        .lock()
        .channel_twitch
        .insert(channel_id.to_owned(), (Instant::now(), c.clone()));
    Some(c)
}

fn provider_sections(sets: &[Arc<EmoteSet>], provider: Provider) -> Vec<MenuSection> {
    let mut out: Vec<MenuSection> = Vec::new();
    for (scope, title) in [
        (Scope::Personal, "Personal"),
        (Scope::Channel, "Channel"),
        (Scope::Global, "Global"),
    ] {
        // V105-04 Names are unique per section, so one emote under two names is listed twice, once per name.
        let mut seen = HashSet::new();
        let emotes: Vec<Emote> = sets
            .iter()
            .filter(|s| s.provider == provider && s.scope == scope)
            .flat_map(|s| s.emotes.iter())
            .filter(|e| seen.insert(e.name.clone()))
            .map(|e| (**e).clone())
            .collect();
        if !emotes.is_empty() {
            out.push(MenuSection {
                id: format!("{provider:?}-{title}").to_lowercase(),
                title: title.into(),
                avatar: None,
                emotes: open(emotes),
            });
        }
    }
    out
}

/// Sections in menu order: this channel, other channels A-Z, then Twitch's own kinds.
fn twitch_sections(
    mine: &[TwitchSet],
    channel: Option<&ChannelEmotes>,
    channel_id: &str,
) -> Vec<MenuSection> {
    let owned: HashSet<&str> = mine
        .iter()
        .flat_map(|s| s.emotes.iter().map(|e| e.id.as_str()))
        .collect();
    let mut by_owner: BTreeMap<String, MenuSection> = BTreeMap::new();
    let mut kinds: BTreeMap<u8, MenuSection> = BTreeMap::new();
    let mut here: Option<MenuSection> = None;
    // 1.40 Sets without a tier (Helix) take it from the channel's products when known.
    let tier_here: HashMap<&str, u8> = channel
        .into_iter()
        .flat_map(|c| &c.tiers)
        .flat_map(|t| t.emotes.iter().map(move |e| (e.id.as_str(), t.tier)))
        .collect();

    for set in mine {
        for e in &set.emotes {
            let emote = load::twitch_emote(e);
            let tier = (e.kind == EmoteKind::Subscription)
                .then(|| set.tier.or_else(|| tier_here.get(e.id.as_str()).copied()))
                .flatten();
            let m = MenuEmote {
                emote,
                locked: None,
                tier,
            };
            let owner = set.owner.as_ref().filter(|_| {
                matches!(
                    e.kind,
                    EmoteKind::Subscription | EmoteKind::Follower | EmoteKind::Bits | EmoteKind::Other
                )
            });
            match owner {
                Some(o) if o.id == channel_id => {
                    let s = here.get_or_insert_with(|| {
                        section(format!("twitch-{}", o.id), &o.name, o.avatar.clone())
                    });
                    push_once(s, m);
                }
                Some(o) if e.kind != EmoteKind::Bits => {
                    let s = by_owner
                        .entry(o.name.to_lowercase())
                        .or_insert_with(|| section(format!("twitch-{}", o.id), &o.name, o.avatar.clone()));
                    push_once(s, m);
                }
                _ => {
                    let (rank, id, title) = kind_section(e.kind);
                    let s = kinds
                        .entry(rank)
                        .or_insert_with(|| section(id.into(), title, None));
                    push_once(s, m);
                }
            }
        }
    }

    // E5 The channel's emotes you don't have yet.
    if let Some(c) = channel {
        let here = here.get_or_insert_with(|| {
            section(
                format!("twitch-{}", c.owner.id),
                &c.owner.name,
                c.owner.avatar.clone(),
            )
        });
        let mut lock = |e: &gql::TwitchEmote, why: String, tier: Option<u8>| {
            if !owned.contains(e.id.as_str()) && !here.emotes.iter().any(|m| m.emote.id == e.id) {
                here.emotes.push(MenuEmote {
                    emote: load::twitch_emote(e),
                    locked: Some(why),
                    tier,
                });
            }
        };
        for e in &c.follower {
            lock(e, "Follow to use".into(), None);
        }
        for t in &c.tiers {
            for e in &t.emotes {
                lock(e, format!("Tier {} sub", t.tier), Some(t.tier));
            }
        }
        for b in &c.bits {
            lock(
                &b.emote,
                format!(
                    "Cheer {} bits here",
                    cyex_core::number::grouped(u64::from(b.bits))
                ),
                None,
            );
        }
        if here.emotes.is_empty() {
            *here = section(String::new(), "", None);
        }
    }

    let mut out: Vec<MenuSection> = here.into_iter().filter(|s| !s.emotes.is_empty()).collect();
    out.extend(by_owner.into_values());
    out.extend(kinds.into_values());
    out
}

/// V105-04 A section lists each emote once: Twitch can put one emote in two sets.
fn push_once(s: &mut MenuSection, m: MenuEmote) {
    if !s.emotes.iter().any(|x| x.emote.id == m.emote.id && x.emote.name == m.emote.name) {
        s.emotes.push(m);
    }
}

fn section(id: String, title: &str, avatar: Option<String>) -> MenuSection {
    MenuSection {
        id,
        title: title.to_owned(),
        avatar,
        emotes: Vec::new(),
    }
}

/// Order, key and title for Twitch's own kinds of emotes.
fn kind_section(kind: EmoteKind) -> (u8, &'static str, &'static str) {
    match kind {
        EmoteKind::Bits => (0, "twitch-bits", "Bits"),
        EmoteKind::HypeTrain => (1, "twitch-hype", "Hype train"),
        EmoteKind::Points => (2, "twitch-points", "Unlocked with points"),
        EmoteKind::LimitedTime => (3, "twitch-limited", "Limited time"),
        EmoteKind::Prime => (4, "twitch-prime", "Prime"),
        EmoteKind::Turbo => (5, "twitch-turbo", "Turbo"),
        EmoteKind::Global => (7, "twitch-global", "Global"),
        _ => (6, "twitch-other", "Other"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_twitch::gql::{EmoteOwner, SubTier, TwitchEmote};

    fn e(id: &str, kind: EmoteKind) -> TwitchEmote {
        TwitchEmote {
            id: id.into(),
            name: format!("e{id}"),
            set_id: "s".into(),
            kind,
            animated: false,
            modifiers: Vec::new(),
        }
    }

    fn owner(id: &str, name: &str) -> Option<EmoteOwner> {
        Some(EmoteOwner {
            id: id.into(),
            login: name.to_lowercase(),
            name: name.into(),
            avatar: None,
        })
    }

    #[test]
    fn sections_put_this_channel_first_and_lock_the_rest() {
        let mine = vec![
            TwitchSet {
                id: "0".into(),
                owner: None,
                emotes: vec![e("25", EmoteKind::Global)],
                tier: None,
            },
            TwitchSet {
                id: "2".into(),
                owner: owner("9", "Zed"),
                emotes: vec![e("90", EmoteKind::Subscription)],
                tier: Some(3),
            },
            TwitchSet {
                id: "1".into(),
                owner: owner("7", "Here"),
                emotes: vec![e("70", EmoteKind::Subscription), e("71", EmoteKind::Bits)],
                tier: None,
            },
        ];
        let channel = ChannelEmotes {
            owner: owner("7", "Here").unwrap(),
            tiers: vec![SubTier {
                tier: 2,
                emotes: vec![e("70", EmoteKind::Subscription), e("72", EmoteKind::Subscription)],
            }],
            follower: Vec::new(),
            bits: Vec::new(),
        };
        let s = twitch_sections(&mine, Some(&channel), "7");
        let titles: Vec<&str> = s.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(titles, ["Here", "Zed", "Global"]);
        let here: Vec<(&str, Option<&str>, Option<u8>)> = s[0]
            .emotes
            .iter()
            .map(|m| (m.emote.id.as_str(), m.locked.as_deref(), m.tier))
            .collect();
        // 1.40 "70" has no set tier (Helix): the channel's products give it.
        assert_eq!(
            here,
            [
                ("70", None, Some(2)),
                ("71", None, None),
                ("72", Some("Tier 2 sub"), Some(2))
            ]
        );
        assert_eq!(s[1].emotes[0].tier, Some(3));
    }

    #[test]
    fn aliases_are_listed_once_each() {
        let set = Arc::new(EmoteSet::new(
            "c",
            Provider::Seventv,
            Scope::Channel,
            vec![
                Emote::new(Provider::Seventv, "A", "uncPls"),
                Emote::new(Provider::Seventv, "A", "Oldheaddance"),
                Emote::new(Provider::Seventv, "B", "x"),
                Emote::new(Provider::Seventv, "C", "x"),
            ],
        ));
        let s = provider_sections(&[set], Provider::Seventv);
        assert_eq!(s.len(), 1);
        let got: Vec<(&str, &str)> = s[0]
            .emotes
            .iter()
            .map(|m| (m.emote.id.as_str(), m.emote.name.as_str()))
            .collect();
        assert_eq!(got, [("A", "uncPls"), ("A", "Oldheaddance"), ("B", "x")]);
    }

    #[test]
    fn twitch_repeats_are_listed_once() {
        let set = |id: &str| TwitchSet {
            id: id.into(),
            owner: None,
            emotes: vec![e("25", EmoteKind::Global)],
            tier: None,
        };
        let s = twitch_sections(&[set("0"), set("1")], None, "7");
        let global = s.iter().find(|s| s.id == "twitch-global").unwrap();
        assert_eq!(global.emotes.len(), 1);
    }
}
