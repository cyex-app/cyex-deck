//! 1.35 What the stream manager does to the broadcaster's own channel, all over Helix
//! (D26): title and category, markers, chat modes, ads, raids, polls and predictions, and
//! Stream Together. Each is a thin wrapper that words Twitch's errors. None run in testing.

use cyex_twitch::gql;
use cyex_twitch::helix::{self, ads, channel, gueststar, modes, roles, votes};
use serde::Deserialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{obs, stats, with_helix};

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
    with_helix(app, |http, c| async move {
        channel::info(&http, c.auth(), &c.channel_id).await
    })
    .await
}

/// Changes title, category, language or tags, then reads the stats again.
pub async fn set_info(app: &AppHandle, edit: channel::Edit) -> Result<(), String> {
    if edit.title.as_deref().is_some_and(|t| t.trim().is_empty()) {
        return Err("The title can't be empty.".into());
    }
    with_helix(app, |http, c| async move {
        channel::set_info(&http, c.auth(), &c.channel_id, &edit).await
    })
    .await?;
    stats::refresh(app);
    Ok(())
}

pub async fn categories(app: &AppHandle, query: String) -> Result<Vec<channel::Category>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    with_helix(app, |http, c| async move {
        channel::search_categories(&http, c.auth(), query.trim()).await
    })
    .await
}

/// Places a stream marker (live only) and logs it (X4).
pub async fn marker(app: &AppHandle, description: String) -> Result<(), String> {
    let text = description.clone();
    with_helix(app, |http, c| async move {
        roles::marker(&http, c.auth(), &c.channel_id, &text).await
    })
    .await?;
    obs::log_marker(app, &description);
    Ok(())
}

pub async fn chat_modes(app: &AppHandle) -> Result<modes::ChatModes, String> {
    with_helix(app, |http, c| async move {
        modes::modes(&http, c.auth(), c.room()).await
    })
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
    with_helix(app, |http, c| async move {
        modes::set_mode(&http, c.auth(), c.room(), change).await
    })
    .await
}

/// Runs an ad; returns the length Twitch picked.
pub async fn commercial(app: &AppHandle, secs: u32) -> Result<u32, String> {
    with_helix(app, |http, c| async move {
        roles::commercial(&http, c.auth(), &c.channel_id, secs).await
    })
    .await
}

pub async fn ad_schedule(app: &AppHandle) -> Result<ads::AdSchedule, String> {
    with_helix(app, |http, c| async move {
        ads::ad_schedule(&http, c.auth(), &c.channel_id).await
    })
    .await
}

pub async fn snooze(app: &AppHandle) -> Result<ads::Snoozed, String> {
    with_helix(app, |http, c| async move {
        ads::snooze(&http, c.auth(), &c.channel_id).await
    })
    .await
}

pub async fn raid(app: &AppHandle, login: String) -> Result<(), String> {
    let to = user_id(app, &login).await?;
    with_helix(app, |http, c| async move {
        roles::raid(&http, c.auth(), &c.channel_id, &to).await
    })
    .await
}

pub async fn unraid(app: &AppHandle) -> Result<(), String> {
    with_helix(app, |http, c| async move {
        roles::unraid(&http, c.auth(), &c.channel_id).await
    })
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
    with_helix(app, |http, c| async move {
        votes::create_poll(
            &http,
            c.auth(),
            &c.channel_id,
            p.title.trim(),
            &choices,
            p.duration,
            p.points_per_vote,
        )
        .await
    })
    .await
    .map(|_| ())
}

pub async fn end_poll(app: &AppHandle, id: String, archive: bool) -> Result<(), String> {
    with_helix(app, |http, c| async move {
        votes::end_poll(&http, c.auth(), &c.channel_id, &id, archive).await
    })
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
    with_helix(app, |http, c| async move {
        votes::create_prediction(
            &http,
            c.auth(),
            &c.channel_id,
            p.title.trim(),
            &outcomes,
            p.window,
        )
        .await
    })
    .await
    .map(|_| ())
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
    let (status, winner) = match end {
        PredictionEnd::Lock => ("LOCKED", None),
        PredictionEnd::Resolve { winner } => ("RESOLVED", Some(winner)),
        PredictionEnd::Cancel => ("CANCELED", None),
    };
    with_helix(app, |http, c| async move {
        votes::end_prediction(&http, c.auth(), &c.channel_id, &id, status, winner.as_deref()).await
    })
    .await
}

/// Stream Together: the session and its pending invites.
#[derive(Clone, Debug, Default, serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmTogether")]
pub struct Together {
    pub session: Option<gueststar::Session>,
    pub invites: Vec<InviteRow>,
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
    with_helix(app, |http, c| async move {
        let session = gueststar::session(&http, c.auth(), &c.channel_id, &c.channel_id).await?;
        let invites = match &session {
            Some(s) => gueststar::invites(&http, c.auth(), c.in_session(&s.id)).await?,
            None => Vec::new(),
        };
        // Names are a nicety: without them the id shows.
        let ids: Vec<&str> = invites.iter().map(|i| i.user_id.as_str()).collect();
        let users = if ids.is_empty() {
            Vec::new()
        } else {
            helix::users_by_id(&http, c.auth(), &ids)
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
        Ok(Together { session, invites })
    })
    .await
}

pub async fn together_start(app: &AppHandle) -> Result<(), String> {
    with_helix(app, |http, c| async move {
        gueststar::create_session(&http, c.auth(), &c.channel_id).await
    })
    .await?;
    Ok(())
}

pub async fn together_end(app: &AppHandle, session: String) -> Result<(), String> {
    with_helix(app, |http, c| async move {
        gueststar::end_session(&http, c.auth(), &c.channel_id, &session).await
    })
    .await
}

/// Invites a channel by name to the session.
pub async fn together_invite(app: &AppHandle, session: String, login: String) -> Result<(), String> {
    let guest = user_id(app, &login).await?;
    with_helix(app, |http, c| async move {
        gueststar::send_invite(&http, c.auth(), c.in_session(&session), &guest).await
    })
    .await
}

pub async fn together_uninvite(app: &AppHandle, session: String, guest: String) -> Result<(), String> {
    with_helix(app, |http, c| async move {
        gueststar::delete_invite(&http, c.auth(), c.in_session(&session), &guest).await
    })
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
    with_helix(app, |http, c| async move {
        if clear {
            gueststar::delete_slot(&http, c.auth(), c.in_session(&session), &guest, &slot, true).await
        } else {
            gueststar::assign_slot(&http, c.auth(), c.in_session(&session), &guest, &slot).await
        }
    })
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
