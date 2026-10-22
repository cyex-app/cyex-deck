//! V107-R36 The Roles Manager (stream manager panel) for the broadcaster's own channel, and
//! the VIP slots the usercard shows. Reads are GQL (`cyex_twitch::gql::dashboard::roles`);
//! a change is [`super::apply`] (V107-R42), with the rows' holds and the slots read here.

use cyex_chat::model::ModRole;
use cyex_moderation::roles::{Member, VipSlots, Wanted, merge};
use cyex_twitch::gql::dashboard::{self, roles::list};
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use super::super::{Actor, http};

/// What the panel shows.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RolesView {
    pub members: Vec<Member>,
    pub vips: VipSlots,
    /// The change went through but the lists couldn't be read again: the panel keeps what it
    /// shows and reloads (members and slots are empty then).
    #[ts(optional = nullable)]
    pub note: Option<String>,
    /// V107-R42 After a change: its line.
    #[ts(optional = nullable)]
    pub line: Option<String>,
    /// V107-N4E A list was cut short or a later page failed: some holders may be missing.
    pub truncated: bool,
}

const STALE: &str = "The list didn’t refresh.";

/// What a change answers, from its line and the read after it: a change that went through
/// is `Ok` even when the read failed (no retry for something done).
fn after_change(change: Result<String, String>, reread: Result<RolesView, String>) -> Result<RolesView, String> {
    match (change, reread) {
        (Ok(line), Ok(v)) => Ok(RolesView { line: Some(line), ..v }),
        (Ok(line), Err(_)) => Ok(RolesView {
            members: Vec::new(),
            vips: VipSlots::unknown(),
            note: Some(STALE.into()),
            line: Some(line),
            truncated: false,
        }),
        (Err(e), _) => Err(e),
    }
}

const NEEDS_GQL: &str = "The Roles Manager needs a GQL sign-in. Add one in Settings → Accounts.";

/// The channel is the account's own, by its sign-in: a login from the UI that names another
/// channel is refused.
fn own_channel(signed_in: Option<&str>, asked: &str) -> Result<String, &'static str> {
    let own = signed_in.ok_or("Sign in again in Settings → Accounts.")?.to_ascii_lowercase();
    let asked = asked.trim().trim_start_matches('#');
    if !asked.is_empty() && !asked.eq_ignore_ascii_case(&own) {
        return Err("Only in your own channel.");
    }
    Ok(own)
}

/// The broadcaster acting in their own channel: their id is the channel's.
fn own(app: &AppHandle, account: &str, login: &str) -> Result<Actor, String> {
    let channel = own_channel(crate::auth::accounts(app).get(account).map(|a| a.login.as_str()), login)?;
    let gql = crate::auth::gql_token(app, account).ok_or(NEEDS_GQL)?;
    Ok(Actor {
        account: account.to_owned(),
        channel,
        channel_id: account.to_owned(),
        role: ModRole::Broadcaster,
        helix: crate::auth::helix_token(app, account),
        gql: Some(gql),
    })
}

/// Everyone with a role, and the VIP slots.
pub async fn view(app: &AppHandle, account: &str, login: &str) -> Result<RolesView, String> {
    let actor = own(app, account, login)?;
    let g = actor.gql().ok_or(NEEDS_GQL)?;
    let http = http(app);
    let ch = dashboard::Channel { http: &http, auth: g, base: None, id: &actor.channel_id };
    let (lists, slots) = tokio::join!(list::role_lists(&ch), list::slots(&ch));
    // V107-R41 `.1`: a list was cut short (V107-N4E: the panel says so).
    let (lists, truncated) = lists.map_err(|e| e.to_string())?;
    let allowed = slots.ok().and_then(|s| s.vips);
    let v = RolesView { members: merge(&lists), vips: VipSlots::new(lists.vips.len() as u32, allowed), note: None, line: None, truncated };
    super::remember(app, account, &v);
    Ok(v)
}

/// One change, then the lists read again. Holds and slots come from the panel's last read
/// (a minute at most), else a new one.
pub async fn set(app: &AppHandle, account: &str, login: &str, who: &str, wanted: Wanted) -> Result<RolesView, String> {
    let actor = own(app, account, login)?;
    let who = who.trim().trim_start_matches('@').to_ascii_lowercase();
    if who.is_empty() {
        return Err("Type a name.".into());
    }
    let now = match super::cached(app, account) {
        Some(v) => v,
        None => view(app, account, login).await?,
    };
    let holds = now.members.iter().find(|m| m.login == who).map(|m| m.roles.clone()).unwrap_or_default();
    let out = super::apply(app, &actor, &who, Some(holds), Some(now.vips), &wanted).await;
    let reread = view(app, account, login).await;
    after_change(if out.ok() { Ok(out.line) } else { Err(out.line) }, reread)
}

/// The usercard's VIP slots in the broadcaster's own channel; `None` elsewhere.
pub async fn vip_slots(app: &AppHandle, account: &str, login: &str) -> Result<Option<VipSlots>, String> {
    if crate::chat::roles::role(app, account, login) != Some(ModRole::Broadcaster) {
        return Ok(None);
    }
    if let Some(v) = super::cached(app, account) {
        return Ok(Some(v.vips));
    }
    view(app, account, login).await.map(|v| Some(v.vips))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> RolesView {
        RolesView { members: Vec::new(), vips: VipSlots::new(2, Some(10)), note: None, line: None, truncated: false }
    }

    #[test]
    fn v107_r36_a_done_change_is_ok_even_when_the_read_fails() {
        let ok = after_change(Ok("Made bob a mod.".into()), Ok(fresh())).unwrap();
        assert_eq!((ok.note, ok.line.as_deref()), (None, Some("Made bob a mod.")));
        let stale = after_change(Ok("x".into()), Err("offline".into())).unwrap();
        assert_eq!(stale.note.as_deref(), Some(STALE));
        assert_eq!(after_change(Err("no".into()), Ok(fresh())).unwrap_err(), "no");
        assert_eq!(after_change(Err("no".into()), Err("offline".into())).unwrap_err(), "no");
    }

    #[test]
    fn v107_r36_only_the_accounts_own_channel() {
        assert_eq!(own_channel(Some("CyexBot"), "cyexbot"), Ok("cyexbot".into()));
        assert_eq!(own_channel(Some("cyexbot"), "#CyexBot"), Ok("cyexbot".into()));
        assert_eq!(own_channel(Some("cyexbot"), ""), Ok("cyexbot".into()), "the sign-in names it");
        assert_eq!(own_channel(Some("cyexbot"), "someone"), Err("Only in your own channel."));
        assert!(own_channel(None, "cyexbot").is_err());
    }
}
