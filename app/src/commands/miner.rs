//! Miner commands (08): status, log (W4), stats (1.32), adding a channel (1.34) and
//! picking the two mined first (1.44). The list itself is edited as settings.

use cyex_miner::stats::Stats;
use cyex_settings::Partition;
use cyex_settings::schema::{Miner, TabChannel};
use tauri::{AppHandle, Manager};

use crate::miner::log::LogLine;
use crate::miner::stats::{self, StatsChannel};
use crate::miner::{self as m, MinerStatus};
use crate::settings::Settings;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn miner_status(app: AppHandle) -> MinerStatus {
    m::status(&app)
}

#[tauri::command]
pub fn miner_log(app: AppHandle) -> Vec<LogLine> {
    m::log::lines(&app)
}

/// V106-27 Empties the log in every window.
#[tauri::command]
pub fn miner_log_clear(app: AppHandle) {
    m::log::clear(&app);
}

/// Whose stats show: the mining account, else the one picked, else the default.
fn stats_account(app: &AppHandle) -> Option<String> {
    m::status(app)
        .account
        .or_else(|| app.state::<Settings>().read::<Miner>(Partition::Miner).account)
        .or_else(|| crate::auth::accounts(app).default_id)
}

#[tauri::command]
pub fn miner_stats_channels(app: AppHandle) -> Vec<StatsChannel> {
    stats_account(&app).map_or_else(Vec::new, |a| stats::channels(&app, &a))
}

/// One channel's stats, or all channels' with `channel` unset.
#[tauri::command]
pub async fn miner_stats(app: AppHandle, channel: Option<String>) -> Stats {
    let Some(account) = stats_account(&app) else {
        return Stats::default();
    };
    tauri::async_runtime::spawn_blocking(move || stats::summary(&app, &account, channel.as_deref()))
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub fn miner_stats_clear(app: AppHandle, channel: Option<String>) -> CmdResult<()> {
    let account = stats_account(&app).ok_or("No account.")?;
    stats::clear(&app, &account, channel.as_deref())
}

/// 1.34 Adds a channel at the bottom of the list. False if it was there already.
#[tauri::command]
pub fn miner_add(app: AppHandle, channel: TabChannel) -> CmdResult<bool> {
    let mut added = false;
    app.state::<Settings>()
        .update::<Miner>(&app, Partition::Miner, |s| {
            added = s.add(channel);
            added
        })?;
    Ok(added)
}

/// 1.44 Puts a channel among the two mined first, or takes it off with `on` false.
/// Returns the name of the channel that made room, if one did.
#[tauri::command]
pub fn miner_first(app: AppHandle, channel: TabChannel, on: bool) -> CmdResult<Option<String>> {
    let mut out = None;
    app.state::<Settings>()
        .update::<Miner>(&app, Partition::Miner, |s| {
            if on {
                out = s.put_first(channel).map(|c| c.name);
                true
            } else {
                let before = s.first.len();
                s.first.retain(|c| c.id != channel.id);
                s.first.len() != before
            }
        })?;
    Ok(out)
}
