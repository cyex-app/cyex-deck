//! BTTV and FFZ (1.38, 1.39): global and channel sets, badges, FFZ's per-user sets, and
//! BTTV's socket (C9 Pro emotes and badges, C10 emotes changing live). E2 A channel's
//! 7TV set is fetched here too, so all three retry alike.

use std::sync::Arc;

use cyex_chat::model::Provider;
use cyex_emotes::bttv::{self, socket::Event as BttvEvent};
use cyex_emotes::ffz::{self, api::RoomBadges};
use cyex_emotes::seventv;
use cyex_emotes::{EmoteSet, Scope};
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use cyex_twitch::Http;
use cyex_twitch::pace::{Lane, Service};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::UnboundedReceiver;

use super::{Emotes, looks};
use crate::settings::Settings;

/// What a global fetch brought: the sets of the providers that answered.
pub struct Globals {
    pub sets: Vec<Arc<EmoteSet>>,
    /// Asked and answered (also "for good": a final error counts, it isn't asked again).
    pub answered: Which,
    /// Failed for now: asked again later (B4).
    pub failed: Which,
}

/// B4 Global sets of those in `want` (7TV's, BTTV's and FFZ's), and BTTV's and FFZ's
/// badges and FFZ's sets for some users with them. A provider counts as failed for now
/// when its set or its badges did.
pub async fn globals(app: &AppHandle, http: &Http, want: Which) -> Globals {
    async fn ask<T>(on: bool, f: impl Future<Output = cyex_twitch::Result<T>>) -> Option<cyex_twitch::Result<T>> {
        if on { Some(f.await) } else { None }
    }
    // V110-E13 A provider whose emotes are off gets no sets; its badges still come.
    let sets = current(app).sets_off.not();
    let (s, b, f, bb, fb) = tokio::join!(
        ask(want.seventv && sets.seventv, seventv::api::global(http)),
        ask(want.bttv && sets.bttv, bttv::api::global(http)),
        ask(want.ffz && sets.ffz, ffz::api::global(http)),
        ask(want.bttv, bttv::api::badges(http)),
        ask(want.ffz, ffz::api::badges(http))
    );
    let mut sets = Vec::new();
    let mut failed = Which::default();
    match s {
        Some(Ok(set)) => sets.push(Arc::new(set)),
        Some(Err(e)) => failed.seventv |= miss(Service::SevenTv, "globals", &e),
        None => {}
    }
    match b {
        Some(Ok(set)) => sets.push(Arc::new(set)),
        Some(Err(e)) => failed.bttv |= miss(Service::Bttv, "globals", &e),
        None => {}
    }
    if let Some(Err(e)) = &bb {
        failed.bttv |= miss(Service::Bttv, "badges", e);
    }
    if let Some(Err(e)) = &fb {
        failed.ffz |= miss(Service::Ffz, "badges", e);
    }
    let state = app.state::<Emotes>();
    let mut inner = state.lock();
    match f {
        Some(Ok(g)) => {
            sets.extend(g.sets.into_iter().map(Arc::new));
            for (set, users) in g.personal {
                let id = set.id.clone();
                inner.personal_sets.insert(id.clone(), Arc::new(set));
                // B9 The whole batch stamped, then rebuilt, then one trim.
                for u in &users {
                    let list = inner.user_sets.get_or_default(u.clone());
                    if !list.contains(&id) {
                        list.push(id.clone());
                    }
                }
                for u in &users {
                    inner.rebuild_personal(u);
                }
            }
            inner.trim_personal();
        }
        Some(Err(e)) => failed.ffz |= miss(Service::Ffz, "globals", &e),
        None => {}
    }
    if let Some(Ok((defs, users))) = bb {
        for d in defs {
            inner.cosmetics.add_badge(d);
        }
        for (user, badge) in users {
            inner.cosmetics.set_bttv_badge(&user, Some(&badge));
        }
    }
    if let Some(Ok((defs, users))) = fb {
        for d in defs {
            inner.cosmetics.add_badge(d);
        }
        inner.cosmetics.set_ffz_users(users);
    }
    Globals {
        sets,
        answered: want.minus(failed),
        failed,
    }
}

/// Logs a failed global fetch; true when it's worth asking again.
fn miss(service: Service, what: &str, e: &cyex_twitch::Error) -> bool {
    let name = match service {
        Service::SevenTv => "7TV",
        Service::Bttv => "BTTV",
        _ => "FFZ",
    };
    eprintln!("emotes: no {name} {what}: {}", e.describe(name));
    service.transient(e)
}

/// E1 Which of 7TV (E2), BTTV and FFZ: to fetch, or that failed for now (429, 5xx,
/// offline). "No 7TV or BTTV user" and "no FFZ room" are answers, not failures.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Which {
    pub seventv: bool,
    pub bttv: bool,
    pub ffz: bool,
}

impl Which {
    pub const ALL: Which = Which {
        seventv: true,
        bttv: true,
        ffz: true,
    };

    /// Everything but these.
    pub fn not(self) -> Which {
        Which::ALL.minus(self)
    }

    pub fn has(self, p: Provider) -> bool {
        match p {
            Provider::Seventv => self.seventv,
            Provider::Bttv => self.bttv,
            Provider::Ffz => self.ffz,
            Provider::Twitch => false,
        }
    }

    pub fn any(self) -> bool {
        self.seventv || self.bttv || self.ffz
    }

    /// These, without those in `other`.
    pub fn minus(self, other: Which) -> Which {
        Which {
            seventv: self.seventv && !other.seventv,
            bttv: self.bttv && !other.bttv,
            ffz: self.ffz && !other.ffz,
        }
    }

    pub fn or(self, other: Which) -> Which {
        Which {
            seventv: self.seventv || other.seventv,
            bttv: self.bttv || other.bttv,
            ffz: self.ffz || other.ffz,
        }
    }
}

/// V110-E13 What the emote switches ask for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// Providers whose emotes are off: no sets, not drawn, not in the order.
    pub sets_off: Which,
    /// Providers nothing is fetched for: emotes off and, for BTTV and FFZ, badges off too.
    pub idle: Which,
}

pub fn plan(c: &ChatSettings) -> Plan {
    let on = Which { seventv: c.seventv_emotes, bttv: c.bttv_emotes, ffz: c.ffz_emotes };
    let fetch = Which { seventv: on.seventv, bttv: on.bttv || c.badges.bttv, ffz: on.ffz || c.badges.ffz };
    Plan { sets_off: on.not(), idle: fetch.not() }
}

pub fn current(app: &AppHandle) -> Plan {
    plan(&app.state::<Settings>().read::<ChatSettings>(Partition::Chat))
}

/// What a channel fetch brought.
#[derive(Default)]
pub struct Fetched {
    /// E2 The channel's 7TV account and set; `None` also when it has none.
    pub seventv: Option<seventv::api::Channel>,
    pub bttv: Option<Arc<EmoteSet>>,
    pub ffz: Option<Arc<EmoteSet>>,
    /// FFZ's mod and VIP pictures, when FFZ answered.
    pub badges: Option<RoomBadges>,
    pub failed: Which,
}

/// A channel's 7TV, BTTV and FFZ sets (those in `want`), and FFZ's mod and VIP pictures.
/// V107-P2 `lane`: the tab on screen goes ahead of background ones.
/// V110-E13 `sets_off`: those whose sets are dropped (FFZ is still asked for its badges).
pub async fn channel(http: &Http, id: &str, lane: Lane, want: Which, sets_off: Which) -> Fetched {
    let b = async {
        if want.bttv {
            Some(bttv::api::channel(http, id, lane).await)
        } else {
            None
        }
    };
    let f = async {
        if want.ffz {
            Some(ffz::api::room(http, id, lane).await)
        } else {
            None
        }
    };
    let s = async {
        if want.seventv {
            Some(seventv::api::channel(http, id, lane).await)
        } else {
            None
        }
    };
    let (s, b, f) = tokio::join!(s, b, f);
    let mut out = Fetched::default();
    match s {
        Some(Ok(ch)) => out.seventv = ch,
        Some(Err(e)) => {
            out.failed.seventv = Service::SevenTv.transient(&e);
            eprintln!("emotes: no 7TV emotes for {id}: {}", e.describe("7TV"));
        }
        None => {}
    }
    match b {
        Some(Ok(s)) => out.bttv = s.map(Arc::new),
        Some(Err(e)) => {
            out.failed.bttv = Service::Bttv.transient(&e);
            eprintln!("emotes: no BTTV emotes for {id}: {}", e.describe("BTTV"));
        }
        None => {}
    }
    match f {
        Some(Ok(Some(room))) => {
            out.ffz = room.set.filter(|_| !sets_off.ffz).map(Arc::new);
            out.badges = Some(room.badges);
        }
        Some(Ok(None)) => out.badges = Some(RoomBadges::default()),
        Some(Err(e)) => {
            out.failed.ffz = Service::Ffz.transient(&e);
            eprintln!("emotes: no FFZ emotes for {id}: {}", e.describe("FFZ"));
        }
        None => {}
    }
    out
}

pub fn spawn_bttv(app: AppHandle, mut rx: UnboundedReceiver<BttvEvent>) {
    tauri::async_runtime::spawn(async move {
        while let Some(e) = rx.recv().await {
            bttv_event(&app, e);
        }
    });
}

fn bttv_event(app: &AppHandle, e: BttvEvent) {
    let state = app.state::<Emotes>();
    match e {
        BttvEvent::User {
            twitch_id,
            badge,
            emotes,
        } => {
            let mut inner = state.lock();
            if let Some(url) = &badge {
                inner
                    .cosmetics
                    .add_badge(bttv::api::badge(url, "BetterTTV Pro", url));
            }
            let changed = inner.cosmetics.set_bttv_badge(&twitch_id, badge.as_deref());
            let personal = !emotes.is_empty();
            if personal {
                let id = format!("bttv-pro-{twitch_id}");
                inner.personal_sets.insert(
                    id.clone(),
                    Arc::new(EmoteSet::new(id.clone(), Provider::Bttv, Scope::Personal, emotes)),
                );
                let list = inner.user_sets.get_or_default(twitch_id.clone());
                if !list.contains(&id) {
                    list.push(id);
                }
                inner.rebuild_personal(&twitch_id);
                inner.trim_personal();
            }
            drop(inner);
            if personal {
                crate::chat::retokenize_users(app, std::slice::from_ref(&twitch_id));
            }
            if changed {
                looks::changed(app, [twitch_id]);
            }
        }
        BttvEvent::Added { channel_id, emote, by } => {
            let text = bttv_added_line(&emote.name, by.as_deref());
            change_channel(app, &channel_id, &text, |emotes| {
                if emotes.iter().any(|e| e.id == emote.id && e.name == emote.name) {
                    return false;
                }
                emotes.retain(|e| e.id != emote.id);
                emotes.push(Arc::new(emote.clone()));
                true
            });
        }
        BttvEvent::Renamed { channel_id, id, name } => {
            let old = find_name(app, &channel_id, &id);
            if old.as_deref() == Some(name.as_str()) {
                return;
            }
            let text = match &old {
                Some(old) => format!("The BTTV emote {old} is now {name}."),
                None => format!("A BTTV emote is now {name}."),
            };
            change_channel(app, &channel_id, &text, |emotes| {
                let Some(slot) = emotes.iter_mut().find(|e| e.id == id) else {
                    return false;
                };
                let mut e = (**slot).clone();
                e.name.clone_from(&name);
                *slot = Arc::new(e);
                true
            });
        }
        BttvEvent::Removed { channel_id, id } => {
            let Some(name) = find_name(app, &channel_id, &id) else {
                return;
            };
            let text = format!("The BTTV emote {name} was removed.");
            change_channel(app, &channel_id, &text, |emotes| {
                let before = emotes.len();
                emotes.retain(|e| e.id != id);
                emotes.len() != before
            });
        }
    }
}

/// V108-C31 BTTV names the emote's maker, not who put it in the channel.
pub(super) fn bttv_added_line(name: &str, by: Option<&str>) -> String {
    match by {
        Some(by) => format!("The BTTV emote {name} by {by} was added."),
        None => format!("The BTTV emote {name} was added."),
    }
}

fn find_name(app: &AppHandle, channel_id: &str, emote_id: &str) -> Option<String> {
    let state = app.state::<Emotes>();
    let inner = state.lock();
    inner
        .channels
        .values()
        .filter(|c| c.id == channel_id)
        .flat_map(|c| c.sets.iter())
        .filter(|s| s.provider == Provider::Bttv)
        .flat_map(|s| s.emotes.iter())
        .find(|e| e.id == emote_id)
        .map(|e| e.name.clone())
}

/// Edits the channel's BTTV set, then tells its chats (C10), unless `edit` found nothing to do
/// (V108-C31 a repeated event makes no second line).
fn change_channel(
    app: &AppHandle,
    channel_id: &str,
    notice: &str,
    edit: impl Fn(&mut Vec<Arc<cyex_emotes::Emote>>) -> bool,
) {
    let state = app.state::<Emotes>();
    let logins: Vec<String> = {
        let mut inner = state.lock();
        if inner.sets_off.bttv {
            return;
        }
        let all: Vec<String> = inner
            .channels
            .iter()
            .filter(|(_, c)| c.id == channel_id)
            .map(|(l, _)| l.clone())
            .collect();
        let mut logins = Vec::new();
        for login in &all {
            let Some(c) = inner.channels.get_mut(login) else {
                continue;
            };
            let changed;
            match c
                .sets
                .iter_mut()
                .find(|s| s.provider == Provider::Bttv && s.scope == Scope::Channel)
            {
                Some(set) => {
                    let mut next = (**set).clone();
                    changed = edit(&mut next.emotes);
                    if changed {
                        *set = Arc::new(next);
                    }
                }
                None => {
                    let mut set = EmoteSet::new(
                        format!("bttv-{channel_id}"),
                        Provider::Bttv,
                        Scope::Channel,
                        Vec::new(),
                    );
                    changed = edit(&mut set.emotes);
                    if changed {
                        c.sets.push(Arc::new(set));
                    }
                }
            }
            if changed {
                inner.rebuild(login);
                logins.push(login.clone());
            }
        }
        logins
    };
    for login in logins {
        super::change_line(app, &login, notice);
    }
}

#[cfg(test)]
mod tests {
    use super::{Which, bttv_added_line, plan};
    use cyex_settings::schema::Chat;

    #[test]
    fn v110_e13_plan_follows_the_switches() {
        let all = plan(&Chat::default());
        assert_eq!((all.sets_off, all.idle), (Which::default(), Which::default()));
        let mut c = Chat { seventv_emotes: false, bttv_emotes: false, ..Chat::default() };
        let p = plan(&c);
        assert!(p.sets_off.seventv && p.sets_off.bttv && !p.sets_off.ffz);
        // 7TV has nothing else to fetch; BTTV still has its badges.
        assert!(p.idle.seventv && !p.idle.bttv);
        c.badges.bttv = false;
        assert!(plan(&c).idle.bttv);
    }

    #[test]
    fn bttv_lines_name_the_maker_when_known() {
        assert_eq!(bttv_added_line("RareHug", Some("LIRIK")), "The BTTV emote RareHug by LIRIK was added.");
        assert_eq!(bttv_added_line("RareHug", None), "The BTTV emote RareHug was added.");
    }
}
