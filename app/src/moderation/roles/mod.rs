//! V107-R42 Every channel role change in the app goes through [`apply`]: the Roles Manager
//! (`manager`), the usercard's Mod and VIP buttons and `/mod` `/unmod` `/vip` `/unvip`
//! `/leadmod` `/unleadmod` ([`command`]). The policy is `cyex_moderation::roles::change`;
//! this side gives it Twitch and the chat (`adapter`), what they hold, the VIP slots, and
//! shows its one line.

mod adapter;
mod mine;
pub mod manager;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cyex_chat::model::{ModRole, NoticeKind};
use cyex_moderation::roles::{self, Echoed, Outcome, Role, VipSlots, Wanted};
use tauri::{AppHandle, Manager};

use super::Actor;
use adapter::Adapter;

/// The last Roles Manager read per account: its rows and VIP slots, for a change's holds
/// and the usercard's slots (one read a minute at most).
#[derive(Default)]
pub struct ViewCache(Mutex<Views>);

const FRESH_FOR: Duration = Duration::from_secs(60);

type Views = HashMap<String, (Instant, manager::RolesView)>;

fn with_cache<R>(app: &AppHandle, f: impl FnOnce(&mut Views) -> R) -> Option<R> {
    let c = app.try_state::<ViewCache>()?;
    let mut g = c.0.lock().unwrap_or_else(|p| p.into_inner());
    Some(f(&mut g))
}

fn remember(app: &AppHandle, account: &str, v: &manager::RolesView) {
    with_cache(app, |c| c.insert(account.to_owned(), (Instant::now(), v.clone())));
}

/// The cached read, while fresh.
fn cached(app: &AppHandle, account: &str) -> Option<manager::RolesView> {
    with_cache(app, |c| c.get(account).filter(|(at, _)| at.elapsed() < FRESH_FOR).map(|(_, v)| v.clone())).flatten()
}

/// What a change taught about the slots, kept with the cached rows.
fn learned(app: &AppHandle, account: &str, s: &VipSlots) {
    with_cache(app, |c| {
        if let Some((_, v)) = c.get_mut(account) {
            v.vips = s.clone();
        }
    });
}

/// Forgets the cached read (a role moved and the slots weren't known).
fn forget(app: &AppHandle, account: &str) {
    with_cache(app, |c| c.remove(account));
}

/// Whether the actor is the broadcaster in their own channel: only there are the slots read.
fn own(actor: &Actor) -> bool {
    actor.role == ModRole::Broadcaster && actor.channel_id == actor.account
}

/// One change for `who`. `holds`: what they hold as the caller saw it (`None`: their badges
/// are read); `slots`: the channel's VIP slots when known (`None`: the cache, or a read in
/// your own channel when a VIP is given). Steps that went through tell the role buttons; the
/// line goes to the channel's chats when a mod, VIP or lead mod change went through (their
/// feed echoes are marked ours).
pub async fn apply(app: &AppHandle, actor: &Actor, who: &str, holds: Option<Vec<Role>>, slots: Option<VipSlots>, wanted: &Wanted) -> Outcome {
    if let Some(line) = broadcaster_line(&actor.channel, who, actor.role) {
        let failed = Role::ALL.into_iter().filter(|r| wanted.give.contains(r) || wanted.take.contains(r)).collect();
        return Outcome { done: Vec::new(), line: line.into(), slots: slots.unwrap_or_else(VipSlots::unknown), failed };
    }
    let a = Adapter::new(app, actor);
    // V107-Mw Yourself (the broadcaster is caught above): a step down, or refused.
    if actor.role != ModRole::Broadcaster && a.is_me(who).await {
        return mine::apply(app, actor, who, wanted).await;
    }
    let unmod = wanted.take.contains(&Role::Mod) && !wanted.take.contains(&Role::LeadMod);
    let holds = match holds {
        Some(h) => h,
        // Only a grant can clash; V107-Ma an unmod needs to know about lead mod.
        None if !wanted.give.is_empty() || unmod => {
            let mut h = super::standing::holds(app, &actor.channel, who).await.unwrap_or_default();
            if unmod && !h.contains(&Role::LeadMod) && lead_mod(app, actor, who).await {
                h.push(Role::LeadMod);
            }
            h
        }
        None => Vec::new(),
    };
    let gives_vip = wanted.give.contains(&Role::Vip);
    let read = slots.is_some() || (gives_vip && own(actor));
    let slots = match slots {
        Some(s) => s,
        None if gives_vip && own(actor) => match cached(app, &actor.account) {
            Some(v) => v.vips,
            None => manager::view(app, &actor.account, &actor.channel).await.map(|v| v.vips).unwrap_or_else(|_| VipSlots::unknown()),
        },
        None => VipSlots::unknown(),
    };
    let out = roles::change(&Echoed { twitch: &a, chat: &a }, who, &holds, slots, wanted).await;
    let vip_moved = out.done.iter().any(|c| c.role == Role::Vip) || wanted.give.contains(&Role::Vip);
    if own(actor) && vip_moved {
        if read { learned(app, &actor.account, &out.slots) } else { forget(app, &actor.account) }
    }
    let chat_role = out.done.iter().any(|c| matches!(c.role, Role::Mod | Role::Vip | Role::LeadMod));
    if out.ok() && chat_role {
        crate::chat::notice_line(app, &actor.channel, NoticeKind::Moderation, &out.line);
    }
    out
}

/// V107-Ma Whether `who` is a lead mod when their badges don't say (a lead mod may wear the
/// plain badge, or none): the Roles Manager's fresh read, else Twitch's list, read once.
/// Only the broadcaster can read or change it; elsewhere, `false`.
async fn lead_mod(app: &AppHandle, actor: &Actor, who: &str) -> bool {
    if actor.role != ModRole::Broadcaster {
        return false;
    }
    if own(actor)
        && let Some(v) = cached(app, &actor.account)
    {
        return v.members.iter().any(|m| m.login.eq_ignore_ascii_case(who) && m.roles.contains(&Role::LeadMod));
    }
    let Some(g) = actor.gql() else { return false };
    let http = super::http(app);
    let ch = cyex_twitch::gql::dashboard::Channel { http: &http, auth: g, base: None, id: &actor.channel_id };
    cyex_twitch::gql::dashboard::roles::holders(&ch, Role::LeadMod)
        .await
        .is_ok_and(|l| l.iter().any(|l| l.eq_ignore_ascii_case(who)))
}

/// V107-N4A The channel's own broadcaster holds no role: Twitch answers that with a bare
/// "Internal Server Error", so it's refused before anything is sent.
fn broadcaster_line(channel: &str, who: &str, by: ModRole) -> Option<&'static str> {
    let who = who.trim().trim_start_matches('@');
    if !who.eq_ignore_ascii_case(channel.trim().trim_start_matches('#')) {
        return None;
    }
    Some(if by == ModRole::Broadcaster { "That’s you." } else { "That’s the broadcaster." })
}

/// A typed or usercard role command. Its line is in chat already (`apply`); `Err` is the
/// line when it didn't all go through.
pub async fn command(app: &AppHandle, actor: &Actor, who: &str, role: Role, on: bool) -> Result<Option<String>, String> {
    let out = apply(app, actor, who, None, None, &Wanted::one(role, on)).await;
    #[cfg(feature = "mocks")]
    if role == Role::LeadMod && std::env::var_os("CYEX_ROLE_PROBE").is_some() {
        return probe(app, actor, out).await;
    }
    if out.ok() { Ok(None) } else { Err(out.line) }
}

/// Test builds: Twitch's own list of lead mods after the change, appended to the line, so a
/// live test can check it (`CYEX_ROLE_PROBE`).
#[cfg(feature = "mocks")]
async fn probe(app: &AppHandle, actor: &Actor, out: Outcome) -> Result<Option<String>, String> {
    let held = match actor.gql() {
        Some(g) => {
            let http = super::http(app);
            let ch = cyex_twitch::gql::dashboard::Channel { http: &http, auth: g, base: None, id: &actor.channel_id };
            cyex_twitch::gql::dashboard::roles::holders(&ch, Role::LeadMod)
                .await
                .map(|l| l.join(", "))
                .unwrap_or_else(|e| format!("read failed: {e}"))
        }
        None => "no GQL sign-in".into(),
    };
    let line = format!("{} [lead mods: {held}]", out.line);
    if out.ok() { Ok(Some(line)) } else { Err(line) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_n4a_no_role_for_the_broadcaster() {
        assert_eq!(broadcaster_line("cyexbot", "CyexBot", ModRole::Broadcaster), Some("That’s you."));
        assert_eq!(broadcaster_line("#cyexbot", "@cyexbot", ModRole::LeadModerator), Some("That’s the broadcaster."));
        assert_eq!(broadcaster_line("cyexbot", "caressz", ModRole::Broadcaster), None);
    }
}
