//! V107-R41 GQL connections (`edges { cursor node } pageInfo { hasNextPage }`): one page
//! decoder, the user every list shows, and one paging loop for reads that want everything.

use std::collections::HashSet;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use crate::Result;

/// Between two pages of one read.
pub const GAP: Duration = Duration::from_millis(250);

/// A user as the dashboard's lists show one: followers, role holders, channels.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UserRef {
    pub id: String,
    pub login: String,
    /// The display name, else the login.
    pub display: String,
    /// Empty when Twitch has none.
    pub avatar: String,
}

impl UserRef {
    /// A node `{ id login displayName profileImageURL }`. `None` without an id or a login
    /// (a deleted account).
    pub(crate) fn from_node(n: &Value) -> Option<Self> {
        let s = |k: &str| n.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
        let (id, login) = (s("id"), s("login"));
        if id.is_empty() || login.is_empty() {
            return None;
        }
        let display = Some(s("displayName"))
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| login.clone());
        Some(Self {
            id,
            login,
            display,
            avatar: s("profileImageURL"),
        })
    }
}

/// One page and the cursor of the next, `None` at the end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<String>,
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next: None,
        }
    }
}

/// Reads a connection; `item` turns one edge (with its `node`) into an item or skips it.
/// The next cursor is the last edge's, only while `hasNextPage`.
pub(crate) fn page<T>(conn: &Value, item: impl FnMut(&Value) -> Option<T>) -> Page<T> {
    let edges = conn
        .get("edges")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let more = conn
        .pointer("/pageInfo/hasNextPage")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let next = more
        .then(|| {
            edges
                .iter()
                .rev()
                .find_map(|e| e.get("cursor").and_then(Value::as_str).filter(|c| !c.is_empty()))
        })
        .flatten()
        .map(str::to_owned);
    Page {
        items: edges.iter().filter_map(item).collect(),
        next,
    }
}

/// Everything a paged read found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paged<T> {
    pub items: Vec<T>,
    /// Stopped before the end: the page cap, a cursor Twitch sent twice, or a later page failed.
    pub truncated: bool,
}

/// Reads page after page from `start`, `max_pages` at most, `gap` apart. A cursor seen
/// before ends it (Twitch looping). The first page's error is the error; a later one keeps
/// what came and says `truncated`.
pub(crate) async fn paginate<T, F, Fut>(
    start: Option<String>,
    max_pages: usize,
    gap: Duration,
    mut fetch: F,
) -> Result<Paged<T>>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: Future<Output = Result<Page<T>>>,
{
    let mut seen: HashSet<String> = start.iter().cloned().collect();
    let mut cursor = start;
    let mut items = Vec::new();
    for i in 0..max_pages {
        if i > 0 && !gap.is_zero() {
            tokio::time::sleep(gap).await;
        }
        let p = match fetch(cursor.clone()).await {
            Ok(p) => p,
            Err(e) if i == 0 => return Err(e),
            Err(_) => {
                return Ok(Paged {
                    items,
                    truncated: true,
                });
            }
        };
        items.extend(p.items);
        match p.next {
            None => {
                return Ok(Paged {
                    items,
                    truncated: false,
                });
            }
            Some(c) if !seen.insert(c.clone()) => {
                return Ok(Paged {
                    items,
                    truncated: true,
                });
            }
            Some(c) => cursor = Some(c),
        }
    }
    Ok(Paged {
        items,
        truncated: true,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use serde_json::json;

    use super::*;
    use crate::Error;

    #[test]
    fn v107_r41_a_page_and_its_cursor() {
        let conn = json!({ "edges": [
            { "cursor": "a", "node": { "id": "1", "login": "ann", "displayName": "Ann", "profileImageURL": "u" } },
            { "cursor": "b", "node": { "id": "2", "login": "bob", "displayName": "" } },
            { "cursor": "c", "node": { "id": "", "login": "gone" } },
            { "cursor": "", "node": null }
        ], "pageInfo": { "hasNextPage": true } });
        let p = page(&conn, |e| UserRef::from_node(&e["node"]));
        assert_eq!(p.next.as_deref(), Some("c"), "the last non-empty cursor");
        assert_eq!(p.items.len(), 2);
        assert_eq!(
            (p.items[1].display.as_str(), p.items[1].avatar.as_str()),
            ("bob", "")
        );
        let end = page(
            &json!({ "edges": [{ "cursor": "a" }], "pageInfo": { "hasNextPage": false } }),
            |_| Some(()),
        );
        assert_eq!(end.next, None);
        assert_eq!(page(&Value::Null, |_| Some(())), Page::default());
    }

    type Script = RefCell<std::vec::IntoIter<Result<Page<u32>>>>;

    fn pages(script: Vec<Result<Page<u32>>>) -> (RefCell<Vec<Option<String>>>, Script) {
        (RefCell::new(Vec::new()), RefCell::new(script.into_iter()))
    }

    fn p(items: &[u32], next: Option<&str>) -> Result<Page<u32>> {
        Ok(Page {
            items: items.to_vec(),
            next: next.map(Into::into),
        })
    }

    #[tokio::test]
    async fn v107_r41_paging_stops_on_a_repeated_cursor() {
        let (asked, script) = pages(vec![
            p(&[1], Some("x")),
            p(&[2], Some("y")),
            p(&[3], Some("x")),
            p(&[9], None),
        ]);
        let out = paginate(None, 10, Duration::ZERO, |c| {
            asked.borrow_mut().push(c);
            let next = script.borrow_mut().next().unwrap();
            async move { next }
        })
        .await
        .unwrap();
        assert_eq!(
            out,
            Paged {
                items: vec![1, 2, 3],
                truncated: true
            }
        );
        assert_eq!(*asked.borrow(), [None, Some("x".into()), Some("y".into())]);
    }

    #[tokio::test]
    async fn v107_r41_paging_caps_and_ends() {
        let (_, script) = pages(vec![p(&[1], Some("a")), p(&[2], Some("b")), p(&[3], Some("c"))]);
        let out = paginate(None, 2, Duration::ZERO, |_| {
            let next = script.borrow_mut().next().unwrap();
            async move { next }
        })
        .await
        .unwrap();
        assert_eq!(
            out,
            Paged {
                items: vec![1, 2],
                truncated: true
            }
        );
        let (asked, script) = pages(vec![p(&[5], None)]);
        let out = paginate(Some("s".into()), 5, Duration::ZERO, |c| {
            asked.borrow_mut().push(c);
            let next = script.borrow_mut().next().unwrap();
            async move { next }
        })
        .await
        .unwrap();
        assert_eq!(
            out,
            Paged {
                items: vec![5],
                truncated: false
            }
        );
        assert_eq!(*asked.borrow(), [Some("s".to_owned())]);
    }

    #[tokio::test]
    async fn v107_r41_first_error_fails_later_ones_truncate() {
        let fail = || Err(Error::Refused("X".into()));
        let (_, script) = pages(vec![fail()]);
        let r = paginate(None, 3, Duration::ZERO, |_| {
            let next = script.borrow_mut().next().unwrap();
            async move { next }
        })
        .await;
        assert!(r.is_err());
        let (_, script) = pages(vec![p(&[1], Some("a")), fail()]);
        let out = paginate(None, 3, Duration::ZERO, |_| {
            let next = script.borrow_mut().next().unwrap();
            async move { next }
        })
        .await
        .unwrap();
        assert_eq!(
            out,
            Paged {
                items: vec![1],
                truncated: true
            }
        );
    }
}
