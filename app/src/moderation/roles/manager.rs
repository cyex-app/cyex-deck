//! V107-R36 The Roles Manager (stream manager panel) for the broadcaster's own channel, and
//! the VIP slots the usercard shows. Reads are GQL (`cyex_twitch::gql::dashboard::roles`);
//! a change is [`super::apply`] (V107-R42), with the rows' holds and the slots read here.

use cyex_chat::model::ModRole;
use cyex_moderation::roles::{Member, VipSlots, Wanted, merge};
use cyex_twitch::gql::dashboard::{self, roles::{Role, list::{self, RoleHolder, RoleLists}, public::{self, PublicLists}}};
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

/// V108-C26 One role and who holds it, for `/mods`, `/vips` and `/roles`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleGroup {
    pub role: Role,
    pub people: Vec<RoleHolder>,
}

/// V108-C26 What the roles pop-up shows.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleListing {
    /// In Twitch's order, a group for each role read (empty ones too).
    pub groups: Vec<RoleGroup>,
    /// A list was cut short: some holders may be missing.
    pub truncated: bool,
    /// V108-C27 Your own channel: every role, and the pop-up may give and take them.
    pub own: bool,
    /// V108-C27 Roles Twitch didn't share here (mods signed out).
    pub unread: Vec<Role>,
}

/// The six lists as groups, in [`Role::ALL`] order.
fn groups(l: RoleLists) -> Vec<RoleGroup> {
    let RoleLists { lead_mods, mods, vips, editors, artists, agents } = l;
    [lead_mods, mods, vips, editors, artists, agents]
        .into_iter()
        .zip(Role::ALL)
        .map(|(people, role)| RoleGroup { role, people })
        .collect()
}

/// V108-C26 Who holds which role, with the dates Twitch gives. V108-C27 Any channel: your
/// own reads every role through the dashboard; elsewhere mods (with a sign-in) and VIPs,
/// as twitch.tv's chat reads them.
pub async fn listing(app: &AppHandle, account: Option<&str>, login: &str) -> Result<RoleListing, String> {
    let channel = login.trim().trim_start_matches('#').to_ascii_lowercase();
    let account = account.filter(|a| !a.is_empty());
    let signed_in = account.and_then(|a| crate::auth::accounts(app).get(a).map(|x| x.login.to_ascii_lowercase()));
    let gql = account.and_then(|a| crate::auth::gql_token(app, a));
    let http = http(app);
    if let (Some(a), true, Some(_)) = (account, signed_in.as_deref() == Some(channel.as_str()), &gql) {
        let actor = own(app, a, &channel)?;
        let g = actor.gql().ok_or(NEEDS_GQL)?;
        let ch = dashboard::Channel { http: &http, auth: g, base: None, id: &actor.channel_id };
        let (lists, truncated) = list::role_lists(&ch).await.map_err(|e| e.to_string())?;
        return Ok(RoleListing { groups: groups(lists), truncated, own: true, unread: Vec::new() });
    }
    let auth = crate::auth::gql_auth_or_public(gql.as_ref());
    let (lists, truncated) = public::public_lists(&http, auth, None, &channel).await.map_err(|e| e.to_string())?;
    Ok(public_listing(lists, truncated))
}

/// V108-C27 A channel's public lists as the pop-up's groups; a list not shared is `unread`.
fn public_listing(l: PublicLists, truncated: bool) -> RoleListing {
    let mut groups = Vec::new();
    let mut unread = Vec::new();
    for (role, people) in [(Role::Mod, l.mods), (Role::Vip, l.vips)] {
        match people {
            Some(people) => groups.push(RoleGroup { role, people }),
            None => unread.push(role),
        }
    }
    RoleListing { groups, truncated, own: false, unread }
}

/// V108-C26 The listing as one chat line, for runs without a pop-up. V108-C27 `role`: that
/// role alone.
pub fn line(l: &RoleListing, role: Option<Role>) -> String {
    if let Some(r) = role.filter(|r| l.unread.contains(r)) {
        return format!("Twitch doesn’t share the {}s here. Sign in to see them.", r.name());
    }
    let parts: Vec<String> = l
        .groups
        .iter()
        .filter(|g| !g.people.is_empty() && role.is_none_or(|r| r == g.role))
        .map(|g| {
            let names: Vec<&str> = g.people.iter().map(|p| p.display.as_str()).collect();
            format!("{}: {}", g.role.name(), names.join(", "))
        })
        .collect();
    match role {
        _ if !parts.is_empty() => parts.join(". "),
        Some(r) => format!("No {}s.", r.name()),
        None => "No roles given.".into(),
    }
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

const NEEDS_GQL: &str = "The Roles manager needs a GQL sign-in. Add one in Settings → Accounts.";

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
    fn v108_c26_groups_follow_twitchs_order_and_keep_dates() {
        let who = |l: &str, at: Option<&str>| RoleHolder {
            id: "1".into(),
            login: l.into(),
            display: l.to_uppercase(),
            avatar: String::new(),
            granted_at: at.map(str::to_owned),
        };
        let lists = RoleLists { mods: vec![who("a", Some("2026-01-02T03:04:05Z")), who("b", None)], vips: vec![who("c", None)], ..Default::default() };
        let g = groups(lists);
        assert_eq!(g.iter().map(|x| x.role).collect::<Vec<_>>(), Role::ALL);
        assert_eq!(g[1].people[0].granted_at.as_deref(), Some("2026-01-02T03:04:05Z"));
        let all = RoleListing { groups: g, truncated: false, own: true, unread: Vec::new() };
        assert_eq!(line(&all, None), "mod: A, B. VIP: C");
        assert_eq!(line(&all, Some(Role::Vip)), "VIP: C");
        assert_eq!(line(&RoleListing { groups: groups(RoleLists::default()), truncated: false, own: true, unread: Vec::new() }, None), "No roles given.");
    }

    #[test]
    fn v108_c27_public_lists_say_what_twitch_kept_back() {
        let who = RoleHolder { id: "1".into(), login: "v".into(), display: "V".into(), avatar: String::new(), granted_at: None };
        let l = public_listing(PublicLists { mods: None, vips: Some(vec![who]) }, false);
        assert_eq!(l.groups.iter().map(|g| g.role).collect::<Vec<_>>(), [Role::Vip]);
        assert_eq!(l.unread, [Role::Mod]);
        assert!(!l.own);
        assert_eq!(line(&l, None), "VIP: V");
        assert!(line(&l, Some(Role::Mod)).starts_with("Twitch doesn’t share the mods"));
        let l = public_listing(PublicLists { mods: Some(Vec::new()), vips: Some(Vec::new()) }, true);
        assert_eq!((l.groups.len(), l.unread.len(), l.truncated), (2, 0, true));
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
