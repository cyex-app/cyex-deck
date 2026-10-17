//! Prediction bets (off by default): once per prediction on a watched channel, at the
//! moment the delay setting picks, with the side and amount the strategy picks
//! (`cyex_miner::bet`). Twitch's prediction terms are accepted on the first bet.

use std::time::Duration;

use cyex_core::number::grouped;
use cyex_miner::bet::{self, Side};
use cyex_miner::stats::{Kind, Record};
use cyex_twitch::gql::predictions::{self, Prediction};
use tauri::AppHandle;

use super::log::LogKind;
use super::{gql_auth, http, lock, log, now_ms};

/// A prediction was created or changed on a watched channel.
pub fn consider(app: &AppHandle, account: &str, channel_id: &str, p: Prediction) {
    if p.status != "ACTIVE" || p.mine.is_some() || !lock(app).bets.insert(p.id.clone()) {
        return;
    }
    let s = super::settings(app).bets;
    let created = time::OffsetDateTime::parse(&p.created_at, &time::format_description::well_known::Rfc3339)
        .map(|t| (t.unix_timestamp_nanos() / 1_000_000) as i64)
        .unwrap_or_else(|_| now_ms());
    let elapsed = (now_ms() - created).max(0) as f64 / 1000.0;
    let wait = bet::wait_secs(&s, p.window_secs, elapsed);
    let (app, account, channel_id) = (app.clone(), account.to_owned(), channel_id.to_owned());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs_f64(wait)).await;
        place(&app, &account, &channel_id, &p.id).await;
    });
}

async fn place(app: &AppHandle, account: &str, channel_id: &str, event_id: &str) {
    let s = super::settings(app).bets;
    if !s.enabled || lock(app).account.as_deref() != Some(account) {
        return;
    }
    let Some((login, name)) = super::name_of(app, channel_id) else {
        return;
    };
    let Some((token, client_id)) = crate::auth::gql_token(app, account) else {
        return;
    };
    let http = http(app);
    let auth = gql_auth(&token, &client_id);
    // Read it again: the sides have moved since it opened.
    let state = match predictions::prediction(&http, auth, &login).await {
        Ok(state) => state,
        Err(e) => {
            eprintln!("miner: prediction in {login}: {e}");
            return log::add(app, LogKind::Error, Some(&name), "Couldn’t place the bet.");
        }
    };
    let Some(p) = state
        .prediction
        .filter(|p| p.id == event_id && p.status == "ACTIVE" && p.mine.is_none())
    else {
        return;
    };
    let sides: Vec<Side> = p
        .outcomes
        .iter()
        .map(|o| Side {
            users: o.users,
            points: o.points,
            top: o.top.as_ref().map_or(0, |t| t.1),
        })
        .collect();
    let Some(i) = bet::choose(&s, &sides) else { return };
    if !bet::passes(&s.filter, &sides, i) {
        return log::add(
            app,
            LogKind::Bet,
            Some(&name),
            format!("Skipped the bet on “{}”: the filter said no.", p.title),
        );
    }
    let jitter = 1 + (now_ms() % 5) as u64;
    let amount = bet::amount(&s, state.balance.unwrap_or(0), sides[i], jitter);
    if amount == 0 {
        return;
    }
    if !state.accepted
        && let Err(e) = predictions::accept_terms(&http, auth).await
    {
        eprintln!("miner: prediction terms: {e}");
        return log::add(app, LogKind::Error, Some(&name), "Couldn’t place the bet.");
    }
    let outcome = &p.outcomes[i];
    match predictions::predict(&http, auth, &p.id, &outcome.id, amount).await {
        Ok(None) => {
            let rec = Record {
                b: state.balance.map(|b| b.saturating_sub(amount)),
                n: Some(p.title.clone()),
                ..Record::new(now_ms(), Kind::Bet, -(amount as i64))
            };
            super::stats::record(app, account, channel_id, &rec);
            log::add(
                app,
                LogKind::Bet,
                Some(&name),
                format!("Bet {} on “{}” in “{}”.", grouped(amount), outcome.title, p.title),
            );
        }
        Ok(Some(code)) => {
            eprintln!("miner: bet in {login} refused: {code}");
            log::add(app, LogKind::Error, Some(&name), "Couldn’t place the bet.");
        }
        Err(e) => {
            eprintln!("miner: bet in {login}: {e}");
            log::add(app, LogKind::Error, Some(&name), "Couldn’t place the bet.");
        }
    }
}
