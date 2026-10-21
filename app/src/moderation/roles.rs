//! V107-R36 The Roles Manager (stream manager panel) for the broadcaster's own channel, and
//! the VIP slots the usercard shows. Reads and the editor/artist/agent mutations are GQL
//! (`cyex_twitch::gql::roles_list`, `roles`); mod and VIP go through the usercard's role
//! sequence (`run::set_role`), lead mod through `lead::set`. The plan and lines are
//! `cyex_moderation::roles_manager`.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cyex_chat::model::ModRole;
use cyex_moderation::roles_manager::{self as rm, ManageOps, Member, Role, Step, VipSlots};
use cyex_twitch::gql::{self, roles_list};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{Actor, http};

/// What the panel shows.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RolesView {
    pub members: Vec<Member>,
    pub vips: VipSlots,
}

const NEEDS_GQL: &str = "The Roles Manager needs a GQL sign-in. Add one in Settings → Accounts.";

/// The broadcaster acting in their own channel: their id is the channel's.
fn own(app: &AppHandle, account: &str, login: &str) -> Result<Actor, String> {
    let gql = crate::auth::gql_token(app, account).ok_or(NEEDS_GQL)?;
    Ok(Actor {
        account: account.to_owned(),
        channel: login.to_ascii_lowercase(),
        channel_id: account.to_owned(),
        role: ModRole::Broadcaster,
        helix: crate::auth::helix_token(app, account),
        gql: Some(gql),
    })
}

/// The last VIP slot count per account, for usercards (one read a minute at most).
#[derive(Default)]
pub struct SlotCache(Mutex<HashMap<String, (Instant, VipSlots)>>);

const SLOTS_FOR: Duration = Duration::from_secs(60);

fn remember(app: &AppHandle, account: &str, s: VipSlots) {
    if let Some(c) = app.try_state::<SlotCache>() {
        c.0.lock().unwrap().insert(account.to_owned(), (Instant::now(), s));
    }
}

/// Everyone with a role, and the VIP slots.
pub async fn view(app: &AppHandle, account: &str, login: &str) -> Result<RolesView, String> {
    let actor = own(app, account, login)?;
    let g = actor.gql().ok_or(NEEDS_GQL)?;
    let http = http(app);
    let (lists, slots) = tokio::join!(
        roles_list::role_lists(&http, g, &actor.channel_id),
        roles_list::slots(&http, g, &actor.channel_id),
    );
    let lists = lists.map_err(|e| e.to_string())?;
    let allowed = slots.ok().and_then(|s| s.vips);
    let vips = VipSlots::new(lists.vips.len() as u32, allowed);
    remember(app, account, vips);
    Ok(RolesView { members: rm::merge(&lists), vips })
}

/// One change, then the lists read again. `holds`: what the row showed.
pub async fn set(app: &AppHandle, account: &str, login: &str, who: &str, role: Role, on: bool, holds: Vec<Role>) -> Result<RolesView, String> {
    let actor = own(app, account, login)?;
    let who = who.trim().trim_start_matches('@').to_ascii_lowercase();
    if who.is_empty() {
        return Err("Type a name.".into());
    }
    // No slot to fill: nothing is taken away first (Twitch says MAX_VIPS_REACHED even with
    // none unlocked, seen 2026/10/21).
    if role == Role::Vip && on && !holds.contains(&Role::Vip) {
        let now = view(app, account, login).await?;
        if let Some(why) = now.vips.blocked() {
            return Err(why.into());
        }
    }
    let res = rm::change(&Adapter { app, actor: &actor }, &who, role, on, &holds).await;
    let mut v = view(app, account, login).await?;
    if let Err(e) = res {
        // Twitch gave no slot count: a refused grant tells.
        if role == Role::Vip && on && v.vips.allowed.is_none() {
            let code = if e == super::codes::gql("MAX_VIPS_REACHED", &who) {
                "MAX_VIPS_REACHED"
            } else if e == super::codes::gql("VIP_ACHIEVEMENT_INCOMPLETE", &who) {
                "VIP_ACHIEVEMENT_INCOMPLETE"
            } else {
                ""
            };
            v.vips = v.vips.learn(code);
            remember(app, account, v.vips);
        }
        return Err(e);
    }
    Ok(v)
}

/// The usercard's VIP slots in the broadcaster's own channel; `None` elsewhere.
pub async fn vip_slots(app: &AppHandle, account: &str, login: &str) -> Result<Option<VipSlots>, String> {
    if crate::chat::roles::role(app, account, login) != Some(ModRole::Broadcaster) {
        return Ok(None);
    }
    if let Some(c) = app.try_state::<SlotCache>()
        && let Some((at, s)) = c.0.lock().unwrap().get(account)
        && at.elapsed() < SLOTS_FOR
    {
        return Ok(Some(*s));
    }
    view(app, account, login).await.map(|v| Some(v.vips))
}

/// Forgets the cached count after a VIP change elsewhere (usercard, `/vip`).
pub fn forget(app: &AppHandle, account: &str) {
    if let Some(c) = app.try_state::<SlotCache>() {
        c.0.lock().unwrap().remove(account);
    }
}

struct Adapter<'a> {
    app: &'a AppHandle,
    actor: &'a Actor,
}

impl ManageOps for Adapter<'_> {
    async fn step(&self, who: &str, s: Step) -> Result<(), String> {
        let (app, a) = (self.app, self.actor);
        let g = a.gql().ok_or(NEEDS_GQL)?;
        let refused = |r: GqlResult| match r {
            Ok(None) => Ok(()),
            Ok(Some(code)) => Err(rm::refused(&code, who, s.role, s.on)),
            Err(e) => Err(e.to_string()),
        };
        match s.role {
            Role::Mod | Role::Vip => super::run::set_role(app, a, who, s.role == Role::Mod, s.on).await,
            Role::LeadMod => super::lead::set(app, a, who, s.on).await.map(|_| ()),
            Role::Editor => refused(gql::roles::set_editor(&http(app), g, &a.channel_id, who, s.on).await),
            Role::Artist => refused(gql::roles::set_artist(&http(app), g, &a.channel_id, who, s.on).await),
            Role::Agent => {
                let id = gql::profile(&http(app), Some(who), None)
                    .await
                    .map_err(|e| e.to_string())?
                    .map(|p| p.id)
                    .ok_or_else(|| format!("No user called {who}."))?;
                refused(gql::roles::set_role(&http(app), g, &a.channel_id, &id, gql::roles::AGENT, s.on).await)
            }
        }
    }
}

type GqlResult = cyex_twitch::Result<Option<String>>;
