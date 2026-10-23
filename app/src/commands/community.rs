//! V107-R40 IPC for the stream manager's Community panels: your followers, and the roles
//! you hold in other channels. GQL only, the way Twitch's dashboard reads them.

use cyex_chat::model::ModRole;
use cyex_moderation::roles::{Outcome, step_down};
use cyex_twitch::gql::dashboard::community::{self, AssignedList, Followers, Role};
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use crate::moderation::roles::Leaving;
use crate::streammanager::gql_call::gql_call;
use crate::streammanager::words;

/// A page of your followers, newest or oldest first, after `cursor`.
#[tauri::command]
pub async fn cm_followers(app: AppHandle, cursor: Option<String>, newest: bool) -> Result<Followers, String> {
    let c = gql_call(&app)?;
    community::followers(&c.channel(), cursor.as_deref(), newest)
        .await
        .map_err(|e| words::gql(&e))
}

/// Every channel where you moderate, lead mods, edit or manage business.
#[tauri::command]
pub async fn cm_assigned(app: AppHandle) -> Result<AssignedList, String> {
    let c = gql_call(&app)?;
    community::assigned(&c.channel()).await.map_err(|e| words::gql(&e))
}

/// V107-Mj What a step down left: the roles gone (their rows go), and why the rest stays.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "CmLeft")]
pub struct Left {
    pub gone: Vec<Role>,
    pub error: Option<String>,
}

/// V107-Mj A step down's rows. `lead`: the lead mod row went with mod, or Twitch said it
/// wasn't held (a stale row goes too).
fn left(out: &Outcome, lead: bool) -> Left {
    let mut gone: Vec<Role> = out.done.iter().map(|c| c.role).collect();
    if lead && gone.contains(&Role::Mod) && !gone.contains(&Role::LeadMod) {
        gone.insert(0, Role::LeadMod);
    }
    Left { gone, error: (!out.ok()).then(|| out.line.clone()) }
}

/// Leaves your own role in a channel. V107-Mj `lead`: you hold lead mod there too, so
/// stepping down as mod drops it first. V107-H3d Mod and lead mod: the policy `/unmod` and
/// `/unleadmod` on yourself use (`roles::own`), so both read the same.
#[tauri::command]
pub async fn cm_step_down(app: AppHandle, role: Role, channel_id: String, lead: bool) -> Result<Left, String> {
    let c = gql_call(&app)?;
    if !matches!(role, Role::Mod | Role::LeadMod) {
        community::step_down(&c.channel(), role, &channel_id).await.map_err(|e| words::gql(&e))?;
        return Ok(Left { gone: vec![role], error: None });
    }
    let me = Leaving { ch: c.channel(), channel_id: &channel_id };
    let lead = lead && role == Role::Mod;
    Ok(left(&step_down(&me, role, lead, ModRole::Moderator).await, lead))
}

#[cfg(test)]
mod tests {
    use cyex_moderation::roles::{Change, VipSlots};

    use super::*;

    fn out(done: &[Role], line: &str, failed: &[Role]) -> Outcome {
        let done = done.iter().map(|r| Change::new(*r, false)).collect();
        Outcome { done, line: line.into(), slots: VipSlots::unknown(), failed: failed.to_vec() }
    }

    #[test]
    fn v107_mj_mod_step_down_rows() {
        let l = left(&out(&[Role::LeadMod, Role::Mod], "You stepped down as mod and lead mod.", &[]), true);
        assert_eq!(l, Left { gone: vec![Role::LeadMod, Role::Mod], error: None });
        assert_eq!(left(&out(&[Role::Mod], "x", &[]), false).gone, [Role::Mod]);
        // NOT_ASSIGNED: the stale lead mod row goes with mod.
        assert_eq!(left(&out(&[Role::Mod], "You stepped down as mod.", &[]), true).gone, [Role::LeadMod, Role::Mod]);
        let l = left(&out(&[], "Couldn’t remove your lead mod role: Twitch didn’t allow that. You’re still a mod.", &[Role::Mod]), true);
        assert!(l.gone.is_empty());
        assert_eq!(l.error.as_deref(), Some("Couldn’t remove your lead mod role: Twitch didn’t allow that. You’re still a mod."));
        let l = left(&out(&[Role::LeadMod], "You’re no longer a lead mod. Twitch didn’t allow that.", &[Role::Mod]), true);
        assert_eq!((l.gone.as_slice(), l.error.is_some()), (&[Role::LeadMod][..], true));
    }
}
