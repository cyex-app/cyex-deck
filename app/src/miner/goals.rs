//! W2 Community goals: when switched on, points go toward a channel's running goals, as
//! much as the goal still needs, this stream allows and the balance holds.

use cyex_core::number::grouped;
use cyex_miner::goal_share;
use cyex_miner::stats::{Kind, Record};
use cyex_twitch::gql::miner::{self as gm, PointsContext};
use tauri::AppHandle;

use super::log::LogKind;
use super::{gql_auth, http, log, now_ms};

pub async fn contribute(app: &AppHandle, account: &str, ctx: &PointsContext) {
    let Some((token, client_id)) = crate::auth::gql_token(app, account) else {
        return;
    };
    let mut balance = ctx.balance;
    for g in ctx.goals.iter().filter(|g| g.status == "STARTED" && g.in_stock) {
        let amount = goal_share(
            g.needed,
            g.contributed,
            g.per_stream_max,
            g.mine_this_stream,
            balance,
        );
        if amount == 0 {
            continue;
        }
        let name = super::claims::label(app, &ctx.channel_id);
        match gm::contribute(
            &http(app),
            gql_auth(&token, &client_id),
            &ctx.channel_id,
            &g.id,
            amount,
        )
        .await
        {
            Ok(None) => {
                balance -= amount;
                let rec = Record {
                    b: Some(balance),
                    n: Some(g.title.clone()),
                    ..Record::new(now_ms(), Kind::Goal, -(amount as i64))
                };
                super::stats::record(app, account, &ctx.channel_id, &rec);
                log::add(
                    app,
                    LogKind::Drop,
                    Some(&name),
                    format!("Put {} points toward “{}”.", grouped(amount), g.title),
                );
            }
            Ok(Some(code)) => {
                eprintln!("miner: goal in {name} refused: {code}");
                log::add(app, LogKind::Error, Some(&name), "Couldn’t read the goals.");
            }
            Err(e) => {
                eprintln!("miner: goal in {name}: {e}");
                log::add(app, LogKind::Error, Some(&name), "Couldn’t read the goals.");
            }
        }
    }
}
