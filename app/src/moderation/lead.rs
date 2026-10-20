//! V107-R28 `/leadmod` and `/unleadmod`: the GQL adapter for
//! `cyex_moderation::lead_mod::set_lead`.

use cyex_moderation::lead_mod::{LeadOps, set_lead};
use cyex_moderation::role_change::RoleAction;
use cyex_twitch::gql;
use tauri::AppHandle;

use super::{Actor, http};

/// Gives or takes lead mod; `Ok` is the chat line.
pub async fn set(app: &AppHandle, actor: &Actor, who: &str, on: bool) -> Result<String, String> {
    let res = set_lead(&Adapter { app, actor, login: who }, who, on).await;
    #[cfg(feature = "mocks")]
    if std::env::var_os("CYEX_ROLE_PROBE").is_some() {
        return probe(app, actor, res).await;
    }
    res
}

/// Test builds: Twitch's own list of lead mods after the change, appended to the line, so a
/// live test can check it (`CYEX_ROLE_PROBE`).
#[cfg(feature = "mocks")]
async fn probe(app: &AppHandle, actor: &Actor, res: Result<String, String>) -> Result<String, String> {
    let held = match actor.gql() {
        Some(g) => gql::roles::holders(&http(app), g, &actor.channel_id, gql::roles::LEAD_MOD)
            .await
            .map(|l| l.join(", "))
            .unwrap_or_else(|e| format!("read failed: {e}")),
        None => "no GQL sign-in".into(),
    };
    let tag = |s: String| format!("{s} [lead mods: {held}]");
    res.map(tag).map_err(tag)
}

struct Adapter<'a> {
    app: &'a AppHandle,
    actor: &'a Actor,
    /// Whom it is about.
    login: &'a str,
}

impl LeadOps for Adapter<'_> {
    fn can(&self) -> bool {
        self.actor.gql().is_some()
    }

    async fn user_id(&self, login: &str) -> Result<Option<String>, String> {
        gql::profile(&http(self.app), Some(login), None).await.map(|p| p.map(|p| p.id)).map_err(|e| e.to_string())
    }

    async fn assign(&self, target_id: &str, on: bool) -> Result<Option<String>, String> {
        let g = self.actor.gql().ok_or(cyex_moderation::lead_mod::NEEDS_GQL)?;
        // Twitch echoes an assign (even ALREADY_ASSIGNED) as "made them a mod" on the mod
        // feeds (seen live 2026/10/20); our own line says it, so that echo adds none.
        let mark = |m: bool| super::feeds::own_role(self.app, &self.actor.channel_id, RoleAction::Mod, self.login, m);
        if on {
            mark(true);
        }
        let res = gql::roles::set_role(&http(self.app), g, &self.actor.channel_id, target_id, gql::roles::LEAD_MOD, on)
            .await
            .map_err(|e| e.to_string());
        if on && !matches!(&res, Ok(None)) && !matches!(&res, Ok(Some(c)) if c == "ALREADY_ASSIGNED") {
            mark(false);
        }
        res
    }
}
