//! Moderation (spec 09) wired to Tauri. Actions go through Helix when the account has the
//! mod scopes, else the matching raw GQL mutations, so a GQL-only account can still
//! moderate (D25). The role gate (1.57) decides whether any mod control shows. EventSub
//! feeds live in `feeds`; the mod view in `view`; mass actions in `massban` and `nuke`.

pub mod codes;
pub mod feeds;
pub mod massban;
pub mod nuke;
pub mod queue;
pub mod restack;
pub mod run;
pub mod standing;
pub mod view;

use cyex_chat::model::ModRole;
use cyex_moderation::command::Need;
use cyex_twitch::gql;
use cyex_twitch::helix;
use tauri::{AppHandle, Manager};

use crate::auth::{self, Auth};

pub fn init(app: &AppHandle) {
    view::init(app);
    feeds::init(app);
    massban::init(app);
}

/// The account's role in a channel (1.57, V102-21): the one source, kept live in
/// `chat::roles`.
pub fn role(app: &AppHandle, account: &str, login: &str) -> Option<ModRole> {
    crate::chat::roles::role(app, account, login)
}

/// The channel id for a login, from an open room's state or the chat catalog.
pub fn channel_id(app: &AppHandle, login: &str) -> Option<String> {
    crate::chat::channel_id_of(app, login)
        .or_else(|| crate::chat::catalog::cached(app, &login.to_ascii_lowercase()).map(|c| c.channel_id.clone()))
}

fn http(app: &AppHandle) -> cyex_twitch::Http {
    app.state::<Auth>().http.clone()
}

/// How an account moderates: its tokens and ids, resolved once per action.
pub struct Actor {
    pub account: String,
    /// The channel's login, as the command named it.
    pub channel: String,
    pub channel_id: String,
    pub role: ModRole,
    helix: Option<(String, String)>,
    gql: Option<(String, String)>,
}

impl Actor {
    /// The Helix auth, only when the token carries `scope`.
    pub fn helix_scoped<'a>(&'a self, app: &AppHandle, scope: &str) -> Option<helix::Auth<'a>> {
        auth::helix_scoped(app, &self.account, scope)?;
        self.helix.as_ref().map(|(token, client_id)| crate::auth::helix_auth(token, client_id))
    }

    /// Any usable Helix mod token (our sign-in asks for every mod scope at once, so a
    /// Helix token has them; a pasted one short of a scope gets a 403 from Twitch).
    pub fn helix_any(&self) -> Option<(helix::Auth<'_>, helix::Room<'_>)> {
        self.helix.as_ref().map(|(token, client_id)| {
            (crate::auth::helix_auth(token, client_id), self.room())
        })
    }

    pub fn gql(&self) -> Option<gql::Auth<'_>> {
        self.gql.as_ref().map(|(token, client_id)| crate::auth::gql_auth(token, client_id))
    }

    pub fn room(&self) -> helix::Room<'_> {
        helix::Room {
            broadcaster_id: &self.channel_id,
            moderator_id: &self.account,
        }
    }
}

/// Resolves the acting account's tokens for a channel, or says why it can't moderate.
pub fn actor(app: &AppHandle, account: &str, login: &str) -> Result<Actor, String> {
    actor_for(app, account, login, Need::Mod)
}

/// Like `actor`, for an action that needs more than a mod (1.50, raids, ads). Every
/// mod-only action passes `Need::allows` here (V102-21).
pub fn actor_for(app: &AppHandle, account: &str, login: &str, need: Need) -> Result<Actor, String> {
    let role = role(app, account, login);
    if !need.allows(role) {
        return Err(need.refusal().into());
    }
    let role = role.expect("allowed roles are set");
    let channel_id = channel_id(app, login).ok_or("Still loading this channel.")?;
    let helix = auth::helix_token(app, account);
    let gql = auth::gql_token(app, account);
    if helix.is_none() && gql.is_none() {
        return Err("This account can’t moderate. Sign in again in Settings → Accounts.".into());
    }
    Ok(Actor {
        account: account.to_owned(),
        channel: login.to_ascii_lowercase(),
        channel_id,
        role,
        helix,
        gql,
    })
}
