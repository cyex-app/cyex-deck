//! IPC for the stream manager (spec 10, 1.35, X4): opening the tab, the channel's info,
//! chat modes, ads, raids, markers, polls and predictions, Stream Together and the OBS
//! status folder. Every action works on the manager account's own channel (D26).

use cyex_twitch::helix::{ads, channel, modes};
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;

use crate::chat::votes::VotesUpdate;
use cyex_twitch::gql::dashboard::ads::{AdEdit, AdSettings};
use cyex_twitch::gql::dashboard::raids::{RaidPick, RaidStatus};

use crate::streammanager::actions::{self, Mode, NewPoll, NewPrediction, PredictionEnd, Together};
use crate::streammanager::activity::Item;
use crate::streammanager::stats::Stats;
use crate::streammanager::{self, Broadcaster, obs};

/// What a freshly opened tab needs to draw.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmOpen")]
pub struct Opened {
    pub manager: Option<Broadcaster>,
    pub activity: Vec<Item>,
    pub stats: Option<Stats>,
    /// X4 viewer graph: (unix seconds, viewers).
    #[ts(type = "Array<[number, number]>")]
    pub series: Vec<(i64, u64)>,
    /// D2 A raid counting down right now.
    pub raid: Option<RaidStatus>,
}

/// The account that runs the manager, or null when none can (the tab hides).
#[tauri::command]
pub fn sm_manager(app: AppHandle) -> Option<Broadcaster> {
    streammanager::manager(&app)
}

#[tauri::command]
pub fn sm_open(app: AppHandle) -> Opened {
    let activity = streammanager::open(&app);
    Opened {
        manager: streammanager::manager(&app),
        activity,
        stats: streammanager::stats::latest(&app),
        series: streammanager::series(&app),
        raid: streammanager::live::raid(&app),
    }
}

#[tauri::command]
pub fn sm_close(app: AppHandle) {
    streammanager::close(&app);
}

#[tauri::command]
pub async fn sm_info(app: AppHandle) -> Result<channel::ChannelInfo, String> {
    actions::info(&app).await
}

#[tauri::command]
pub async fn sm_set_info(app: AppHandle, edit: channel::Edit) -> Result<(), String> {
    actions::set_info(&app, edit).await
}

#[tauri::command]
pub async fn sm_categories(app: AppHandle, query: String) -> Result<Vec<channel::Category>, String> {
    actions::categories(&app, query).await
}

#[tauri::command]
pub async fn sm_marker(app: AppHandle, description: String) -> Result<(), String> {
    actions::marker(&app, description).await
}

/// X4 The newest marker-log lines, newest first.
#[tauri::command]
pub fn sm_markers(app: AppHandle) -> Vec<String> {
    obs::markers(&app)
}

#[tauri::command]
pub async fn sm_modes(app: AppHandle) -> Result<modes::ChatModes, String> {
    actions::chat_modes(&app).await
}

#[tauri::command]
pub async fn sm_set_mode(app: AppHandle, mode: Mode) -> Result<modes::ChatModes, String> {
    actions::set_mode(&app, mode).await
}

#[tauri::command]
pub async fn sm_commercial(app: AppHandle, secs: u32) -> Result<u32, String> {
    actions::commercial(&app, secs).await
}

#[tauri::command]
pub async fn sm_ad_schedule(app: AppHandle) -> Result<ads::AdSchedule, String> {
    actions::ad_schedule(&app).await
}

#[tauri::command]
pub async fn sm_snooze(app: AppHandle) -> Result<ads::Snoozed, String> {
    actions::snooze(&app).await
}

#[tauri::command]
pub async fn sm_raid(app: AppHandle, login: String) -> Result<(), String> {
    actions::raid(&app, login).await
}

#[tauri::command]
pub async fn sm_unraid(app: AppHandle) -> Result<(), String> {
    actions::unraid(&app).await
}

#[tauri::command]
pub async fn sm_raid_now(app: AppHandle) -> Result<(), String> {
    actions::raid_now(&app).await
}

#[tauri::command]
pub async fn sm_raid_picks(app: AppHandle) -> Result<Vec<RaidPick>, String> {
    actions::raid_picks(&app).await
}

#[tauri::command]
pub async fn sm_alert(app: AppHandle, id: String, replay: bool) -> Result<(), String> {
    actions::alert(&app, id, replay).await
}

#[tauri::command]
pub async fn sm_ad_settings(app: AppHandle) -> Result<Option<AdSettings>, String> {
    actions::ad_settings(&app).await
}

#[tauri::command]
pub async fn sm_set_ad_settings(app: AppHandle, edit: AdEdit) -> Result<(), String> {
    actions::set_ad_settings(&app, edit).await
}

#[tauri::command]
pub async fn sm_join_answer(app: AppHandle, user_id: String, approve: bool) -> Result<(), String> {
    actions::join_answer(&app, user_id, approve).await
}

#[tauri::command]
pub async fn sm_auto_add(app: AppHandle, session: Option<String>, on: bool) -> Result<(), String> {
    actions::auto_add(&app, session, on).await
}

/// The channel's poll and prediction now; updates come as `chat:votes` for its login.
#[tauri::command]
pub fn sm_votes(app: AppHandle) -> Option<VotesUpdate> {
    let m = streammanager::manager(&app)?;
    crate::chat::votes::state(&app, &m.login)
}

#[tauri::command]
pub async fn sm_create_poll(app: AppHandle, poll: NewPoll) -> Result<(), String> {
    actions::create_poll(&app, poll).await
}

#[tauri::command]
pub async fn sm_end_poll(app: AppHandle, id: String, archive: bool) -> Result<(), String> {
    actions::end_poll(&app, id, archive).await
}

#[tauri::command]
pub async fn sm_create_prediction(app: AppHandle, prediction: NewPrediction) -> Result<(), String> {
    actions::create_prediction(&app, prediction).await
}

#[tauri::command]
pub async fn sm_end_prediction(app: AppHandle, id: String, end: PredictionEnd) -> Result<(), String> {
    actions::end_prediction(&app, id, end).await
}

#[tauri::command]
pub async fn sm_together(app: AppHandle) -> Result<Together, String> {
    actions::together(&app).await
}

#[tauri::command]
pub async fn sm_together_start(app: AppHandle) -> Result<(), String> {
    actions::together_start(&app).await
}

#[tauri::command]
pub async fn sm_together_end(app: AppHandle, session: String) -> Result<(), String> {
    actions::together_end(&app, session).await
}

#[tauri::command]
pub async fn sm_together_invite(app: AppHandle, session: String, login: String) -> Result<(), String> {
    actions::together_invite(&app, session, login).await
}

#[tauri::command]
pub async fn sm_together_uninvite(app: AppHandle, session: String, guest: String) -> Result<(), String> {
    actions::together_uninvite(&app, session, guest).await
}

#[tauri::command]
pub async fn sm_together_slot(
    app: AppHandle,
    session: String,
    guest: String,
    slot: String,
    clear: bool,
) -> Result<(), String> {
    actions::together_slot(&app, session, guest, slot, clear).await
}

/// X4 Where the status files go, for Settings.
#[tauri::command]
pub fn sm_obs_folder(app: AppHandle) -> Option<String> {
    obs::folder(&app).map(|p| p.to_string_lossy().into_owned())
}

/// X4 Opens the status folder in Explorer, making it first.
#[tauri::command]
pub fn sm_obs_open(app: AppHandle) -> Result<(), String> {
    let dir = obs::folder(&app).ok_or("No folder for the status files.")?;
    crate::paths::open_folder(&app, &dir)
}

/// V106-15 Picks the status folder. Nothing when cancelled.
#[tauri::command]
pub async fn sm_obs_pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    let mut d = app.dialog().file().set_title("Folder for the OBS files");
    if let Some(dir) = obs::folder(&app).filter(|p| p.is_dir()) {
        d = d.set_directory(dir);
    }
    let Some(picked) = d.blocking_pick_folder() else {
        return Ok(None);
    };
    Ok(Some(picked.into_path().map_err(|e| e.to_string())?.to_string_lossy().into_owned()))
}

/// V106-16 The status files, for the table in Settings.
#[tauri::command]
pub fn sm_obs_files(app: AppHandle) -> Vec<obs::ObsFile> {
    obs::table(&app)
}

/// V106-16 Opens one status file. Only the known names.
#[tauri::command]
pub fn sm_obs_open_file(app: AppHandle, name: String) -> Result<(), String> {
    use cyex_settings::Partition;
    use cyex_settings::schema::StreamManager;
    use tauri::Manager;
    let file = obs::table(&app)
        .into_iter()
        .find(|f| f.name == name)
        .ok_or("Not one of the status files.")?;
    if !file.exists {
        let sm: StreamManager = app.state::<crate::settings::Settings>().read(Partition::StreamManager);
        return Err(if sm.obs.enabled {
            "Not there yet. It’s written while the stream manager is open."
        } else {
            "Not there yet. Turn on Write status files first."
        }
        .into());
    }
    app.opener().open_path(file.path, None::<&str>).map_err(|e| e.to_string())
}

/// V107-R41 Stops the running bulk change (clips, videos, redemptions) after its request.
#[tauri::command]
pub fn sm_bulk_stop(app: AppHandle) {
    streammanager::bulk::stop(&app);
}

/// V107-R41 The limits the dashboard panels check, from Rust.
#[tauri::command]
pub fn sm_limits() -> streammanager::limits::DashboardLimits {
    streammanager::limits::limits()
}
