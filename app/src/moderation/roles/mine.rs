//! V107-Mw `/unmod` and `/unleadmod` on yourself (and the usercard's Unmod on your own
//! name): the step down My roles uses (`community::step_down`, V107-Mj). The policy is
//! `cyex_moderation::roles::own`.

use cyex_chat::model::{ModRole, NoticeKind};
use cyex_moderation::roles::{self, Leave, Outcome, Refusal, Role, VipSlots, Wanted};
use cyex_twitch::gql::dashboard::{self, community};
use tauri::AppHandle;

use super::super::{Actor, http};
use super::adapter::gql_refusal;

struct Mine<'a> {
    app: &'a AppHandle,
    actor: &'a Actor,
}

impl Leave for Mine<'_> {
    async fn leave(&self, role: Role) -> Result<(), Refusal> {
        let g = self.actor.gql().ok_or(Refusal::Line(roles::NEEDS_GQL.into()))?;
        let http = http(self.app);
        let ch = dashboard::Channel { http: &http, auth: g, base: None, id: &self.actor.account };
        community::step_down(&ch, role, &self.actor.channel_id).await.map_err(gql_refusal)
    }
}

/// Whether you hold lead mod here: the badge, else Twitch's list (a lead mod may wear the
/// plain badge).
async fn lead(app: &AppHandle, actor: &Actor) -> bool {
    if actor.role == ModRole::LeadModerator {
        return true;
    }
    let Some(g) = actor.gql() else { return false };
    let http = http(app);
    let ch = dashboard::Channel { http: &http, auth: g, base: None, id: &actor.account };
    community::lead_mod_in(&ch, &actor.channel_id).await.unwrap_or(false)
}

/// A change on yourself (`who`, as typed): a step down, or refused.
pub(super) async fn apply(app: &AppHandle, actor: &Actor, who: &str, wanted: &Wanted) -> Outcome {
    let asked: Vec<Role> = Role::ALL.into_iter().filter(|r| wanted.give.contains(r) || wanted.take.contains(r)).collect();
    let refuse = |line: &str| Outcome { done: Vec::new(), line: line.into(), slots: VipSlots::unknown(), failed: asked.clone() };
    let Some(role) = roles::own_step(wanted) else { return refuse(roles::SELF) };
    if actor.gql().is_none() {
        return refuse(roles::NEEDS_GQL);
    }
    let lead = role == Role::Mod && lead(app, actor).await;
    let out = roles::step_down(&Mine { app, actor }, role, lead, actor.role).await;
    if out.done.iter().any(|c| c.role == Role::Mod) {
        super::super::standing::changed(app, &actor.channel, who.trim_start_matches('@'), true, false);
    }
    if out.ok() {
        crate::chat::notice_line(app, &actor.channel, NoticeKind::Moderation, &out.line);
    }
    out
}
