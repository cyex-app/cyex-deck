//! V1041-02 Channel names from a layout to user ids (1.15), asked in batches.

use std::collections::{HashMap, HashSet};

use super::Layout;
use crate::schema::TabChannel;

/// Names per lookup: Helix answers up to 100 at once.
pub const BATCH: usize = 100;

/// Whether `name` can be a Twitch login: 1 to 25 of `a-z`, `0-9` and `_`.
pub fn is_login(name: &str) -> bool {
    (1..=25).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The logins to look up, once each, in the layout's order. Names that can't be logins
/// are left out; they end up as not found.
pub fn names(layout: &Layout) -> Vec<String> {
    let mut seen = HashSet::new();
    layout
        .tabs
        .iter()
        .flat_map(|t| t.channels())
        .filter(|n| is_login(n) && seen.insert(n.as_str()))
        .cloned()
        .collect()
}

/// Asks `lookup` about every name, [`BATCH`] at a time, and keys the answers by login.
/// Names Twitch doesn't know (renamed, banned, deleted) are simply missing.
pub async fn resolve<F, Fut, E>(names: &[String], mut lookup: F) -> Result<HashMap<String, TabChannel>, E>
where
    F: FnMut(Vec<String>) -> Fut,
    Fut: Future<Output = Result<Vec<TabChannel>, E>>,
{
    let mut found = HashMap::new();
    for batch in names.chunks(BATCH) {
        for channel in lookup(batch.to_vec()).await? {
            if !channel.id.is_empty() {
                found.insert(channel.login.to_ascii_lowercase(), channel);
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
pub(crate) mod testing {
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use crate::schema::TabChannel;

    /// Runs a future that never waits, like the fake lookups here.
    pub fn now<F: Future>(f: F) -> F::Output {
        let mut f = pin!(f);
        match f.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(v) => v,
            Poll::Pending => panic!("the fake lookup waited"),
        }
    }

    pub fn channel(login: &str) -> TabChannel {
        TabChannel {
            id: format!("id-{login}"),
            login: login.into(),
            name: login.to_uppercase(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{channel, now};
    use super::*;
    use crate::chatterino::LayoutTab;

    #[test]
    fn logins() {
        for ok in ["a", "x_1", "abcdefghijklmnopqrstuvwxy"] {
            assert!(is_login(ok), "{ok}");
        }
        for bad in [
            "",
            "Upper",
            "has space",
            "dash-ed",
            "é",
            "abcdefghijklmnopqrstuvwxyz",
            "/mentions",
            "$$$",
        ] {
            assert!(!is_login(bad), "{bad}");
        }
    }

    #[test]
    fn names_are_asked_once_in_order() {
        let layout = Layout {
            tabs: vec![
                LayoutTab {
                    title: None,
                    columns: vec![vec!["b".into(), "a".into()], vec!["bad name".into()]],
                },
                LayoutTab {
                    title: None,
                    columns: vec![vec!["a".into(), "c".into()]],
                },
            ],
            ..Layout::default()
        };
        assert_eq!(names(&layout), ["b", "a", "c"]);
    }

    #[test]
    fn lookups_go_in_batches_and_skip_unknown_names() {
        let names: Vec<String> = (0..250).map(|i| format!("chan_{i:03}")).collect();
        let mut asked = Vec::new();
        let found = now(resolve(&names, |batch| {
            asked.push(batch.len());
            // A fake Twitch that knows only the even ones, and answers in upper case.
            let known: Vec<TabChannel> = batch
                .iter()
                .filter(|n| n.ends_with(['0', '2', '4', '6', '8']))
                .map(|n| TabChannel {
                    login: n.to_uppercase(),
                    ..channel(n)
                })
                .collect();
            async move { Ok::<_, ()>(known) }
        }))
        .unwrap();
        assert_eq!(asked, [100, 100, 50]);
        assert_eq!(found.len(), 125);
        assert_eq!(found["chan_248"].id, "id-chan_248");
        assert!(!found.contains_key("chan_249"));
    }

    #[test]
    fn a_failed_lookup_stops() {
        let names: Vec<String> = (0..150).map(|i| format!("c{i}")).collect();
        let mut calls = 0;
        let out = now(resolve(&names, |_| {
            calls += 1;
            async { Err::<Vec<TabChannel>, _>("offline") }
        }));
        assert_eq!((out, calls), (Err("offline"), 1));
    }
}
