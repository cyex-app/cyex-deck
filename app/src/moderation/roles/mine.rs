//! V107-Mw `/unmod` and `/unleadmod` on yourself (and the usercard's Unmod on your own
//! name), and My roles' step down (V107-Mj): one policy, `cyex_moderation::roles::own`.

use cyex_chat::model::{ModRole, NoticeKind};
use cyex_moderation::roles::{self, Leave, Outcome, Refusal, Role, RoleAction, VipSlots, Wanted};
use cyex_twitch::gql::dashboard::{self, community};
use tauri::AppHandle;

use super::super::{Actor, http};

/// Leaves your own (`ch.id`) roles in `channel_id`.
pub struct Leaving<'a> {
    pub ch: dashboard::Channel<'a>,
    pub channel_id: &'a str,
}

impl Leave for Leaving<'_> {
    async fn leave(&self, role: Role) -> Result<(), Refusal> {
        community::step_down(&self.ch, role, self.channel_id).await.map_err(|e| match e {
            cyex_twitch::Error::Refused(code) => Refusal::Code(code),
            // V107-H3c Worded like the dashboard's ("Couldn’t reach Twitch."), not the raw error.
            e => Refusal::Line(crate::streammanager::words::gql(&e)),
        })
    }
}

/// V107-H3f From chat: your unmod's echo in the mod feed is marked ours, so the step down
/// is one line (it also showed "x removed x as mod.").
struct Marked<'a> {
    app: &'a AppHandle,
    me: Leaving<'a>,
    login: &'a str,
}

impl Leave for Marked<'_> {
    async fn leave(&self, role: Role) -> Result<(), Refusal> {
        let mark = |on| super::super::feeds::own_role(self.app, self.me.channel_id, RoleAction::Unmod, self.login, on);
        if role == Role::Mod {
            mark(true);
        }
        let res = self.me.leave(role).await;
        if role == Role::Mod && res.is_err() {
            mark(false);
        }
        res
    }
}

/// A change on yourself (`who`, as typed): a step down, or refused.
pub(super) async fn apply(app: &AppHandle, actor: &Actor, who: &str, wanted: &Wanted) -> Outcome {
    let asked: Vec<Role> = Role::ALL.into_iter().filter(|r| wanted.give.contains(r) || wanted.take.contains(r)).collect();
    let refuse = |line: &str| Outcome { done: Vec::new(), line: line.into(), slots: VipSlots::unknown(), failed: asked.clone() };
    let Some(role) = roles::own_step(wanted) else { return refuse(roles::SELF) };
    let Some(g) = actor.gql() else { return refuse(roles::NEEDS_GQL) };
    let http = http(app);
    let me = Leaving { ch: dashboard::Channel { http: &http, auth: g, base: None, id: &actor.account }, channel_id: &actor.channel_id };
    // The badge, else Twitch's list (a lead mod may wear the plain badge).
    let lead = role == Role::Mod
        && (actor.role == ModRole::LeadModerator || community::lead_mod_in(&me.ch, &actor.channel_id).await.unwrap_or(false));
    let login = who.trim().trim_start_matches('@');
    let out = roles::step_down(&Marked { app, me, login }, role, lead, actor.role).await;
    if out.done.iter().any(|c| c.role == Role::Mod) {
        super::super::standing::changed(app, &actor.channel, login, true, false);
    }
    if out.ok() {
        let others = roles::as_seen_by_others(&out.line, login);
        crate::chat::notice_line_by(app, &actor.account, &actor.channel, NoticeKind::Moderation, &out.line, &others);
    }
    out
}
