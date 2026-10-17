//! V1051-01 The app's own sounds (U6 chime, mention and highlight files V104-01/V105-17,
//! live alerts, whispers) play here, natively, so Windows' volume mixer lists them as
//! "cyex deck", apart from streams ("cyex deck - streams", WebView2's own process), or with
//! them as one entry. The page decodes a sound file the first time (Chromium reads every
//! type in `SOUND_TYPES`; Windows can't read ogg and opus). Settings → Sound.
//! V106-34 Every app sound goes through [`alert`], which stays quiet while a cyex deck window
//! is in front unless the user turned that on; Settings' Play and test buttons use [`preview`].

mod cache;
mod chime;
mod look;

pub use look::Look;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use cyex_settings::Partition;
use cyex_settings::schema::{General, Highlights, OutputDevice, is_sound_file};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::settings::Settings;
use crate::windows::MAIN;
use cache::Cache;

/// The rate everything here plays at; the page decodes to it.
pub const RATE: u32 = 48_000;
/// The longest part of a sound file that plays.
pub const MAX_SECONDS: usize = 30;
/// Decodes asked for and not answered yet; older ones are forgotten.
const MAX_PENDING: usize = 8;
/// A decode the page didn't answer (it wasn't loaded yet, or reloaded) is asked again after this.
const PENDING_FOR: Duration = Duration::from_secs(10);
const DECODE_EVENT: &str = "sound:decode";

/// The main window is asked to decode this file (`sound_decoded` answers with `id`).
#[derive(Clone, Serialize, TS)]
#[ts(export)]
pub struct SoundDecode {
    pub id: u32,
    pub path: String,
}

#[derive(Default)]
struct State {
    cache: Cache,
    pending: VecDeque<(u32, String, Instant)>,
}

pub struct Sounds {
    state: Mutex<State>,
    next: AtomicU32,
    chime: OnceLock<Arc<[f32]>>,
    watcher: crate::sys::audio::Watcher,
    /// V1051-01 Pages holding the microphone permission right now (see [`permit`]).
    permits: Mutex<usize>,
}

/// Before anything can ask for a sound (chat, alerts, whispers start later in setup).
pub fn init(app: &AppHandle) {
    let s: General = app.state::<Settings>().read(Partition::General);
    let (streams, sounds) = look::looks(s.sound.separate_streams);
    let icon = std::env::current_exe()
        .map(|p| format!("{},0", p.display()))
        .unwrap_or_default();
    app.manage(Sounds {
        state: Mutex::default(),
        next: AtomicU32::new(1),
        chime: OnceLock::new(),
        watcher: crate::sys::audio::Watcher::start(icon, streams, sounds),
        permits: Mutex::default(),
    });
    crate::sys::audio::prime();
}

/// General changed: the mixer's look follows.
pub fn on_settings(app: &AppHandle) {
    let s: General = app.state::<Settings>().read(Partition::General);
    let (streams, sounds) = look::looks(s.sound.separate_streams);
    if let Some(sounds_state) = app.try_state::<Sounds>() {
        sounds_state.watcher.restyle(streams, sounds);
    }
}

/// V1051-01 The page may use the microphone only while it picks a speaker (`setSinkId`
/// and the device list need it): a page asks with `on` before and `off` right after, and
/// the permission is back to the default once nobody holds it. Returns when WebView2 has it.
pub async fn permit(app: &AppHandle, on: bool) -> Result<(), String> {
    let main = app.get_webview_window(MAIN).ok_or("No window.")?;
    let sounds = app.try_state::<Sounds>().ok_or("No sound.")?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let mut held = sounds.permits.lock().unwrap_or_else(|p| p.into_inner());
        if on {
            *held += 1;
        } else {
            *held = held.saturating_sub(1);
        }
        // Only the first holder grants and the last one resets. Issued under the lock, so
        // WebView2 sees them in order.
        if (on && *held == 1) || (!on && *held == 0) {
            crate::sys::allow_speaker_choice(&main, on, move || {
                let _ = tx.send(());
            });
        } else {
            let _ = tx.send(());
        }
    }
    tokio::time::timeout(Duration::from_secs(3), rx)
        .await
        .map_err(|_| "WebView2 didn’t answer.".to_string())?
        .map_err(|_| "WebView2 refused.".to_string())
}

/// The main page loaded: nobody holds the permission, and one left on by an older version
/// (or a page that died mid-pick) is taken back.
pub fn reset_permit(app: &AppHandle) {
    let Some(main) = app.get_webview_window(MAIN) else {
        return;
    };
    if let Some(sounds) = app.try_state::<Sounds>() {
        *sounds.permits.lock().unwrap_or_else(|p| p.into_inner()) = 0;
    }
    crate::sys::allow_speaker_choice(&main, false, || {});
}

/// U6, V104-01 A mention, highlight, live alert or whisper sound: its file, or the chime.
/// Streamer mode can mute them (U4). V106-34 Quiet while cyex deck is in front, unless the
/// user said otherwise (any cyex window counts).
pub fn alert(app: &AppHandle, file: Option<&str>) {
    if crate::streamer::hides(app, |s| s.mute_sounds) {
        return;
    }
    let s: General = app.state::<Settings>().read(Partition::General);
    if !may_play(crate::toasts::app_in_front(app), s.sound.play_in_front) {
        return;
    }
    play_file(app, file);
}

/// Settings' Play and test buttons: always, you clicked them in the app (V106-34).
/// Streamer mode can still mute them (U4).
pub fn preview(app: &AppHandle, file: Option<&str>) {
    if crate::streamer::hides(app, |s| s.mute_sounds) {
        return;
    }
    play_file(app, file);
}

/// V106-34 An app sound plays unless cyex deck is in front and the user said not then.
fn may_play(in_front: bool, play_in_front: bool) -> bool {
    play_in_front || !in_front
}

/// A sound file, or the chime when there's none.
fn play_file(app: &AppHandle, file: Option<&str>) {
    let Some(sounds) = app.try_state::<Sounds>() else {
        return;
    };
    let Some(path) = file.filter(|p| !p.is_empty()) else {
        return play(app, sounds.chime());
    };
    let mut state = sounds.state.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(pcm) = state.cache.get(path) {
        drop(state);
        return play(app, pcm);
    }
    state
        .pending
        .retain(|(_, _, asked)| asked.elapsed() < PENDING_FOR);
    if state.pending.iter().any(|(_, p, _)| p == path) {
        return;
    }
    let id = sounds.next.fetch_add(1, Ordering::Relaxed);
    state.pending.push_back((id, path.to_owned(), Instant::now()));
    if state.pending.len() > MAX_PENDING {
        state.pending.pop_front();
    }
    drop(state);
    let asked = app.emit_to(
        MAIN,
        DECODE_EVENT,
        SoundDecode {
            id,
            path: path.to_owned(),
        },
    );
    // No page to ask, or none that answers (not loaded yet, reloaded): the chime plays instead
    // of nothing.
    if asked.is_err() {
        if take_pending(&sounds, id) {
            play(app, sounds.chime());
        }
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(PENDING_FOR).await;
        if let Some(sounds) = app.try_state::<Sounds>()
            && take_pending(&sounds, id)
        {
            play(&app, sounds.chime());
        }
    });
}

/// Forgets decode `id` if it is still unanswered. True when it was.
fn take_pending(sounds: &Sounds, id: u32) -> bool {
    let mut state = sounds.state.lock().unwrap_or_else(|p| p.into_inner());
    let Some(at) = state.pending.iter().position(|(i, _, _)| *i == id) else {
        return false;
    };
    state.pending.remove(at);
    true
}

/// The page's answer to [`SoundDecode`]: interleaved stereo f32 (little-endian) at
/// [`RATE`], or nothing when the file wouldn't decode (the chime plays, as before).
pub fn decoded(app: &AppHandle, id: u32, bytes: &[u8]) -> Result<(), String> {
    let sounds = app.try_state::<Sounds>().ok_or("Sounds aren’t ready yet.")?;
    let path = {
        let mut state = sounds.state.lock().unwrap_or_else(|p| p.into_inner());
        let at = state
            .pending
            .iter()
            .position(|(i, _, _)| *i == id)
            .ok_or("Not a sound the app asked for.")?;
        state.pending.remove(at).map(|(_, p, _)| p).unwrap_or_default()
    };
    if bytes.is_empty() {
        play(app, sounds.chime());
        return Ok(());
    }
    let pcm: Arc<[f32]> = samples(bytes)?.into();
    sounds
        .state
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .cache
        .put(&path, pcm.clone());
    play(app, pcm);
    Ok(())
}

/// Raw bytes to samples: whole stereo frames, at most [`MAX_SECONDS`].
fn samples(bytes: &[u8]) -> Result<Vec<f32>, String> {
    if !bytes.len().is_multiple_of(8) || bytes.len() > MAX_SECONDS * RATE as usize * 8 {
        return Err("That sound can’t be played.".into());
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect())
}

impl Sounds {
    fn chime(&self) -> Arc<[f32]> {
        self.chime.get_or_init(|| chime::chime().into()).clone()
    }
}

fn play(app: &AppHandle, pcm: Arc<[f32]>) {
    let s: General = app.state::<Settings>().read(Partition::General);
    crate::sys::audio::play(pcm, s.sound.sounds_device.map(|d| d.id));
}

/// The output devices for Settings → Sound, and the name of the one Windows uses by default.
#[derive(Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SoundDevices {
    pub devices: Vec<OutputDevice>,
    pub default_name: String,
}

pub fn devices() -> SoundDevices {
    let (devices, default_name) = crate::sys::audio::outputs();
    SoundDevices { devices, default_name }
}

/// Whether `path` is a sound file one of your highlights names (V106-33: mentions are
/// highlights now). Only those are read or played.
pub fn named(app: &AppHandle, path: &str) -> bool {
    let h: Highlights = app.state::<Settings>().read(Partition::Highlights);
    h.alerts().any(|a| a.sound_file == path) && is_sound_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_wait_for_the_background_unless_asked() {
        assert!(may_play(false, false));
        assert!(!may_play(true, false));
        assert!(may_play(false, true));
        assert!(may_play(true, true));
    }

    #[test]
    fn samples_are_whole_stereo_frames() {
        let one: Vec<u8> = [0.5f32, -0.25].iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(samples(&one).unwrap(), vec![0.5, -0.25]);
        assert!(samples(&one[..4]).is_err());
        assert!(samples(&vec![0; MAX_SECONDS * RATE as usize * 8 + 8]).is_err());
    }
}
