//! One place for Hermes (D14): anonymous connections for channel topics, shared by chat
//! (pins, redemptions, polls, predictions, celebrations) and the miner, and one signed-in
//! connection per account for its own topics (points, bets, whispers, the bell). Topics are counted, so two
//! features can follow the same one. Events go to whoever handles their kind.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Mutex;

use cyex_twitch::hermes::{Event, Hermes, Kind, Topic};
use cyex_twitch::ws::Pool;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

pub struct Hub {
    anon: Pool<Hermes>,
    /// By account id.
    users: Mutex<HashMap<String, Pool<Hermes>>>,
    events: mpsc::UnboundedSender<Event>,
}

pub fn init(app: &AppHandle) {
    let (tx, rx) = mpsc::unbounded_channel();
    app.manage(Hub {
        anon: Pool::new(Hermes::anonymous, tx.clone(), spawn),
        users: Mutex::default(),
        events: tx,
    });
    let app = app.clone();
    tauri::async_runtime::spawn(async move { pump(app, rx).await });
}

fn spawn(task: Pin<Box<dyn Future<Output = ()> + Send>>) {
    tauri::async_runtime::spawn(task);
}

/// Follows a channel topic.
pub fn add(app: &AppHandle, kind: Kind, channel_id: &str) {
    if let Some(hub) = app.try_state::<Hub>() {
        hub.anon.add(Topic::new(kind, channel_id));
    }
}

pub fn remove(app: &AppHandle, kind: Kind, channel_id: &str) {
    if let Some(hub) = app.try_state::<Hub>() {
        hub.anon.remove(&Topic::new(kind, channel_id));
    }
}

/// Follows one of an account's own topics, signed in with its GQL token.
pub fn add_user(app: &AppHandle, account: &str, kind: Kind) {
    let Some(hub) = app.try_state::<Hub>() else { return };
    let mut users = hub.users.lock().unwrap_or_else(|p| p.into_inner());
    let pool = users.entry(account.to_owned()).or_insert_with(|| {
        let (app, id) = (app.clone(), account.to_owned());
        let make = move || {
            let (app, id) = (app.clone(), id.clone());
            Hermes::signed_in(Box::new(move || {
                crate::auth::gql_token(&app, &id).map(|(token, _)| token)
            }))
        };
        Pool::new(make, hub.events.clone(), spawn)
    });
    pool.add(Topic::new(kind, account));
}

pub fn remove_user(app: &AppHandle, account: &str, kind: Kind) {
    let Some(hub) = app.try_state::<Hub>() else { return };
    let users = hub.users.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(pool) = users.get(account) {
        pool.remove(&Topic::new(kind, account));
    }
}

async fn pump(app: AppHandle, mut rx: mpsc::UnboundedReceiver<Event>) {
    while let Some(e) = rx.recv().await {
        match e.topic.kind {
            Kind::Polls | Kind::Celebrations => crate::chat::votes::on_event(&app, &e),
            Kind::Predictions => {
                crate::chat::votes::on_event(&app, &e);
                crate::miner::on_channel_event(&app, &e);
            }
            Kind::Pins => crate::chat::live::on_event(&app, &e),
            Kind::Points => {
                crate::chat::live::on_event(&app, &e);
                crate::miner::on_channel_event(&app, &e);
            }
            Kind::Playback | Kind::Raid | Kind::Moments => crate::miner::on_channel_event(&app, &e),
            Kind::UserPoints | Kind::UserPredictions => crate::miner::on_user_event(&app, &e),
            Kind::Whispers => crate::whispers::on_hermes(&app, &e),
            Kind::Onsite => crate::bell::on_event(&app, &e),
            Kind::ActivityAlerts | Kind::ActivityFeed => crate::streammanager::on_hermes(&app, &e),
        }
    }
}
