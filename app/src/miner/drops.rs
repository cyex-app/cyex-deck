//! Drops (1.30, 1.32): the account's inventory every minute while a watched channel has
//! drops (every 15 minutes otherwise); earned drops are claimed 5-10 s apart. Twitch
//! wants the game account linked before it hands a drop out.

use std::time::Duration;

use cyex_miner::stats::{Kind, Record};
use cyex_twitch::gql::drops;
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use super::{gql_auth, http, lock, log, now_ms};

const BUSY: Duration = Duration::from_secs(60);
const IDLE: Duration = Duration::from_secs(15 * 60);

/// A drop still being earned, for the miner view.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DropProgress {
    pub name: String,
    pub game: Option<String>,
    pub image: Option<String>,
    pub minutes: u32,
    pub required: u32,
}

pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // Let the first live check run first.
        tokio::time::sleep(Duration::from_secs(20)).await;
        loop {
            let wait = tick(&app).await;
            tokio::time::sleep(wait).await;
        }
    });
}

async fn tick(app: &AppHandle) -> Duration {
    if !super::settings(app).claim_drops {
        return IDLE;
    }
    let Some((account, token, client_id)) = super::signed_in(app) else {
        return IDLE;
    };
    let (busy, credit_to) = {
        let st = lock(app);
        let watched: Vec<&super::Chan> = st
            .channels
            .iter()
            .filter(|c| st.slots.contains(&c.id) || st.watching.contains(&c.id))
            .collect();
        let with_drops = watched.iter().find(|c| c.drops);
        (
            with_drops.is_some(),
            with_drops.or(watched.first()).map(|c| c.id.clone()),
        )
    };
    let http = http(app);
    let auth = gql_auth(&token, &client_id);
    let campaigns = match drops::inventory(&http, auth).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("miner: drops: {e}");
            return IDLE;
        }
    };
    let progress: Vec<DropProgress> = campaigns
        .iter()
        .flat_map(|c| c.drops.iter().map(move |d| (c, d)))
        .filter(|(_, d)| !d.claimed && d.instance_id.is_none())
        .map(|(c, d)| DropProgress {
            name: d.name.clone(),
            game: c.game.clone(),
            image: d.image.clone(),
            minutes: d.watched_minutes,
            required: d.required_minutes,
        })
        .collect();
    let changed = {
        let mut st = lock(app);
        let changed = st.drop_progress != progress;
        st.drop_progress = progress;
        changed
    };
    if changed {
        super::emit_status(app);
    }
    let mut first = true;
    for c in &campaigns {
        for d in c.drops.iter().filter(|d| d.claimable()) {
            if !first {
                let pause = 5_000 + (now_ms() % 5_000) as u64;
                tokio::time::sleep(Duration::from_millis(pause)).await;
            }
            first = false;
            let Some(instance) = d.instance_id.as_deref() else {
                continue;
            };
            match drops::claim(&http, auth, instance).await {
                Ok(true) => {
                    let game = c.game.as_deref().map(|g| format!(" ({g})")).unwrap_or_default();
                    log::add(app, None, format!("Claimed the drop “{}”{game}.", d.name));
                    if let Some(id) = &credit_to {
                        let rec = Record {
                            n: Some(d.name.clone()),
                            ..Record::new(now_ms(), Kind::Drop, 0)
                        };
                        super::stats::record(app, &account, id, &rec);
                    }
                }
                Ok(false) => log::add(
                    app,
                    None,
                    format!(
                        "Couldn't claim “{}”. Is the game account linked on Twitch?",
                        d.name
                    ),
                ),
                Err(e) => eprintln!("miner: drop claim: {e}"),
            }
        }
    }
    if busy { BUSY } else { IDLE }
}
