//! IPC for the stream manager (spec 10, 1.35, X4): opening the tab, the channel's info,
//! chat modes, ads, raids, markers, polls and predictions, Stream Together and the OBS
//! status folder. Every action works on the manager account's own channel (D26).

use cyex_twitch::helix::{ads, channel, modes};
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;

use crate::chat::votes::VotesUpdate;
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
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}
