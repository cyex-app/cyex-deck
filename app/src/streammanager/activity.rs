//! 1.35 The stream manager's activity feed: follows, subs, gifts, cheers, raids and
//! redemptions on the broadcaster's own channel, and with a GQL sign-in power-ups, hype
//! trains and goals (D3). With a GQL sign-in it comes from Hermes, like Twitch's dashboard,
//! and opens with the last 100 events (`gql_feed`, D30, D1); otherwise from its own
//! EventSub session (D26). The UI filters by kind (`streammanager` settings). Works
//! whether or not the chat is open.

use cyex_core::number::grouped;
use cyex_eventsub::events::{Activity, Who, parse_activity};
use cyex_eventsub::session::{self, Out, Session};
use cyex_eventsub::subs;
use cyex_twitch::helix;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{State, lock, push_activity};
use crate::auth;

/// One line in the feed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmActivity")]
pub struct Item {
    pub id: String,
    /// `follow`, `sub`, `gift`, `cheer`, `raid`, `redemption`, `streak`, `powerup`, `hype`
    /// or `goal`; the filter keys.
    pub kind: String,
    pub text: String,
    /// Login of who did it, for the usercard; `None` when anonymous.
    pub user: Option<String>,
    /// Unix ms.
    #[ts(type = "number")]
    pub ts: i64,
    /// D4 Its alert in Twitch's alert box, for Replay and Skip; `None` without one.
    pub alert: Option<String>,
    /// D4 Where that alert is: `QUEUED`, `PLAYING`, `PLAYED`, `SKIPPED`, …
    pub status: Option<String>,
}

/// Starts the feed for `account`'s channel. With a GQL sign-in it follows Hermes topics;
/// otherwise it opens an EventSub session, whose handle comes back (dropping it ends it).
pub fn start(app: &AppHandle, account: &str) -> Option<Session> {
    if auth::gql_token(app, account).is_some() {
        super::gql_feed::start(app, account);
        return None;
    }
    Some(start_eventsub(app))
}

/// Ends the Hermes feed.
pub fn stop_gql(app: &AppHandle, account: &str) {
    super::gql_feed::stop(app, account);
}

fn start_eventsub(app: &AppHandle) -> Session {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (session, task) = session::start(tx);
    tauri::async_runtime::spawn(task);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(out) = rx.recv().await {
            handle(&app, out);
        }
    });
    session
}

fn handle(app: &AppHandle, out: Out) {
    match out {
        Out::Welcome(id) => {
            let target = {
                let st = app.state::<State>();
                let mut live = lock(&st.live);
                let Some(live) = live.as_mut() else { return };
                live.session_id = Some(id.clone());
                (live.account.clone(), live.channel_id.clone())
            };
            subscribe(app, target.0, target.1, id);
        }
        Out::Lost => {
            let st = app.state::<State>();
            if let Some(live) = lock(&st.live).as_mut() {
                live.session_id = None;
            }
        }
        Out::Notification { kind, event, .. } => match parse_activity(&kind, &event) {
            // Shared with the mod feed, which may hold the follow subscription (D30).
            Some(Activity::Follow { user, .. }) => {
                let channel = lock(&app.state::<State>().live)
                    .as_ref()
                    .map(|l| l.channel_id.clone());
                if let Some(channel) = channel {
                    super::follows::seen(app, &channel, &user);
                }
            }
            Some(a) => push(app, &a),
            None => {}
        },
        Out::Revoked { kind, .. } => eprintln!("stream manager: lost {kind}"),
    }
}

/// Subscribes every activity type on the new session (Helix). A type the token lacks the
/// scope for fails on its own; the rest still come in.
fn subscribe(app: &AppHandle, account: String, channel_id: String, session_id: String) {
    let Some((token, client_id)) = auth::helix_token(app, &account) else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let http = app.state::<auth::Auth>().http.clone();
        let auth = helix::Auth {
            client_id: &client_id,
            token: &token,
        };
        for (kind, version, cond) in subs::for_broadcaster(&channel_id, &account) {
            if let Err(e) = helix::eventsub::subscribe(&http, auth, &session_id, kind, version, &cond).await
                && !e.api_message().is_some_and(|m| m.contains("already"))
            {
                eprintln!("stream manager: couldn't follow {kind}: {e}");
            }
        }
    });
}

/// Adds a line for `a` to the feed.
pub(crate) fn push(app: &AppHandle, a: &Activity) {
    push_activity(app, item(a, cyex_chat::build::local_id(), crate::chat::now_ms()));
}

/// The feed line for `a`.
pub(crate) fn item(a: &Activity, id: String, ts: i64) -> Item {
    let (kind, text, user) = describe(a);
    Item {
        id,
        kind: kind.into(),
        text,
        user,
        ts,
        alert: None,
        status: None,
    }
}

fn name(w: &Who) -> String {
    if w.name.is_empty() {
        w.login.clone()
    } else {
        w.name.clone()
    }
}

/// `"1000"` → `"Tier 1"`.
fn tier(t: &str) -> &'static str {
    match t {
        "2000" => "Tier 2",
        "3000" => "Tier 3",
        _ => "Tier 1",
    }
}

/// Appends `: text` when there's a message, in place of the line's closing period.
pub(crate) fn with_text(line: String, text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        line
    } else {
        format!("{}: {text}", line.trim_end_matches('.'))
    }
}

/// The filter key, the line shown and who did it.
pub fn describe(a: &Activity) -> (&'static str, String, Option<String>) {
    match a {
        Activity::Follow { user, .. } => (
            "follow",
            format!("{} followed.", name(user)),
            Some(user.login.clone()),
        ),
        Activity::Subscribe { user, tier: t, gift } => {
            let line = if *gift {
                format!("{} got a gifted {} sub.", name(user), tier(t))
            } else {
                format!("{} subscribed at {}.", name(user), tier(t))
            };
            ("sub", line, Some(user.login.clone()))
        }
        Activity::SubGift { user, tier: t, total } => {
            let who = user.as_ref().map_or_else(|| "Someone".to_owned(), name);
            let what = if *total == 1 {
                format!("a {} sub", tier(t))
            } else {
                format!("{} {} subs", grouped(*total), tier(t))
            };
            (
                "gift",
                format!("{who} gifted {what}."),
                user.as_ref().map(|u| u.login.clone()),
            )
        }
        Activity::Resub {
            user,
            tier: t,
            months,
            streak,
            text,
        } => {
            let mut line = format!(
                "{} resubscribed at {} for {} months",
                name(user),
                tier(t),
                grouped(*months)
            );
            if let Some(s) = streak {
                line.push_str(&format!(", {} in a row", grouped(*s)));
            }
            line.push('.');
            ("sub", with_text(line, text), Some(user.login.clone()))
        }
        Activity::Cheer { user, bits, text } => {
            let who = user.as_ref().map_or_else(|| "Someone".to_owned(), name);
            let unit = if *bits == 1 { "bit" } else { "bits" };
            let line = format!("{who} cheered {} {unit}.", grouped(*bits));
            (
                "cheer",
                with_text(line, text),
                user.as_ref().map(|u| u.login.clone()),
            )
        }
        Activity::Raid { from, viewers } => (
            "raid",
            format!("{} raided with {} viewers.", name(from), grouped(*viewers)),
            Some(from.login.clone()),
        ),
        Activity::Redemption {
            user,
            reward,
            cost,
            input,
        } => {
            let line = format!("{} redeemed {reward} · {}", name(user), grouped(*cost));
            ("redemption", with_text(line, input), Some(user.login.clone()))
        }
        Activity::PowerUp { user, bits, what, text } => {
            let who = user.as_ref().map_or_else(|| "Someone".to_owned(), name);
            let line = format!("{who} used {what} · {} bits", grouped(*bits));
            ("powerup", with_text(line, text), user.as_ref().map(|u| u.login.clone()))
        }
        Activity::HypeTrain { state, level } => {
            let line = match state.as_str() {
                "STARTED" | "START" | "STARTING" => "Hype train started.".to_owned(),
                "LEVEL_ACHIEVED" | "LEVEL_UP" => format!("Hype train reached level {level}."),
                "COMPLETED" | "ENDED" | "END" | "EXPIRED" | "SUCCEEDED" => format!("Hype train ended at level {level}."),
                _ => format!("Hype train at level {level}."),
            };
            ("hype", line, None)
        }
        Activity::Goal {
            kind,
            state,
            current,
            target,
            text,
        } => {
            let goal = match kind.as_str() {
                "FOLLOWERS" | "FOLLOWER" => "Follower goal",
                "SUBSCRIPTIONS" | "SUBSCRIPTION_COUNT" | "SUBS" => "Sub goal",
                "NEW_SUBSCRIPTIONS" | "NEW_SUBSCRIPTION_COUNT" => "New sub goal",
                _ => "Goal",
            };
            let how = match state.as_str() {
                "STARTED" | "CREATED" | "ACTIVE" => "started",
                "ACHIEVED" | "COMPLETED" | "SUCCEEDED" => "reached",
                "ENDED" | "FINISHED" | "EXPIRED" | "CANCELED" => "ended",
                _ => "moved",
            };
            let line = if *target > 0 {
                format!("{goal} {how}, {} of {}.", grouped(*current), grouped(*target))
            } else {
                format!("{goal} {how}.")
            };
            ("goal", with_text(line, text), None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who(login: &str, name: &str) -> Who {
        Who {
            id: "1".into(),
            login: login.into(),
            name: name.into(),
        }
    }

    #[test]
    fn lines_read_well() {
        let u = who("kuni", "Kuni");
        assert_eq!(
            describe(&Activity::Follow {
                user: u.clone(),
                at: String::new()
            })
            .1,
            "Kuni followed."
        );
        assert_eq!(
            describe(&Activity::Subscribe {
                user: u.clone(),
                tier: "2000".into(),
                gift: false
            })
            .1,
            "Kuni subscribed at Tier 2."
        );
        assert_eq!(
            describe(&Activity::SubGift {
                user: None,
                tier: "1000".into(),
                total: 1500
            })
            .1,
            "Someone gifted 1.500 Tier 1 subs."
        );
        assert_eq!(
            describe(&Activity::SubGift {
                user: Some(u.clone()),
                tier: "1000".into(),
                total: 1
            })
            .1,
            "Kuni gifted a Tier 1 sub."
        );
        assert_eq!(
            describe(&Activity::Resub {
                user: u.clone(),
                tier: "1000".into(),
                months: 12,
                streak: Some(3),
                text: "hi".into()
            })
            .1,
            "Kuni resubscribed at Tier 1 for 12 months, 3 in a row: hi"
        );
        let (kind, line, user) = describe(&Activity::Cheer {
            user: None,
            bits: 1,
            text: " ".into(),
        });
        assert_eq!(
            (kind, line.as_str(), user),
            ("cheer", "Someone cheered 1 bit.", None)
        );
        assert_eq!(
            describe(&Activity::Raid {
                from: who("r", ""),
                viewers: 1234
            })
            .1,
            "r raided with 1.234 viewers."
        );
        assert_eq!(
            describe(&Activity::Redemption {
                user: u.clone(),
                reward: "Hydrate".into(),
                cost: 3220,
                input: String::new()
            })
            .1,
            "Kuni redeemed Hydrate · 3.220"
        );
        // D3
        assert_eq!(
            describe(&Activity::PowerUp {
                user: Some(u),
                bits: 1000,
                what: "a message effect".into(),
                text: "gg".into()
            }),
            ("powerup", "Kuni used a message effect · 1.000 bits: gg".into(), Some("kuni".into()))
        );
        let hype = |state: &str| {
            describe(&Activity::HypeTrain {
                state: state.into(),
                level: 3,
            })
            .1
        };
        assert_eq!(hype("LEVEL_ACHIEVED"), "Hype train reached level 3.");
        assert_eq!(hype("COMPLETED"), "Hype train ended at level 3.");
        assert_eq!(
            describe(&Activity::Goal {
                kind: "FOLLOWERS".into(),
                state: "STARTED".into(),
                current: 12052,
                target: 13000,
                text: "road to 13k".into()
            })
            .1,
            "Follower goal started, 12.052 of 13.000: road to 13k"
        );
    }
}
