//! 1.41 What an emote card shows: where the emote comes from, who made and added it,
//! and a link to its page. Local facts first, the rest fetched when the card opens.

use cyex_chat::model::Provider;
use cyex_emotes::{Emote, Scope, seventv};
use cyex_twitch::gql::{self, EmoteKind};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::Emotes;
use crate::auth::Auth;

#[derive(Serialize, TS)]
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
}

/// Facts about an emote. `login` is the chat it was clicked in.
pub async fn card(
    app: &AppHandle,
    login: &str,
    provider: Provider,
    id: &str,
    name: &str,
) -> Result<EmoteCard, String> {
    let http = app.state::<Auth>().http.clone();
    if provider == Provider::Twitch {
        return twitch(&http, id, name).await;
    }
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
        page: page(provider, id),
    };
    match provider {
        Provider::Seventv => {
            let (owner, names) = tokio::join!(
                async {
                    match &card.owner {
                        Some(o) => Some(o.clone()),
                        None => seventv::api::owner_name(&http, id).await.ok().flatten(),
                    }
                },
                async {
                    let ids: Vec<String> = local.added_by.iter().cloned().collect();
                    seventv::api::user_names(&http, &ids).await.unwrap_or_default()
                }
            );
            card.owner = owner;
            card.added_by = local.added_by.as_ref().and_then(|a| names.get(a).cloned());
        }
        _ => super::providers::card(&http, &mut card).await,
    }
    Ok(card)
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
        page: None,
    };
    let Some(info) = info else { return Ok(card) };
    card.animated = info.emote.animated;
    card.owner = info.artist;
    card.kind = Some(match (info.emote.kind, info.tier, info.bits) {
        (_, Some(t), _) => format!("Tier {t}"),
        (_, _, Some(b)) => format!("Bits, {} cheered", cyex_core::number::grouped(u64::from(b))),
        (EmoteKind::Follower, ..) => "Follower".into(),
        (EmoteKind::Global, ..) => "Global emote".into(),
        (EmoteKind::Prime, ..) => "Prime".into(),
        (EmoteKind::Turbo, ..) => "Turbo".into(),
        (EmoteKind::HypeTrain, ..) => "Hype train".into(),
        (EmoteKind::Points, ..) => "Unlocked with points".into(),
        (EmoteKind::LimitedTime, ..) => "Limited time".into(),
        _ => "Twitch emote".into(),
    });
    if let Some(o) = info.owner {
        card.page = Some(format!("https://www.twitch.tv/{}", o.login));
        card.channel = Some(o.name);
    }
    Ok(card)
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

fn page(provider: Provider, id: &str) -> Option<String> {
    match provider {
        Provider::Seventv => Some(format!("https://7tv.app/emotes/{id}")),
        Provider::Bttv => Some(format!("https://betterttv.com/emotes/{id}")),
        Provider::Ffz => Some(format!("https://www.frankerfacez.com/emoticon/{id}")),
        Provider::Twitch => None,
    }
}
