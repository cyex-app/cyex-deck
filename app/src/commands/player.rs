//! Player commands (spec 03): sessions for hls.js, clips, VOD extras, stories and the
//! channel pages next to the stream (1.21).

use cyex_chat::build::replay::{Comment, replay};
use cyex_chat::build::{Colors, Ctx};
use cyex_chat::model::ChatMessage;
use cyex_twitch::gql::{
    self, About, Clip, ClipPeriod, ClipPlayback, ClipSort, Story, StoryRing, Storyboard, Video, VideoKind, VideoSort,
};
use tauri::{AppHandle, Manager, Window};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::auth::{Auth, gql_token, unix_now};
use crate::player::{self, PlayerSession, Players};

type CmdResult<T> = Result<T, String>;

/// 1.1 A live stream for this window's player, signed in as `account` when it can (D19).
#[tauri::command]
pub fn player_live(app: AppHandle, window: Window, login: String, account: Option<String>) -> PlayerSession {
    player::open_live(&app, window.label(), &login, account.as_deref())
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

/// 1.19, 1.20 A clip with its video files. `None` if it was deleted.
#[tauri::command]
pub async fn player_clip(app: AppHandle, slug: String) -> CmdResult<Option<ClipPlayback>> {
    gql::clip(&http(&app), &slug).await.map_err(|e| match e {
        // V102-08
        cyex_twitch::Error::Refused(_) => "Twitch couldn't make this clip.".into(),
        e => message(e),
    })
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

/// P9, V102-08 Clips the last seconds of the stream as `account`.
#[tauri::command]
pub async fn player_create_clip(app: AppHandle, account: String, channel_id: String) -> CmdResult<player::clips::MadeClip> {
    player::clips::create(&app, &account, &channel_id).await
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
    gql::about(&http(&app), &login).await.map_err(message)
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
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    gql::story_ring(&http(&app), auth, &channel_id).await.ok().flatten()
}

#[tauri::command]
pub async fn channel_stories(app: AppHandle, channel_id: String, account: String) -> CmdResult<Vec<Story>> {
    let (token, client_id) = gql_token(&app, &account).ok_or("Stories need a GQL sign-in.")?;
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    gql::stories(&http(&app), auth, &channel_id).await.map_err(message)
}

/// Marks stories up to `created_at` as seen, as twitch.tv does after viewing.
#[tauri::command]
pub async fn channel_stories_seen(app: AppHandle, channel_id: String, account: String, created_at: String) {
    let Some((token, client_id)) = gql_token(&app, &account) else { return };
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    if let Err(e) = gql::mark_seen(&http(&app), auth, &channel_id, &created_at).await {
        eprintln!("player: couldn't mark stories seen: {e}");
    }
}

fn http(app: &AppHandle) -> cyex_twitch::Http {
    app.state::<Auth>().http.clone()
}

fn message(e: cyex_twitch::Error) -> String {
    if e.is_network() {
        "Couldn't reach Twitch.".into()
    } else {
        e.to_string()
    }
}
