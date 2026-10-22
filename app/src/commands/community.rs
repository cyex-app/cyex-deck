//! V107-R40 IPC for the stream manager's Community panels: your followers, and the roles
//! you hold in other channels. GQL only, the way Twitch's dashboard reads them.

use cyex_twitch::gql::dashboard::community::{self, AssignedList, Followers, ModLeft, Role};
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

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

/// V107-Mj Lines for stepping down as mod, in V107-Ma's wording.
fn mod_left(m: ModLeft) -> Left {
    let roles = |lead: bool| if lead { vec![Role::LeadMod, Role::Mod] } else { vec![Role::Mod] };
    match m {
        ModLeft::Done { lead } => Left { gone: roles(lead), error: None },
        ModLeft::LeadKept(e) => {
            let why = words::gql(&e);
            let why = why.trim_end_matches('.');
            Left { gone: vec![], error: Some(format!("Couldn’t remove your lead mod role: {why}. You’re still a mod.")) }
        }
        ModLeft::ModKept { lead: true, err } => {
            Left { gone: vec![Role::LeadMod], error: Some(format!("You’re no longer a lead mod. {}", words::gql(&err))) }
        }
        ModLeft::ModKept { lead: false, err } => Left { gone: vec![], error: Some(words::gql(&err)) },
    }
}

/// Leaves your own role in a channel. V107-Mj `lead`: you hold lead mod there too, so
/// stepping down as mod drops it first.
#[tauri::command]
pub async fn cm_step_down(app: AppHandle, role: Role, channel_id: String, lead: bool) -> Result<Left, String> {
    let c = gql_call(&app)?;
    let ch = c.channel();
    if role == Role::Mod {
        return Ok(mod_left(community::step_down_mod(&ch, &channel_id, lead).await));
    }
    community::step_down(&ch, role, &channel_id).await.map_err(|e| words::gql(&e))?;
    Ok(Left { gone: vec![role], error: None })
}

#[cfg(test)]
mod tests {
    use cyex_twitch::Error;

    use super::*;

    #[test]
    fn v107_mj_mod_step_down_lines() {
        let no = || Error::Refused("FORBIDDEN".into());
        assert_eq!(mod_left(ModLeft::Done { lead: true }), Left { gone: vec![Role::LeadMod, Role::Mod], error: None });
        assert_eq!(mod_left(ModLeft::Done { lead: false }).gone, [Role::Mod]);
        let l = mod_left(ModLeft::LeadKept(no()));
        assert!(l.gone.is_empty());
        assert_eq!(l.error.as_deref(), Some("Couldn’t remove your lead mod role: Twitch didn’t allow that. You’re still a mod."));
        let l = mod_left(ModLeft::ModKept { lead: true, err: no() });
        assert_eq!(l.gone, [Role::LeadMod]);
        assert_eq!(l.error.as_deref(), Some("You’re no longer a lead mod. Twitch didn’t allow that."));
        let l = mod_left(ModLeft::ModKept { lead: false, err: no() });
        assert_eq!((l.gone.is_empty(), l.error.as_deref()), (true, Some("Twitch didn’t allow that.")));
    }
}
