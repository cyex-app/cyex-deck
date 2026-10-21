//! V107-R42 Every channel role change in one module: the Roles Manager (V107-R36), the
//! usercard's Mod and VIP buttons (V107-M8), `/mod` `/unmod` `/vip` `/unvip` (1.50) and
//! `/leadmod` `/unleadmod` (V107-R28) all go through [`change`]. The policy is here and only
//! here: the order (a VIP loses VIP before becoming a mod or lead mod, a mod loses mod
//! before becoming a VIP; Twitch makes a lead mod a mod, so that one isn't sent), the
//! rollback when a step fails (the VIP goes back), VIP slots (checked before sending,
//! learned from Twitch's codes) and one table for what Twitch's codes read as (`lines`).
//!
//! The app gives [`change`] a [`Twitch`] port (Helix or GQL, by [`access`]); [`Echoed`]
//! wraps it so the mod feeds' echoes of our own steps add no line (the internal seam).

mod change;
mod echo;
mod lines;
mod members;
mod slots;

use cyex_chat::model::ModRole;

pub use change::change;
pub use cyex_twitch::gql::dashboard::roles::Role;
pub use echo::{Chat, Echoed, RoleAction, holds_of};
pub use lines::refused;
pub use members::{Member, merge};
pub use slots::{FULL, LOCKED, VipSlots, VipState};

use crate::command::Need;

/// What one change asks for: roles to give and roles to take, for one person.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wanted {
    pub give: Vec<Role>,
    pub take: Vec<Role>,
}

impl Wanted {
    /// One role given (`on`) or taken.
    pub fn one(role: Role, on: bool) -> Wanted {
        if on { Wanted { give: vec![role], take: vec![] } } else { Wanted { give: vec![], take: vec![role] } }
    }
}

/// One mutation: give (`on`) or take `role`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    pub role: Role,
    pub on: bool,
}

impl Change {
    pub const fn new(role: Role, on: bool) -> Change {
        Change { role, on }
    }

    fn undo(self) -> Change {
        Change { on: !self.on, ..self }
    }
}

/// What a change came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// What stuck, in order (a step rolled back isn't here).
    pub done: Vec<Change>,
    /// The one line about it all: what happened, or why not.
    pub line: String,
    /// The VIP slots after it, with what Twitch's answers taught.
    pub slots: VipSlots,
    /// Asked-for roles that didn't change. Empty: it all went through.
    pub failed: Vec<Role>,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Why Twitch said no to one step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Twitch's code: GQL's, or a Helix status read as one ([`helix_code`]).
    Code(String),
    /// Said as it is: no connection, Twitch's own Helix message.
    Line(String),
}

/// Which API a step goes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Path {
    /// `POST/DELETE /moderation/moderators` or `/channels/vips`.
    Helix,
    /// The dashboard's mutations (`cyex_twitch::gql::dashboard::roles::assign`).
    Gql,
}

/// Lead mods without a GQL sign-in: Helix won't take their token.
pub const LEAD_NEEDS_GQL: &str = "Lead mods change mods and VIPs with a GQL sign-in. Add one in Settings → Accounts.";
/// Lead mod, editor, artist and agent: GQL only (Helix has no endpoint, or not for us).
pub const NEEDS_GQL: &str = "That needs a GQL sign-in. Add one in Settings → Accounts.";
/// Signed in, but neither token can do it.
pub const NO_TOKEN: &str = "This account can’t moderate. Sign in again in Settings → Accounts.";

/// 1.50 How `by` changes `role`. Mod and VIP: Helix only takes the broadcaster's token (a
/// lead mod gets 401), so lead mods go through GQL like twitch.tv; a lead mod may wear the
/// plain mod badge, so a mod's attempt goes to Twitch too and Twitch decides. The rest are
/// GQL. `helix_scope`: a Helix token with the matching `channel:manage:*` scope.
pub fn access(by: ModRole, role: Role, helix_scope: bool, gql: bool) -> Result<Path, &'static str> {
    if !matches!(role, Role::Mod | Role::Vip) {
        return if gql { Ok(Path::Gql) } else { Err(NEEDS_GQL) };
    }
    match by {
        ModRole::Broadcaster if helix_scope => Ok(Path::Helix),
        _ if gql => Ok(Path::Gql),
        ModRole::Broadcaster => Err(NO_TOKEN),
        ModRole::LeadModerator => Err(LEAD_NEEDS_GQL),
        ModRole::Moderator => Err(Need::Lead.refusal()),
    }
}

/// A Helix status read as the GQL code it means, where one does (Helix reference): 422 on a
/// grant, they hold the other role; 409 on a VIP grant, no slot; 425, VIPs not unlocked.
pub fn helix_code(c: Change, status: u16) -> Option<&'static str> {
    match (c.role, c.on, status) {
        (Role::Mod, true, 422) => Some("TARGET_IS_VIP"),
        (Role::Vip, true, 422) => Some("GRANTEE_IS_MOD"),
        (Role::Vip, true, 409) => Some("MAX_VIPS_REACHED"),
        (Role::Vip, true, 425) => Some("VIP_ACHIEVEMENT_INCOMPLETE"),
        _ => None,
    }
}

/// What [`change`] needs from the app, for one account in one channel.
pub trait Twitch: Sync {
    /// The acting account's role here (lines depend on it).
    fn by(&self) -> ModRole;
    /// How this account changes `role`, or the line why it can't ([`access`]).
    fn path(&self, role: Role) -> Result<Path, &'static str>;
    /// Gives (`on`) or takes `role` from `who` (a login).
    fn assign(&self, who: &str, role: Role, on: bool) -> impl Future<Output = Result<(), Refusal>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ModRole::{Broadcaster, LeadModerator, Moderator};

    #[test]
    fn broadcaster_uses_helix_when_it_can() {
        assert_eq!(access(Broadcaster, Role::Mod, true, true), Ok(Path::Helix));
        assert_eq!(access(Broadcaster, Role::Vip, true, false), Ok(Path::Helix));
        assert_eq!(access(Broadcaster, Role::Mod, false, true), Ok(Path::Gql));
        assert_eq!(access(Broadcaster, Role::Mod, false, false), Err(NO_TOKEN));
    }

    #[test]
    fn lead_mods_never_use_helix() {
        assert_eq!(access(LeadModerator, Role::Vip, true, true), Ok(Path::Gql));
        assert_eq!(access(LeadModerator, Role::Mod, true, false), Err(LEAD_NEEDS_GQL));
        assert_eq!(access(Moderator, Role::Mod, true, true), Ok(Path::Gql), "Twitch decides");
        assert_eq!(access(Moderator, Role::Vip, true, false), Err(Need::Lead.refusal()));
    }

    #[test]
    fn v107_r42_the_rest_is_gql() {
        for r in [Role::LeadMod, Role::Editor, Role::Artist, Role::Agent] {
            assert_eq!(access(Broadcaster, r, true, true), Ok(Path::Gql));
            assert_eq!(access(Broadcaster, r, true, false), Err(NEEDS_GQL));
        }
    }

    #[test]
    fn v107_r42_helix_statuses_as_codes() {
        assert_eq!(helix_code(Change::new(Role::Mod, true), 422), Some("TARGET_IS_VIP"));
        assert_eq!(helix_code(Change::new(Role::Vip, true), 422), Some("GRANTEE_IS_MOD"));
        assert_eq!(helix_code(Change::new(Role::Vip, true), 409), Some("MAX_VIPS_REACHED"));
        assert_eq!(helix_code(Change::new(Role::Vip, false), 422), None, "not a VIP: Twitch's words");
        assert_eq!(helix_code(Change::new(Role::Mod, true), 400), None);
    }
}
