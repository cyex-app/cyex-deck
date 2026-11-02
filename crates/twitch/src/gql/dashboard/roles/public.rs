//! V108-C27 Mods and VIPs of any channel, read the way twitch.tv's own chat lists them:
//! `user(login){ mods vips }`, each edge with its `grantedAt`. Checked signed out on
//! 2026/11/02: VIPs read and page; mods, editors and `usersWithRole("lead_mod")` answer
//! `Unauthenticated`, and `usersByCommunityRole(ARTIST)` fails the integrity check. So mods
//! are asked for only with a sign-in, and the rest stays with the broadcaster's own read
//! (`list`). A list Twitch doesn't share comes back as `None`.

use serde_json::{Value, json};

use super::list::{NODE, RoleHolder, holder};
use crate::gql::connection::{GAP, Page, page as connection_page, paginate};
use crate::gql::{Auth, run_partial_at};
use crate::{Http, Result};

/// What a channel shares with anyone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicLists {
    /// `None`: not asked (signed out) or Twitch didn't share it.
    pub mods: Option<Vec<RoleHolder>>,
    pub vips: Option<Vec<RoleHolder>>,
}

/// At most this many people per list (100 a page).
pub const MAX_PEOPLE: usize = 2000;
const PER_PAGE: usize = 100;

fn query() -> String {
    format!(
        "query ChannelRoleLists($login: String!, $includeMods: Boolean!, $includeVIPs: Boolean!, $modsCursor: Cursor, $vipsCursor: Cursor) {{ \
         user(login: $login) {{ id \
           mods(first: {PER_PAGE}, after: $modsCursor) @include(if: $includeMods) {{ {NODE} }} \
           vips(first: {PER_PAGE}, after: $vipsCursor) @include(if: $includeVIPs) {{ {NODE} }} }} }}"
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Mods,
    Vips,
}

impl Part {
    fn key(self) -> &'static str {
        match self {
            Part::Mods => "mods",
            Part::Vips => "vips",
        }
    }
}

fn vars(login: &str, want: &[(Part, Option<String>)]) -> Value {
    let get = |p: Part| want.iter().find(|(w, _)| *w == p);
    json!({
        "login": login,
        "includeMods": get(Part::Mods).is_some(),
        "includeVIPs": get(Part::Vips).is_some(),
        "modsCursor": get(Part::Mods).and_then(|(_, c)| c.clone()),
        "vipsCursor": get(Part::Vips).and_then(|(_, c)| c.clone()),
    })
}

/// One page of one list; `None` when Twitch left it out (null).
fn page(data: &Value, part: Part) -> Option<Page<RoleHolder>> {
    let conn = data.pointer(&format!("/user/{}", part.key())).filter(|v| v.is_object())?;
    Some(connection_page(conn, holder))
}

/// The channel's mods (only with `auth` signed in) and VIPs, every page up to
/// [`MAX_PEOPLE`]. The flag says a list was cut short. `base` aims at a mock in tests.
pub async fn public_lists(http: &Http, auth: Auth<'_>, base: Option<&str>, login: &str) -> Result<(PublicLists, bool)> {
    let q = query();
    let keep = |v: &Value| v.get("user").is_some_and(|u| !u.is_null());
    let parts: Vec<Part> = if auth.token.is_some() { vec![Part::Mods, Part::Vips] } else { vec![Part::Vips] };
    let first: Vec<(Part, Option<String>)> = parts.iter().map(|p| (*p, None)).collect();
    let data = run_partial_at(http, auth, base, &q, vars(login, &first), keep).await?;
    let mut out = PublicLists::default();
    let mut truncated = false;
    for p in parts {
        let Some(Page { mut items, next }) = page(&data, p) else { continue };
        if next.is_some() {
            let q = q.as_str();
            let more = paginate(next, MAX_PEOPLE / PER_PAGE - 1, GAP, |cursor| async move {
                let data = run_partial_at(http, auth, base, q, vars(login, &[(p, cursor)]), keep).await?;
                Ok(page(&data, p).unwrap_or_default())
            })
            .await;
            match more {
                Ok(m) => {
                    items.extend(m.items);
                    truncated |= m.truncated;
                }
                Err(_) => truncated = true,
            }
        }
        *match p {
            Part::Mods => &mut out.mods,
            Part::Vips => &mut out.vips,
        } = Some(items);
    }
    Ok((out, truncated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gql::dashboard::mock::{AUTH, serve};

    fn edge(login: &str, cursor: &str) -> Value {
        json!({ "cursor": cursor, "grantedAt": "2019-05-08T20:19:47.639308937Z", "node": { "id": format!("id-{login}"), "login": login, "displayName": login.to_uppercase(), "profileImageURL": "https://x/a.png" } })
    }

    fn conn(edges: Vec<Value>, more: bool) -> Value {
        json!({ "edges": edges, "pageInfo": { "hasNextPage": more } })
    }

    #[test]
    fn v108_c27_reads_a_signed_out_answer() {
        // As Twitch answered signed out on 2026/11/02: mods null with an error, VIPs there.
        let d = json!({ "user": { "id": "1", "mods": null, "vips": conn(vec![edge("supibot", ""), edge("spropl", "")], false) } });
        assert!(page(&d, Part::Mods).is_none());
        let v = page(&d, Part::Vips).unwrap();
        assert_eq!(v.items.iter().map(|h| h.login.as_str()).collect::<Vec<_>>(), ["supibot", "spropl"]);
        assert_eq!(v.items[0].display, "SUPIBOT");
        assert_eq!(v.items[0].granted_at.as_deref(), Some("2019-05-08T20:19:47.639308937Z"));
        assert_eq!(v.next, None);
    }

    #[test]
    fn v108_c27_asks_only_for_what_it_wants() {
        let v = vars("xqc", &[(Part::Vips, Some("c".into()))]);
        assert_eq!(v["login"], "xqc");
        assert_eq!((v["includeMods"].clone(), v["includeVIPs"].clone()), (json!(false), json!(true)));
        assert_eq!((v["modsCursor"].clone(), v["vipsCursor"].clone()), (Value::Null, json!("c")));
        assert!(query().contains("mods(first: 100, after: $modsCursor) @include(if: $includeMods)"));
    }

    #[tokio::test]
    async fn v108_c27_pages_each_list_alone_to_the_end() {
        let m = serve([
            json!({ "data": { "user": { "id": "1", "mods": conn(vec![edge("m1", "c1")], true), "vips": conn(vec![edge("v1", "")], false) } } }).to_string(),
            json!({ "data": { "user": { "id": "1", "mods": conn(vec![edge("m2", "c2")], false) } } }).to_string(),
        ]);
        let (l, truncated) = public_lists(&m.http, AUTH, Some(&m.base), "chan").await.unwrap();
        let logins = |v: &Option<Vec<RoleHolder>>| v.as_ref().unwrap().iter().map(|h| h.login.clone()).collect::<Vec<_>>();
        assert_eq!(logins(&l.mods), ["m1", "m2"]);
        assert_eq!(logins(&l.vips), ["v1"]);
        assert!(!truncated);
        let sent = m.sent();
        assert_eq!(sent[0]["variables"]["includeMods"], true);
        assert_eq!(sent[1]["variables"]["includeVIPs"], false);
        assert_eq!(sent[1]["variables"]["modsCursor"], "c1");
    }

    #[tokio::test]
    async fn v108_c27_signed_out_asks_for_vips_only_and_stops_at_the_cap() {
        let pages = MAX_PEOPLE / PER_PAGE;
        let replies: Vec<String> = (0..pages)
            .map(|i| json!({ "data": { "user": { "id": "1", "vips": conn(vec![edge(&format!("v{i}"), &format!("c{i}"))], true) } } }).to_string())
            .collect();
        let m = serve(replies);
        let signed_out = Auth { client_id: AUTH.client_id, token: None };
        let (l, truncated) = public_lists(&m.http, signed_out, Some(&m.base), "chan").await.unwrap();
        assert_eq!(l.mods, None);
        assert_eq!(l.vips.unwrap().len(), pages);
        assert!(truncated, "more pages than the cap");
        let sent = m.sent();
        assert_eq!(sent.len(), pages);
        assert_eq!(sent[0]["variables"]["includeMods"], false);
    }
}
