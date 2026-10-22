//! V107-R42 The app's side of `cyex_moderation::roles`: Twitch (Helix for the broadcaster's
//! mod and VIP when the token has the scope, else the dashboard's GQL), the mod feeds'
//! own-echo marks and the role buttons, for one actor.

use std::sync::Mutex;

use cyex_chat::model::ModRole;
use cyex_moderation::roles::{self, Change, Chat, Path, Refusal, Role, RoleAction, Twitch};
use cyex_twitch::gql::{self, dashboard};
use cyex_twitch::helix::roles::{self as hr, Role as HRole};
use tauri::AppHandle;

use super::super::{Actor, codes, http};

pub(super) struct Adapter<'a> {
    pub app: &'a AppHandle,
    pub actor: &'a Actor,
    /// The last login looked up, and its user id (Helix and channel roles want the id).
    id: Mutex<Option<(String, String)>>,
}

impl<'a> Adapter<'a> {
    pub fn new(app: &'a AppHandle, actor: &'a Actor) -> Adapter<'a> {
        Adapter { app, actor, id: Mutex::default() }
    }

    /// 1.50 Helix takes only the broadcaster's token, with the role's scope.
    fn helix(&self, role: Role) -> Option<cyex_twitch::helix::Auth<'_>> {
        let scope = match role {
            Role::Mod => "channel:manage:moderators",
            Role::Vip => "channel:manage:vips",
            _ => return None,
        };
        if self.actor.role == ModRole::Broadcaster { self.actor.helix_scoped(self.app, scope) } else { None }
    }

    /// V107-Mw Whether `who` is the acting account (a failed lookup: no).
    pub async fn is_me(&self, who: &str) -> bool {
        self.user_id(who).await.is_ok_and(|id| id == self.actor.account)
    }

    /// `who`'s user id; `Code(TARGET_NOT_FOUND)` when nobody has the login.
    async fn user_id(&self, who: &str) -> Result<String, Refusal> {
        if let Some((l, id)) = self.id.lock().unwrap_or_else(|p| p.into_inner()).as_ref()
            && l == who
        {
            return Ok(id.clone());
        }
        let id = gql::profile(&http(self.app), Some(who), None)
            .await
            .map_err(|e| Refusal::Line(e.to_string()))?
            .map(|p| p.id)
            .ok_or_else(|| Refusal::Code("TARGET_NOT_FOUND".into()))?;
        *self.id.lock().unwrap_or_else(|p| p.into_inner()) = Some((who.to_owned(), id.clone()));
        Ok(id)
    }
}

/// A GQL failure: Twitch's code when it gave one; GQL's own "forbidden" reads as that code,
/// so a plain mod hears who may (`roles::refused`).
pub(super) fn gql_refusal(e: cyex_twitch::Error) -> Refusal {
    match e {
        cyex_twitch::Error::Refused(code) => Refusal::Code(code),
        e => {
            let text = e.to_string();
            let up = text.to_ascii_uppercase();
            if ["FORBIDDEN", "UNAUTHORIZED", "PERMISSION"].iter().any(|w| up.contains(w)) {
                Refusal::Code("FORBIDDEN".into())
            } else {
                Refusal::Line(text)
            }
        }
    }
}

impl Twitch for Adapter<'_> {
    fn by(&self) -> ModRole {
        self.actor.role
    }

    fn path(&self, role: Role) -> Result<Path, &'static str> {
        roles::access(self.actor.role, role, self.helix(role).is_some(), self.actor.gql().is_some())
    }

    async fn assign(&self, who: &str, role: Role, on: bool) -> Result<(), Refusal> {
        // V107-N2f Not on yourself (the broadcaster is refused in `apply`): a swap takes your
        // own mod role first and then can't give or put back anything (seen live 2026/10/21).
        if self.user_id(who).await? == self.actor.account {
            return Err(Refusal::Code("TARGET_IS_SELF".into()));
        }
        let http = http(self.app);
        match self.path(role).map_err(|l| Refusal::Line(l.into()))? {
            Path::Helix => {
                let h = self.helix(role).ok_or(Refusal::Line(roles::NO_TOKEN.into()))?;
                let hrole = if role == Role::Mod { HRole::Mod } else { HRole::Vip };
                let id = self.user_id(who).await?;
                hr::set_role(&http, h, &self.actor.channel_id, &id, hrole, on).await.map_err(|e| {
                    let status = match &e {
                        cyex_twitch::Error::Api { status, .. } => *status,
                        _ => 0,
                    };
                    let said = e.api_message().and_then(cyex_moderation::said::code);
                    match roles::helix_code(Change::new(role, on), status).or(said) {
                        Some(code) => Refusal::Code(code.into()),
                        None => Refusal::Line(codes::helix(status, e.api_message().unwrap_or(""))),
                    }
                })
            }
            Path::Gql => {
                let g = self.actor.gql().ok_or(Refusal::Line(roles::NEEDS_GQL.into()))?;
                let id = if role.by_id() { self.user_id(who).await? } else { String::new() };
                let ch = dashboard::Channel { http: &http, auth: g, base: None, id: &self.actor.channel_id };
                let target = dashboard::roles::Who { login: who, id: &id };
                dashboard::roles::assign(&ch, role, target, on).await.map_err(gql_refusal)
            }
        }
    }
}

impl Chat for Adapter<'_> {
    fn mark(&self, action: RoleAction, who: &str, on: bool) {
        super::super::feeds::own_role(self.app, &self.actor.channel_id, action, who, on);
    }

    fn standing(&self, who: &str, moderator: bool, on: bool) {
        super::super::standing::changed(self.app, &self.actor.channel, who, moderator, on);
    }
}
