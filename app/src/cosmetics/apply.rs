//! V1051-04 Putting on a pair (a combo, or Undo): only the parts that differ go to 7TV,
//! paint and badge together, and the result says what happened to each.

use std::future::Future;

use cyex_emotes::seventv::account::model::Inventory;
use cyex_settings::schema::ComboItem;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::lookup::{Have, have, worn};
use super::{Kind, Picked, SelectError, Why, has_consent, read, select, state, words};
use crate::auth::Auth;

pub struct Want {
    pub paint: Option<ComboItem>,
    pub badge: Option<ComboItem>,
}

#[derive(Debug, Serialize, TS, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum Part {
    Changed,
    Same,
    NotOwned,
    NeedsSub,
    Failed { error: SelectError },
    Overtaken,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Applied {
    pub paint: Part,
    pub badge: Part,
    pub now_paint: Option<String>,
    pub now_badge: Option<String>,
    pub line: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Same,
    Skip(Part),
    Send(Option<String>),
}

/// Pure: what to do with one part.
pub fn decide(inv: &Inventory, kind: Kind, want: Option<&ComboItem>) -> Step {
    let on = match kind {
        Kind::Paint => inv.paint.as_deref(),
        Kind::Badge => inv.badge.as_deref(),
    };
    let id = want.map(|w| w.id.as_str());
    if id == on {
        return Step::Same;
    }
    match id.map(|id| have(inv, kind, id)) {
        Some(Have::NotOwned) => Step::Skip(Part::NotOwned),
        Some(Have::NeedsSub) => Step::Skip(Part::NeedsSub),
        _ => Step::Send(id.map(str::to_owned)),
    }
}

fn outcome(sent: Result<Picked, SelectError>) -> Part {
    match sent {
        Ok(Picked::Done { .. }) => Part::Changed,
        Ok(Picked::Superseded) => Part::Overtaken,
        Err(SelectError::NotAllowed) => Part::NotOwned,
        Err(error) => Part::Failed { error },
    }
}

/// Sends the parts that need it, in parallel, through `send` (the real one is `select`).
pub async fn run<F, Fut>(inv: &Inventory, want: &Want, send: F) -> [Part; 2]
where
    F: Fn(Kind, Option<String>) -> Fut,
    Fut: Future<Output = Result<Picked, SelectError>>,
{
    let one = |kind: Kind, item: &Option<ComboItem>| {
        let step = decide(inv, kind, item.as_ref());
        let send = &send;
        async move {
            match step {
                Step::Same => Part::Same,
                Step::Skip(p) => p,
                Step::Send(id) => outcome(send(kind, id).await),
            }
        }
    };
    let (paint, badge) = tokio::join!(one(Kind::Paint, &want.paint), one(Kind::Badge, &want.badge));
    [paint, badge]
}

/// Both failed the same way: one plain error for the caller.
pub fn both_failed(parts: &[Part; 2]) -> Option<SelectError> {
    match parts {
        [Part::Failed { error: a }, Part::Failed { error: b }] if a == b => Some(a.clone()),
        _ => None,
    }
}

pub async fn apply(
    app: &AppHandle,
    account: &str,
    want: Want,
    name: Option<&str>,
) -> Result<Applied, SelectError> {
    if !has_consent(app, account) {
        return Err(SelectError::NoConsent);
    }
    if app.state::<Auth>().seventv.get(account).is_none() {
        return Err(SelectError::SignedOut);
    }
    let inv = read(app, account).await.map_err(|w| match w {
        Why::NoConsent => SelectError::NoConsent,
        Why::NoAccount => SelectError::SignedOut,
        Why::Down => SelectError::Down,
    })?;
    let parts = run(&inv, &want, |kind, id| select(app, account, kind, id)).await;
    if let Some(e) = both_failed(&parts) {
        return Err(e);
    }
    let [paint, badge] = parts;
    // `select` kept the cached inventory's active ids current.
    let now = state(app).cached(account).map_or(inv, |(_, i)| i);
    let (np, nb) = worn(&now);
    let line = words::applied(name, &paint, &badge);
    Ok(Applied {
        paint,
        badge,
        now_paint: np.map(|i| i.name),
        now_badge: nb.map(|i| i.name),
        line,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::super::lookup::tests::inv;
    use super::*;

    fn item(id: &str) -> Option<ComboItem> {
        Some(ComboItem {
            id: id.into(),
            name: id.into(),
        })
    }

    #[test]
    fn decide_gives_all_four_steps() {
        let i = inv();
        assert_eq!(decide(&i, Kind::Paint, item("P1").as_ref()), Step::Same);
        assert_eq!(
            decide(&i, Kind::Paint, item("P2").as_ref()),
            Step::Skip(Part::NeedsSub)
        );
        assert_eq!(
            decide(&i, Kind::Badge, item("ZZ").as_ref()),
            Step::Skip(Part::NotOwned)
        );
        assert_eq!(
            decide(&i, Kind::Badge, item("B2").as_ref()),
            Step::Send(Some("B2".into()))
        );
        assert_eq!(decide(&i, Kind::Badge, None), Step::Send(None));
    }

    #[test]
    fn decide_none_on_none_is_same() {
        let mut i = inv();
        i.paint = None;
        assert_eq!(decide(&i, Kind::Paint, None), Step::Same);
    }

    async fn go(
        want: Want,
        reply: impl Fn(Kind) -> Result<Picked, SelectError>,
    ) -> ([Part; 2], Vec<(Kind, Option<String>)>) {
        let sent = Mutex::new(Vec::new());
        let parts = run(&inv(), &want, |k, id| {
            sent.lock().unwrap().push((k, id.clone()));
            let r = reply(k).map(|p| match p {
                Picked::Done { .. } => Picked::Done { id },
                o => o,
            });
            async move { r }
        })
        .await;
        (parts, sent.into_inner().unwrap())
    }

    fn done(_: Kind) -> Result<Picked, SelectError> {
        Ok(Picked::Done { id: None })
    }

    #[tokio::test]
    async fn same_pair_sends_nothing() {
        let (parts, sent) = go(
            Want {
                paint: item("P1"),
                badge: item("B1"),
            },
            done,
        )
        .await;
        assert_eq!(parts, [Part::Same, Part::Same]);
        assert!(sent.is_empty());
    }

    #[tokio::test]
    async fn unowned_badge_sends_only_the_paint() {
        let (parts, sent) = go(
            Want {
                paint: item("P4"),
                badge: item("ZZ"),
            },
            done,
        )
        .await;
        assert_eq!(parts, [Part::Changed, Part::NotOwned]);
        assert_eq!(sent, [(Kind::Paint, Some("P4".to_owned()))]);
    }

    #[tokio::test]
    async fn not_allowed_means_not_owned() {
        let (parts, _) = go(
            Want {
                paint: item("P4"),
                badge: item("B1"),
            },
            |_| Err(SelectError::NotAllowed),
        )
        .await;
        assert_eq!(parts, [Part::NotOwned, Part::Same]);
    }

    #[tokio::test]
    async fn down_on_the_badge_only() {
        let (parts, _) = go(
            Want {
                paint: item("P4"),
                badge: item("B2"),
            },
            |k| match k {
                Kind::Paint => done(k),
                Kind::Badge => Err(SelectError::Down),
            },
        )
        .await;
        assert_eq!(
            parts,
            [
                Part::Changed,
                Part::Failed {
                    error: SelectError::Down
                }
            ]
        );
        assert_eq!(both_failed(&parts), None);
    }

    #[tokio::test]
    async fn superseded_is_overtaken() {
        let (parts, _) = go(
            Want {
                paint: item("P4"),
                badge: None,
            },
            |_| Ok(Picked::Superseded),
        )
        .await;
        assert_eq!(parts[0], Part::Overtaken);
    }

    #[test]
    fn both_failed_alike_is_one_error() {
        let f = || Part::Failed {
            error: SelectError::SignInAgain,
        };
        assert_eq!(both_failed(&[f(), f()]), Some(SelectError::SignInAgain));
        assert_eq!(both_failed(&[f(), Part::Same]), None);
        let d = Part::Failed {
            error: SelectError::Down,
        };
        assert_eq!(both_failed(&[f(), d]), None);
    }
}
