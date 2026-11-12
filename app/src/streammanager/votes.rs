//! V110-N13 Polls and predictions on a channel the account can run them on: its own (the
//! stream manager's way, GQL first, Helix otherwise, D30) or one it moderates. Twitch lets
//! moderators run both; Helix only takes the broadcaster's token, so a moderated channel is
//! GQL only. The mod view's Poll and Prediction panels and `/poll`, `/prediction` use it.
//! While a card shows it keeps the chat's vote watcher on for that channel, counted per
//! window, so a window that closes or reloads lets go of what its cards held.

use std::collections::HashMap;
use std::sync::Mutex;

use cyex_twitch::gql::dashboard as dash;
use tauri::{AppHandle, Manager};

use super::actions::{self, NewPoll, NewPrediction, PredictionEnd, check_poll, check_prediction};
use super::gql_call::{GqlCall, gql_call_as};
use super::words;

/// The account is the stream manager's and `login` its own channel.
fn own(manager: Option<(&str, &str)>, account: &str, login: &str) -> bool {
    let login = login.trim().trim_start_matches('#');
    manager.is_some_and(|(id, own)| id == account && own.eq_ignore_ascii_case(login))
}

/// `None`: the manager's own channel (its own calls). Else the account's GQL call as a mod
/// of the channel, or why it can't.
fn call(app: &AppHandle, account: &str, login: &str) -> Result<Option<GqlCall>, String> {
    let m = super::manager(app);
    if own(m.as_ref().map(|m| (m.account_id.as_str(), m.login.as_str())), account, login) {
        return Ok(None);
    }
    let actor = crate::moderation::actor(app, account, login)?;
    gql_call_as(app, account, actor.channel_id, actor.channel).map(Some)
}

pub async fn create_poll(app: &AppHandle, account: &str, login: &str, p: NewPoll) -> Result<(), String> {
    let Some(c) = call(app, account, login)? else { return actions::create_poll(app, p).await };
    let choices = check_poll(&p)?;
    dash::votes::create_poll(&c.http, c.auth(), &c.channel_id, p.title.trim(), &choices, p.duration, p.points_per_vote)
        .await
        .map_err(|e| words::gql(&e))
}

pub async fn end_poll(app: &AppHandle, account: &str, login: &str, id: String, archive: bool) -> Result<(), String> {
    let Some(c) = call(app, account, login)? else { return actions::end_poll(app, id, archive).await };
    dash::votes::end_poll(&c.http, c.auth(), &id, archive)
        .await
        .map_err(|e| words::gql(&e))
}

pub async fn create_prediction(app: &AppHandle, account: &str, login: &str, p: NewPrediction) -> Result<(), String> {
    let Some(c) = call(app, account, login)? else { return actions::create_prediction(app, p).await };
    let outcomes = check_prediction(&p)?;
    dash::votes::create_prediction(&c.http, c.auth(), &c.channel_id, p.title.trim(), &outcomes, p.window)
        .await
        .map_err(|e| words::gql(&e))
}

pub async fn end_prediction(
    app: &AppHandle,
    account: &str,
    login: &str,
    id: String,
    end: PredictionEnd,
) -> Result<(), String> {
    let Some(c) = call(app, account, login)? else { return actions::end_prediction(app, id, end).await };
    actions::end_prediction_gql(&c.http, c.auth(), &id, &end)
        .await
        .map_err(|e| words::gql(&e))
}

/// How many cards each window shows per channel.
#[derive(Default)]
pub struct Watching(Mutex<Counts>);

#[derive(Default)]
struct Counts(HashMap<(String, String), u32>);

impl Counts {
    fn add(&mut self, window: &str, login: &str) {
        *self.0.entry((window.to_owned(), login.to_owned())).or_default() += 1;
    }

    /// `true` when the window held one to let go of.
    fn remove(&mut self, window: &str, login: &str) -> bool {
        let key = (window.to_owned(), login.to_owned());
        let Some(n) = self.0.get_mut(&key) else { return false };
        *n -= 1;
        if *n == 0 {
            self.0.remove(&key);
        }
        true
    }

    /// Everything the window held, one login per card.
    fn take(&mut self, window: &str) -> Vec<String> {
        let mut out = Vec::new();
        self.0.retain(|(w, login), n| {
            if w != window {
                return true;
            }
            out.extend(std::iter::repeat_n(login.clone(), *n as usize));
            false
        });
        out
    }
}

fn counts(app: &AppHandle) -> std::sync::MutexGuard<'_, Counts> {
    app.state::<Watching>().inner().0.lock().unwrap_or_else(|p| p.into_inner())
}

/// A card in `window` shows the channel's poll or prediction.
pub fn watch(app: &AppHandle, window: &str, account: &str, login: &str) {
    let login = login.trim().trim_start_matches('#').to_ascii_lowercase();
    counts(app).add(window, &login);
    crate::chat::votes::watch(app, &login, Some(account.to_owned()));
}

/// That card is gone.
pub fn unwatch(app: &AppHandle, window: &str, login: &str) {
    let login = login.trim().trim_start_matches('#').to_ascii_lowercase();
    let held = counts(app).remove(window, &login);
    if held {
        crate::chat::votes::unwatch(app, &login);
    }
}

/// The window closed or reloaded: its cards' cleanup never ran.
pub fn forget(app: &AppHandle, window: &str) {
    let gone = counts(app).take(window);
    for login in gone {
        crate::chat::votes::unwatch(app, &login);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v110_n13_own_channel_is_the_managers_account_on_its_login() {
        let m = Some(("1", "me"));
        assert!(own(m, "1", "Me"));
        assert!(own(m, "1", "#me"));
        assert!(!own(m, "2", "me"));
        assert!(!own(m, "1", "other"));
        assert!(!own(None, "1", "me"));
    }

    #[test]
    fn v110_n13_counts_per_window() {
        let mut c = Counts::default();
        c.add("main", "a");
        c.add("main", "a");
        c.add("pop", "a");
        assert!(c.remove("main", "a"));
        assert!(!c.remove("main", "b"));
        let mut gone = c.take("main");
        gone.extend(c.take("pop"));
        assert_eq!(gone, ["a", "a"]);
        assert!(c.take("main").is_empty());
        assert!(!c.remove("main", "a"));
    }
}
