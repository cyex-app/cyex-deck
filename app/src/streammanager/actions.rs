//! 1.35 What the stream manager does to the broadcaster's own channel: title and
//! category, markers, chat modes, ads, raids, polls and predictions, and Stream Together.
//! Each runs GQL first, like Twitch's dashboard, and Helix otherwise (D30); Twitch's
//! errors come back as short lines. D2, D4, D5, D6 (raid now and suggestions, alerts, ad
//! settings, join requests) are GQL only, as on Twitch's dashboard. None run in testing.

use cyex_twitch::gql::{self, dashboard as dash};
use cyex_twitch::helix::{self, ads, channel, gueststar, modes, roles, votes};
use serde::Deserialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{Call, either, obs, stats, words};

/// For what only Twitch's own sign-in can do.
const GQL_ONLY: &str = "This needs the sign-in with a code (Settings → Accounts).";

/// The manager's GQL sign-in, or the line to show.
fn gql_call(app: &AppHandle) -> Result<Call, String> {
    let c = Call::new(app)?;
    if c.has_gql() { Ok(c) } else { Err(GQL_ONLY.into()) }
}

/// Twitch user id for a login, from public GQL.
async fn user_id(app: &AppHandle, login: &str) -> Result<String, String> {
    let login = login.trim().trim_start_matches('@');
    if login.is_empty() {
        return Err("Type a channel name.".into());
    }
    let http = app.state::<crate::auth::Auth>().http.clone();
    gql::profile(&http, Some(login), None)
        .await
        .map_err(|e| e.to_string())?
        .map(|p| p.id)
        .ok_or_else(|| format!("No channel called {login}."))
}

pub async fn info(app: &AppHandle) -> Result<channel::ChannelInfo, String> {
    either(
        app,
        async |c: &Call| dash::info::info(&c.http, c.gql(), &c.channel_id).await,
        async |c: &Call| channel::info(&c.http, c.helix(), &c.channel_id).await,
    )
    .await
}

/// Changes title, category, language or tags, then reads the stats again.
pub async fn set_info(app: &AppHandle, edit: channel::Edit) -> Result<(), String> {
    if edit.title.as_deref().is_some_and(|t| t.trim().is_empty()) {
        return Err("The title can’t be empty.".into());
    }
    either(
        app,
        async |c: &Call| dash::info::set_info(&c.http, c.gql(), &c.channel_id, &edit).await,
        async |c: &Call| channel::set_info(&c.http, c.helix(), &c.channel_id, &edit).await,
    )
    .await?;
    stats::refresh(app);
    Ok(())
}

pub async fn categories(app: &AppHandle, query: String) -> Result<Vec<channel::Category>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let q = query.trim();
    either(
        app,
        async |c: &Call| dash::info::categories(&c.http, c.gql(), q).await,
        async |c: &Call| channel::search_categories(&c.http, c.helix(), q).await,
    )
    .await
}

/// Places a stream marker (live only) and logs it (X4).
pub async fn marker(app: &AppHandle, description: String) -> Result<(), String> {
    let text = description.trim();
    either(
        app,
        async |c: &Call| dash::info::marker(&c.http, c.gql(), &c.channel_id, text).await,
        async |c: &Call| roles::marker(&c.http, c.helix(), &c.channel_id, text).await,
    )
    .await?;
    obs::log_marker(app, &description);
    Ok(())
}

pub async fn chat_modes(app: &AppHandle) -> Result<modes::ChatModes, String> {
    either(
        app,
        async |c: &Call| dash::modes::modes(&c.http, c.gql(), &c.channel_id).await,
        async |c: &Call| modes::modes(&c.http, c.helix(), c.room()).await,
    )
    .await
}

/// One chat-mode switch from the UI.
#[derive(Clone, Copy, Debug, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, rename = "SmMode")]
pub enum Mode {
    Emote {
        on: bool,
    },
    Subs {
        on: bool,
    },
    Unique {
        on: bool,
    },
    /// Minimum follow age in minutes; `None` turns it off.
    Followers {
        minutes: Option<u32>,
    },
    /// Seconds between messages; `None` turns it off.
    Slow {
        seconds: Option<u32>,
    },
}

impl From<Mode> for modes::ModeChange {
    fn from(m: Mode) -> Self {
        match m {
            Mode::Emote { on } => Self::Emote(on),
            Mode::Subs { on } => Self::Subs(on),
            Mode::Unique { on } => Self::Unique(on),
            Mode::Followers { minutes } => Self::Followers(minutes),
            Mode::Slow { seconds } => Self::Slow(seconds),
        }
    }
}

pub async fn set_mode(app: &AppHandle, mode: Mode) -> Result<modes::ChatModes, String> {
    let change = modes::ModeChange::from(mode);
    either(
        app,
        async |c: &Call| dash::modes::set_mode(&c.http, c.gql(), &c.channel_id, change).await,
        async |c: &Call| modes::set_mode(&c.http, c.helix(), c.room(), change).await,
    )
    .await
}

/// Runs an ad; returns the length Twitch picked.
pub async fn commercial(app: &AppHandle, secs: u32) -> Result<u32, String> {
    either(
        app,
        async |c: &Call| dash::ads::start(&c.http, c.gql(), &c.channel_id, secs).await,
        async |c: &Call| roles::commercial(&c.http, c.helix(), &c.channel_id, secs).await,
    )
    .await
}

pub async fn ad_schedule(app: &AppHandle) -> Result<ads::AdSchedule, String> {
    either(
        app,
        async |c: &Call| dash::ads::schedule(&c.http, c.gql(), &c.channel_id).await,
        async |c: &Call| ads::ad_schedule(&c.http, c.helix(), &c.channel_id).await,
    )
    .await
}

pub async fn snooze(app: &AppHandle) -> Result<ads::Snoozed, String> {
    either(
        app,
        async |c: &Call| dash::ads::snooze(&c.http, c.gql(), &c.channel_id).await,
        async |c: &Call| ads::snooze(&c.http, c.helix(), &c.channel_id).await,
    )
    .await
}

pub async fn raid(app: &AppHandle, login: String) -> Result<(), String> {
    let to = user_id(app, &login).await?;
    either(
        app,
        async |c: &Call| dash::raids::start(&c.http, c.gql(), &c.channel_id, &to).await,
        async |c: &Call| roles::raid(&c.http, c.helix(), &c.channel_id, &to).await,
    )
    .await
}

/// D2 Sends the raid now instead of waiting out the countdown.
pub async fn raid_now(app: &AppHandle) -> Result<(), String> {
    let c = gql_call(app)?;
    dash::raids::go(&c.http, c.gql(), &c.channel_id).await.map_err(|e| words::gql(&e))
}

/// D2 Twitch's raid suggestions; none without a GQL sign-in.
pub async fn raid_picks(app: &AppHandle) -> Result<Vec<dash::raids::RaidPick>, String> {
    let Ok(c) = gql_call(app) else { return Ok(Vec::new()) };
    dash::raids::suggestions(&c.http, c.gql(), &c.channel_id).await.map_err(|e| words::gql(&e))
}

/// D4 Plays an alert again, or skips it, in Twitch's alert box.
pub async fn alert(app: &AppHandle, id: String, replay: bool) -> Result<(), String> {
    let c = gql_call(app)?;
    dash::feed::set_alert(&c.http, c.gql(), &id, replay).await.map_err(|e| words::gql(&e))
}

/// D5 The ad schedule's settings; `None` without a GQL sign-in.
pub async fn ad_settings(app: &AppHandle) -> Result<Option<dash::ads::AdSettings>, String> {
    let Ok(c) = gql_call(app) else { return Ok(None) };
    dash::ads::settings(&c.http, c.gql(), &c.channel_id)
        .await
        .map(Some)
        .map_err(|e| words::gql(&e))
}

pub async fn set_ad_settings(app: &AppHandle, edit: dash::ads::AdEdit) -> Result<(), String> {
    let c = gql_call(app)?;
    dash::ads::set_settings(&c.http, c.gql(), &c.channel_id, &edit)
        .await
        .map_err(|e| words::gql(&e))
}

/// D6 Lets a creator into the Stream Together, or not.
pub async fn join_answer(app: &AppHandle, user_id: String, approve: bool) -> Result<(), String> {
    let c = gql_call(app)?;
    dash::together::answer(&c.http, c.gql(), &user_id, approve)
        .await
        .map_err(|e| words::gql(&e))
}

/// D6 Guests show on stream once they're set up: for the session, or the next one.
pub async fn auto_add(app: &AppHandle, session: Option<String>, on: bool) -> Result<(), String> {
    let c = gql_call(app)?;
    dash::together::set_auto_add(&c.http, c.gql(), session.as_deref(), on)
        .await
        .map_err(|e| words::gql(&e))
}

pub async fn unraid(app: &AppHandle) -> Result<(), String> {
    either(
        app,
        async |c: &Call| dash::raids::cancel(&c.http, c.gql(), &c.channel_id).await,
        async |c: &Call| roles::unraid(&c.http, c.helix(), &c.channel_id).await,
    )
    .await
}

/// A poll to start, from the UI.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmPoll")]
pub struct NewPoll {
    pub title: String,
    pub choices: Vec<String>,
    /// Seconds.
    pub duration: u32,
    /// Channel points per extra vote; `None` = no extra votes.
    pub points_per_vote: Option<u32>,
}

/// Checks a poll before it goes out: a title and 2-5 choices.
pub fn check_poll(p: &NewPoll) -> Result<Vec<String>, String> {
    if p.title.trim().is_empty() {
        return Err("Give the poll a question.".into());
    }
    let choices: Vec<String> = p
        .choices
        .iter()
        .map(|c| c.trim().to_owned())
        .filter(|c| !c.is_empty())
        .collect();
    match choices.len() {
        0 | 1 => Err("A poll needs at least two choices.".into()),
        2..=5 => Ok(choices),
        _ => Err("A poll can have at most five choices.".into()),
    }
}

pub async fn create_poll(app: &AppHandle, p: NewPoll) -> Result<(), String> {
    let choices = check_poll(&p)?;
    let (title, secs, points) = (p.title.trim(), p.duration, p.points_per_vote);
    either(
        app,
        async |c: &Call| dash::votes::create_poll(&c.http, c.gql(), &c.channel_id, title, &choices, secs, points).await,
        async |c: &Call| {
            votes::create_poll(&c.http, c.helix(), &c.channel_id, title, &choices, secs, points)
                .await
                .map(|_| ())
        },
    )
    .await
}

pub async fn end_poll(app: &AppHandle, id: String, archive: bool) -> Result<(), String> {
    either(
        app,
        async |c: &Call| dash::votes::end_poll(&c.http, c.gql(), &id, archive).await,
        async |c: &Call| votes::end_poll(&c.http, c.helix(), &c.channel_id, &id, archive).await,
    )
    .await
}

/// A prediction to start, from the UI.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmPrediction")]
pub struct NewPrediction {
    pub title: String,
    pub outcomes: Vec<String>,
    /// Seconds betting stays open.
    pub window: u32,
}

/// Checks a prediction: a title and 2-10 outcomes.
pub fn check_prediction(p: &NewPrediction) -> Result<Vec<String>, String> {
    if p.title.trim().is_empty() {
        return Err("Give the prediction a question.".into());
    }
    let outcomes: Vec<String> = p
        .outcomes
        .iter()
        .map(|o| o.trim().to_owned())
        .filter(|o| !o.is_empty())
        .collect();
    match outcomes.len() {
        0 | 1 => Err("A prediction needs at least two outcomes.".into()),
        2..=10 => Ok(outcomes),
        _ => Err("A prediction can have at most ten outcomes.".into()),
    }
}

pub async fn create_prediction(app: &AppHandle, p: NewPrediction) -> Result<(), String> {
    let outcomes = check_prediction(&p)?;
    let (title, window) = (p.title.trim(), p.window);
    either(
        app,
        async |c: &Call| dash::votes::create_prediction(&c.http, c.gql(), &c.channel_id, title, &outcomes, window).await,
        async |c: &Call| {
            votes::create_prediction(&c.http, c.helix(), &c.channel_id, title, &outcomes, window)
                .await
                .map(|_| ())
        },
    )
    .await
}

/// How a prediction ends: lock betting, pay out a winner, or refund everyone.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, rename = "SmPredictionEnd")]
pub enum PredictionEnd {
    Lock,
    Resolve { winner: String },
    Cancel,
}

pub async fn end_prediction(app: &AppHandle, id: String, end: PredictionEnd) -> Result<(), String> {
    let (status, winner) = match &end {
        PredictionEnd::Lock => ("LOCKED", None),
        PredictionEnd::Resolve { winner } => ("RESOLVED", Some(winner.as_str())),
        PredictionEnd::Cancel => ("CANCELED", None),
    };
    either(
        app,
        async |c: &Call| {
            let (http, auth) = (&c.http, c.gql());
            match winner {
                Some(w) => dash::votes::resolve_prediction(http, auth, &id, w).await,
                None if status == "LOCKED" => dash::votes::lock_prediction(http, auth, &id).await,
                None => dash::votes::cancel_prediction(http, auth, &id).await,
            }
        },
        async |c: &Call| votes::end_prediction(&c.http, c.helix(), &c.channel_id, &id, status, winner).await,
    )
    .await
}

/// Stream Together: the session and its pending invites.
#[derive(Clone, Debug, Default, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmTogether")]
pub struct Together {
    pub session: Option<gueststar::Session>,
    pub invites: Vec<InviteRow>,
    /// D6 Creators asking to join (GQL sign-in only).
    pub requests: Vec<dash::together::JoinRequest>,
    /// D6 Guests show on stream once set up; `None` when unknown (no GQL sign-in).
    pub auto_add: Option<bool>,
}

/// A pending invite with the guest's name (Twitch only gives the id).
#[derive(Clone, Debug, Default, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmInvite")]
pub struct InviteRow {
    pub user_id: String,
    pub login: String,
    pub name: String,
    /// `INVITED`, `ACCEPTED` or `READY`.
    pub status: String,
}

pub async fn together(app: &AppHandle) -> Result<Together, String> {
    let login = super::manager(app).map(|m| m.login).unwrap_or_default();
    either(
        app,
        async |c: &Call| {
            let (found, requests, auto_add) = tokio::join!(
                dash::together::session(&c.http, c.gql(), &c.channel_id),
                dash::together::join_requests(&c.http, c.gql(), &c.channel_id),
                dash::together::auto_add(&c.http, c.gql(), &c.channel_id, &login),
            );
            // Both are extras: the session still shows without them.
            let requests = requests.unwrap_or_default();
            let auto_add = auto_add.ok().flatten();
            let Some((session, invited)) = found? else {
                return Ok(Together {
                    requests,
                    auto_add,
                    ..Together::default()
                });
            };
            let invites = invited
                .into_iter()
                .map(|i| InviteRow {
                    user_id: i.user_id,
                    login: i.login,
                    name: i.name,
                    status: i.status,
                })
                .collect();
            Ok(Together {
                session: Some(session),
                invites,
                requests,
                auto_add,
            })
        },
        async |c: &Call| {
            let session = gueststar::session(&c.http, c.helix(), &c.channel_id, &c.channel_id).await?;
            let invites = match &session {
                Some(s) => gueststar::invites(&c.http, c.helix(), c.in_session(&s.id)).await?,
                None => Vec::new(),
            };
            // Names are a nicety: without them the id shows.
            let ids: Vec<&str> = invites.iter().map(|i| i.user_id.as_str()).collect();
            let users = if ids.is_empty() {
                Vec::new()
            } else {
                helix::users_by_id(&c.http, c.helix(), &ids)
                    .await
                    .unwrap_or_default()
            };
            let invites = invites
                .into_iter()
                .map(|i| {
                    let user = users.iter().find(|u| u.id == i.user_id);
                    InviteRow {
                        login: user.map(|u| u.login.clone()).unwrap_or_default(),
                        name: user.map_or_else(|| i.user_id.clone(), |u| u.display_name.clone()),
                        user_id: i.user_id,
                        status: i.status,
                    }
                })
                .collect();
            Ok(Together {
                session,
                invites,
                ..Together::default()
            })
        },
    )
    .await
}

pub async fn together_start(app: &AppHandle) -> Result<(), String> {
    either(
        app,
        async |c: &Call| dash::together::start(&c.http, c.gql(), &c.channel_id).await,
        async |c: &Call| {
            gueststar::create_session(&c.http, c.helix(), &c.channel_id)
                .await
                .map(|_| ())
        },
    )
    .await
}

pub async fn together_end(app: &AppHandle, session: String) -> Result<(), String> {
    either(
        app,
        async |c: &Call| dash::together::end(&c.http, c.gql(), &session).await,
        async |c: &Call| gueststar::end_session(&c.http, c.helix(), &c.channel_id, &session).await,
    )
    .await
}

/// Invites a channel by name to the session.
pub async fn together_invite(app: &AppHandle, session: String, login: String) -> Result<(), String> {
    let guest = user_id(app, &login).await?;
    either(
        app,
        async |c: &Call| dash::together::invite(&c.http, c.gql(), &session, &guest).await,
        async |c: &Call| gueststar::send_invite(&c.http, c.helix(), c.in_session(&session), &guest).await,
    )
    .await
}

pub async fn together_uninvite(app: &AppHandle, session: String, guest: String) -> Result<(), String> {
    either(
        app,
        async |c: &Call| dash::together::uninvite(&c.http, c.gql(), &session, &guest).await,
        async |c: &Call| gueststar::delete_invite(&c.http, c.helix(), c.in_session(&session), &guest).await,
    )
    .await
}

/// Puts a guest in a slot, or with `clear` takes them out of it (they stay invited).
pub async fn together_slot(
    app: &AppHandle,
    session: String,
    guest: String,
    slot: String,
    clear: bool,
) -> Result<(), String> {
    either(
        app,
        async |c: &Call| {
            if clear {
                dash::together::unassign(&c.http, c.gql(), &session, &guest, &slot).await
            } else {
                dash::together::assign(&c.http, c.gql(), &session, &guest, &slot).await
            }
        },
        async |c: &Call| {
            if clear {
                gueststar::delete_slot(&c.http, c.helix(), c.in_session(&session), &guest, &slot, true).await
            } else {
                gueststar::assign_slot(&c.http, c.helix(), c.in_session(&session), &guest, &slot).await
            }
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poll(title: &str, choices: &[&str]) -> NewPoll {
        NewPoll {
            title: title.into(),
            choices: choices.iter().map(|c| (*c).into()).collect(),
            duration: 60,
            points_per_vote: None,
        }
    }

    #[test]
    fn polls_need_a_question_and_two_to_five_choices() {
        assert!(check_poll(&poll(" ", &["a", "b"])).is_err());
        assert!(check_poll(&poll("q", &["a", " "])).is_err());
        assert_eq!(check_poll(&poll("q", &[" a ", "", "b"])).unwrap(), ["a", "b"]);
        assert!(check_poll(&poll("q", &["1", "2", "3", "4", "5", "6"])).is_err());
    }

    #[test]
    fn predictions_need_two_to_ten_outcomes() {
        let p = |o: &[&str]| NewPrediction {
            title: "q".into(),
            outcomes: o.iter().map(|x| (*x).into()).collect(),
            window: 120,
        };
        assert!(check_prediction(&p(&["yes"])).is_err());
        assert_eq!(check_prediction(&p(&["yes", "no"])).unwrap().len(), 2);
        assert!(check_prediction(&p(&["x"; 11])).is_err());
    }

    #[test]
    fn modes_map_to_helix_changes() {
        assert_eq!(
            modes::ModeChange::from(Mode::Followers { minutes: Some(10) }),
            modes::ModeChange::Followers(Some(10))
        );
        assert_eq!(
            modes::ModeChange::from(Mode::Subs { on: false }),
            modes::ModeChange::Subs(false)
        );
    }
}
