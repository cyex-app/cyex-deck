//! Player commands (spec 03): sessions for hls.js, clips, VOD extras, stories and the
//! channel pages next to the stream (1.21).

use cyex_chat::build::replay::{Comment, replay};
use cyex_chat::build::{Colors, Ctx};
use cyex_chat::model::ChatMessage;
use cyex_twitch::gql::{
    self, About, Clip, ClipPeriod, ClipSort, Story, StoryRing, Storyboard, Video, VideoKind, VideoSort,
};
use tauri::{AppHandle, Manager, Window};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::auth::{Auth, gql_token, unix_now};
use crate::player::{self, PlayerSession, Players};

type CmdResult<T> = Result<T, String>;

/// 1.1 A live stream for this window's player, signed in as `account` when it can (D19).
/// `replay`: the player's key for going back in what it played (V108-N24).
#[tauri::command]
pub fn player_live(
    app: AppHandle,
    window: Window,
    login: String,
    account: Option<String>,
    replay: Option<String>,
) -> PlayerSession {
    player::open_live(&app, window.label(), &login, account.as_deref(), replay.as_deref())
}

/// V108-N24 Live session `id`'s replay from `at` (ms since 1970), None when it holds nothing.
#[tauri::command]
pub fn player_replay(app: AppHandle, id: String, at: f64, audio: bool) -> Option<player::replay::ReplayStart> {
    player::replay::start(&app, &id, at as i64, audio)
}

/// V108-N24 What live session `id`'s replay holds now.
#[tauri::command]
pub fn player_replay_range(app: AppHandle, id: String, audio: bool) -> Option<player::replay::ReplayRange> {
    player::replay::range(&app, &id, audio)
}

/// V108-N24 The player couldn't load this replay segment.
#[tauri::command]
pub fn player_replay_gone(app: AppHandle, id: String, uri: String) {
    player::replay::gone(&app, &id, &uri);
}

/// 1.1 A VOD for this window's player.
#[tauri::command]
pub fn player_vod(app: AppHandle, window: Window, id: String, account: Option<String>) -> PlayerSession {
    player::open_vod(&app, window.label(), &id, account.as_deref())
}

#[tauri::command]
pub fn player_close(app: AppHandle, id: String) {
    player::close(&app, &id);
}

/// V103-07 Whether another player plays this channel now, so a new one can start paused.
#[tauri::command]
pub fn player_playing(app: AppHandle, login: String) -> bool {
    player::plays(&app, &login)
}

/// 1.19, 1.20 A clip with its video files, or where it is (V109-F73: being made, failed, gone).
#[tauri::command]
pub async fn player_clip(app: AppHandle, slug: String) -> CmdResult<player::clips::ClipLookup> {
    player::clips::lookup(&app, &slug).await.map_err(message)
}

#[tauri::command]
pub async fn player_video(app: AppHandle, id: String) -> CmdResult<Option<Video>> {
    gql::video(&http(&app), &id).await.map_err(message)
}

/// P6
#[tauri::command]
pub async fn player_storyboard(app: AppHandle, id: String) -> CmdResult<Option<Storyboard>> {
    gql::storyboard(&http(&app), &id).await.map_err(message)
}

/// P8 Chat from `offset` seconds on, as chat lines.
#[tauri::command]
pub async fn player_comments(app: AppHandle, video_id: String, offset: u32) -> CmdResult<Vec<ChatMessage>> {
    let comments = gql::comments(&http(&app), &video_id, offset).await.map_err(message)?;
    let mut colors = Colors::default();
    let mut ctx = Ctx::new(None, &mut colors, 0);
    Ok(comments
        .iter()
        .map(|c| {
            let ts = OffsetDateTime::parse(&c.created_at, &Rfc3339)
                .map(|t| (t.unix_timestamp_nanos() / 1_000_000) as i64)
                .unwrap_or(0);
            let mut msg = replay(
                &Comment {
                    id: &c.id,
                    ts,
                    user_id: &c.user_id,
                    login: &c.login,
                    display: &c.display,
                    color: c.color.as_deref(),
                    badges: &c.badges,
                    pieces: &c.pieces,
                },
                &mut ctx,
            );
            // The replay shows lines by their place in the VOD.
            msg.id = format!("{}@{}", c.offset, c.id);
            msg
        })
        .collect())
}

/// P5 Where the VOD was left, in seconds.
#[tauri::command]
pub fn player_position(app: AppHandle, id: String) -> Option<f64> {
    app.state::<Players>().positions.get(&id)
}

#[tauri::command]
pub fn player_save_position(app: AppHandle, id: String, at: f64, length: f64) {
    app.state::<Players>().positions.save(&id, at, length, unix_now());
}

/// V107-Mv How far each of these VODs was watched; unstarted ones are left out.
#[tauri::command]
pub fn player_progress(app: AppHandle, ids: Vec<String>) -> Vec<player::positions::VodProgress> {
    app.state::<Players>().positions.progress(&ids)
}

/// P9, V102-08 Clips the last seconds of the stream as `account`.
#[tauri::command]
/// V109-F73 `at`: where the player is in the broadcast (s), for a GQL clip's end.
pub async fn player_create_clip(
    app: AppHandle,
    account: String,
    channel_id: String,
    at: Option<f64>,
) -> CmdResult<player::clips::MadeClip> {
    player::clips::create(&app, &account, &channel_id, at).await
}

/// V108-N25 Clips the live stream up to `offset_secs` into the broadcast, as `account`.
#[tauri::command]
pub async fn player_create_clip_at(
    app: AppHandle,
    account: String,
    channel_id: String,
    offset_secs: f64,
) -> CmdResult<player::clips::MadeClip> {
    player::clips::create_at(&app, &account, &channel_id, offset_secs).await
}

/// V108-N01 Clips a VOD of the channel `offset_secs` into it, as `account`.
#[tauri::command]
pub async fn player_create_clip_from_video(
    app: AppHandle,
    account: String,
    channel_id: String,
    video_id: String,
    offset_secs: f64,
) -> CmdResult<player::clips::MadeClip> {
    player::clips::create_from_video(&app, &account, &channel_id, &video_id, offset_secs).await
}

/// V108-N02 Saves a clip as an MP4 through a save dialog. `None`: cancelled; else the path.
#[tauri::command]
pub async fn player_clip_save(app: AppHandle, slug: String, height: Option<u32>) -> CmdResult<Option<String>> {
    player::clip_save::save(&app, &slug, height).await
}

/// V108-N02 Stops a clip save that runs.
#[tauri::command]
pub fn player_clip_save_cancel(slug: String) -> bool {
    player::clip_save::cancel(&slug)
}

/// P10 The proxy as settings show it: `host:port`.
#[tauri::command]
pub fn player_proxy() -> Option<String> {
    player::proxy::load().map(|u| player::proxy::shown(&u))
}

#[tauri::command]
pub fn player_set_proxy(app: AppHandle, url: Option<String>) -> CmdResult<()> {
    player::set_proxy(&app, url.as_deref())
}

/// 1.21
#[tauri::command]
pub async fn channel_about(app: AppHandle, login: String) -> CmdResult<Option<About>> {
    let http = http(&app);
    let mut about = gql::about(&http, &login).await.map_err(message)?;
    // V106-24
    if let Some(a) = about.as_mut() {
        player::blank::drop_blank(&http, a).await;
    }
    Ok(about)
}

/// 1.21
#[tauri::command]
pub async fn channel_clips(app: AppHandle, login: String, period: ClipPeriod, sort: ClipSort) -> CmdResult<Vec<Clip>> {
    gql::channel_clips(&http(&app), &login, period, sort).await.map_err(message)
}

/// 1.21 All kinds when `kind` is null.
#[tauri::command]
pub async fn channel_videos(
    app: AppHandle,
    login: String,
    kind: Option<VideoKind>,
    sort: VideoSort,
) -> CmdResult<Vec<Video>> {
    gql::channel_videos(&http(&app), &login, kind, sort).await.map_err(message)
}

/// 1.1 Stories need a GQL sign-in: without one, there's no ring.
#[tauri::command]
pub async fn channel_story_ring(app: AppHandle, channel_id: String, account: Option<String>) -> Option<StoryRing> {
    let (token, client_id) = gql_token(&app, account.as_deref()?)?;
    let auth = crate::auth::gql_auth(&token, &client_id);
    gql::story_ring(&http(&app), auth, &channel_id).await.ok().flatten()
}

#[tauri::command]
pub async fn channel_stories(app: AppHandle, channel_id: String, account: String) -> CmdResult<Vec<Story>> {
    let (token, client_id) = gql_token(&app, &account).ok_or("Stories need a GQL sign-in.")?;
    let auth = crate::auth::gql_auth(&token, &client_id);
    gql::stories(&http(&app), auth, &channel_id).await.map_err(message)
}

/// Marks stories up to `created_at` as seen, as twitch.tv does after viewing.
#[tauri::command]
pub async fn channel_stories_seen(app: AppHandle, channel_id: String, account: String, created_at: String) {
    let Some((token, client_id)) = gql_token(&app, &account) else { return };
    let auth = crate::auth::gql_auth(&token, &client_id);
    if let Err(e) = gql::mark_seen(&http(&app), auth, &channel_id, &created_at).await {
        eprintln!("player: couldn't mark stories seen: {e}");
    }
}

fn http(app: &AppHandle) -> cyex_twitch::Http {
    app.state::<Auth>().http.clone()
}

fn message(e: cyex_twitch::Error) -> String {
    if e.is_network() {
        "Couldn’t reach Twitch.".into()
    } else {
        e.to_string()
    }
}
