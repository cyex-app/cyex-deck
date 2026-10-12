//! Hermes for the miner: an account's own points and bets (user topics), and the
//! watched channels' streams, raids, moments, predictions and goals (channel topics).
//! Chests are claimed for mined channels and, with C22, for channels open in a tab.

use cyex_core::number::grouped;
use cyex_miner::events::{self, Event as MinerEvent};
use cyex_miner::stats::{Kind as StatKind, Record};
use cyex_twitch::gql::predictions::Prediction;
use cyex_twitch::hermes::{Event, Kind};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use super::{lock, log, now_ms};

const BALANCE_EVENT: &str = "points:balance";

/// A balance changed, for the chat's points button.
#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BalanceUpdate {
    pub account: String,
    pub channel_id: String,
    #[ts(type = "number")]
    pub balance: u64,
}

/// A user topic: points or bets of `e.topic.channel_id`, the account.
pub fn on_user_event(app: &AppHandle, e: &Event) {
    let account = e.topic.channel_id.clone();
    let Some(ev) = events::parse(&e.payload) else {
        return;
    };
    let mining = lock(app).account.as_deref() == Some(account.as_str());
    match ev {
        MinerEvent::ClaimReady { channel_id, claim_id } => {
            let s = super::settings(app);
            let mined = mining && lock(app).channels.iter().any(|c| c.id == channel_id);
            let open = s.claim_in_tabs && super::tabs::open_with(app, &account, &channel_id);
            if mined || open {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    super::claims::bonus(&app, &account, &channel_id, &claim_id).await;
                });
            }
        }
        MinerEvent::Earned {
            channel_id,
            balance,
            gain,
            reason,
        } => {
            balance_changed(app, &account, &channel_id, balance);
            if mining {
                earned(app, &account, &channel_id, balance, gain, &reason);
            }
        }
        MinerEvent::Spent { channel_id, balance } => balance_changed(app, &account, &channel_id, balance),
        MinerEvent::BetResult {
            channel_id,
            result,
            won,
            ..
        } if mining => bet_result(app, &account, &channel_id, &result, won),
        _ => {}
    }
}

fn balance_changed(app: &AppHandle, account: &str, channel_id: &str, balance: u64) {
    let _ = app.emit(
        BALANCE_EVENT,
        BalanceUpdate {
            account: account.to_owned(),
            channel_id: channel_id.to_owned(),
            balance,
        },
    );
    let changed = {
        let mut st = lock(app);
        let mining = st.account.as_deref() == Some(account);
        match st.channels.iter_mut().find(|c| c.id == channel_id) {
            Some(c) if mining && c.balance != Some(balance) => {
                c.balance = Some(balance);
                true
            }
            _ => false,
        }
    };
    if changed {
        super::emit_status(app);
    }
}

fn earned(app: &AppHandle, account: &str, channel_id: &str, balance: u64, gain: u64, reason: &str) {
    let kind = StatKind::from_reason(reason);
    if kind == StatKind::Streak {
        let mut st = lock(app);
        if let Some(c) = st.channels.iter_mut().find(|c| c.id == channel_id) {
            c.streak.earned();
        }
    }
    let rec = Record {
        b: Some(balance),
        n: (kind == StatKind::Other).then(|| reason.to_owned()),
        ..Record::new(now_ms(), kind, gain as i64)
    };
    super::stats::record(app, account, channel_id, &rec);
    let what = match kind {
        StatKind::Chest => "from a bonus chest",
        StatKind::Streak => "for the watch streak",
        StatKind::Raid => "for joining a raid",
        StatKind::Moment => "for a moment",
        _ => return,
    };
    let name = super::claims::label(app, channel_id);
    log::add(app, Some(&name), format!("+{} {what}.", grouped(gain)));
}

fn bet_result(app: &AppHandle, account: &str, channel_id: &str, result: &str, won: u64) {
    let (kind, text) = match result {
        "WIN" => (StatKind::Won, format!("Won {} points on a bet.", grouped(won))),
        "LOSE" => (StatKind::Lost, "Lost a bet.".to_owned()),
        "REFUND" => (StatKind::Refund, "The bet was refunded.".to_owned()),
        _ => return,
    };
    let rec = Record::new(now_ms(), kind, won as i64);
    super::stats::record(app, account, channel_id, &rec);
    let name = super::claims::label(app, channel_id);
    log::add(app, Some(&name), text);
}

/// A channel topic. Only channels the miner follows count; chat may follow the same.
pub fn on_channel_event(app: &AppHandle, e: &Event) {
    let id = e.topic.channel_id.clone();
    let Some(account) = ({
        let st = lock(app);
        st.account.clone().filter(|_| st.followed.contains(&id))
    }) else {
        return;
    };
    let s = super::settings(app);
    if e.topic.kind == Kind::Predictions {
        if s.bets.enabled
            && let Some(p) = Prediction::from_hermes(&e.payload, None)
        {
            super::bets::consider(app, &account, &id, p);
        }
        return;
    }
    let Some(ev) = events::parse(&e.payload) else {
        return;
    };
    match ev {
        MinerEvent::StreamUp => super::channels::nudge(app),
        MinerEvent::StreamDown => {
            {
                let mut st = lock(app);
                if let Some(c) = st.channels.iter_mut().find(|c| c.id == id) {
                    super::channels::went_offline(c, now_ms());
                }
                st.slots.retain(|s| *s != id);
            }
            super::emit_status(app);
        }
        MinerEvent::Raid { raid_id, target } if s.join_raids && lock(app).slots.contains(&id) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                super::claims::raid(&app, &account, &id, &raid_id, &target).await;
            });
        }
        MinerEvent::Moment { moment_id } if s.claim_moments => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                super::claims::moment(&app, &account, &id, &moment_id).await;
            });
        }
        MinerEvent::Goals if s.goals => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move { goals_changed(&app, &account, &id).await });
        }
        _ => {}
    }
}

async fn goals_changed(app: &AppHandle, account: &str, channel_id: &str) {
    let Some(login) = super::name_of(app, channel_id).map(|n| n.0) else {
        return;
    };
    let Some((token, client_id)) = crate::auth::gql_token(app, account) else {
        return;
    };
    let auth = super::gql_auth(&token, &client_id);
    if let Ok(Some(ctx)) = cyex_twitch::gql::miner::points_context(&super::http(app), auth, &login).await {
        super::goals::contribute(app, account, &ctx).await;
    }
}
