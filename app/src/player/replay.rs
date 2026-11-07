//! V108-N24 Replay what you watched: a live session's replay (`cyex_player::replay`) as the
//! player asks for it. The playlists are served at `live/<id>/replay.m3u8` and
//! `live/<id>/replay-audio.m3u8` (`protocol.rs`).

use cyex_player::replay::Shared;
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use super::{Kind, PlayerSession, protocol};

/// Where the replay starts: the session to load and the spot in its playlist.
#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReplayStart {
    pub session: PlayerSession,
    /// Seconds into the playlist.
    pub offset: f64,
}

/// What the replay holds now, as program date times in ms.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReplayRange {
    #[ts(type = "number")]
    pub from: i64,
    #[ts(type = "number")]
    pub to: i64,
    pub ended: bool,
    /// V109-F72 How long the replay keeps, ms (`Replay::keep_ms`).
    #[ts(type = "number")]
    pub keep: i64,
}

fn ring(app: &AppHandle, id: &str) -> Option<Shared> {
    match super::peek(app, id)? {
        Kind::Live(live) => live.replay_ring().cloned(),
        Kind::Vod(_) => None,
    }
}

/// The replay of live session `id` from `at` (ms), None when this track doesn't hold it (the
/// player goes live then: a switch to or from audio only would land minutes off). `offset`
/// is only a first guess: the ring may trim before the player reads the playlist, so the
/// player places `at` by the segments' dates (V108-N24, engine.svelte.ts).
pub fn start(app: &AppHandle, id: &str, at: i64, audio: bool) -> Option<ReplayStart> {
    let ring = ring(app, id)?;
    let offset = {
        let r = ring.lock().unwrap_or_else(|p| p.into_inner());
        if !r.covers(audio, at) {
            return None;
        }
        r.offset(audio, at)?
    };
    let base = format!("{}live/{id}/", protocol::base());
    Some(ReplayStart {
        session: PlayerSession {
            id: id.to_owned(),
            url: format!("{base}replay.m3u8"),
            audio: format!("{base}replay-audio.m3u8"),
        },
        offset,
    })
}

pub fn range(app: &AppHandle, id: &str, audio: bool) -> Option<ReplayRange> {
    let ring = ring(app, id)?;
    let r = ring.lock().unwrap_or_else(|p| p.into_inner());
    let range = r.range(audio)?;
    Some(ReplayRange {
        from: range.from,
        to: range.to,
        ended: r.ended(),
        keep: r.keep_ms(),
    })
}

/// The player couldn't load `uri`: the replay learns how long Twitch keeps segments.
pub fn gone(app: &AppHandle, id: &str, uri: &str) {
    if let Some(ring) = ring(app, id) {
        ring.lock().unwrap_or_else(|p| p.into_inner()).gone(uri);
    }
}
