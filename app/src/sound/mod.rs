//! V1051-01 The app's own sounds (U6 chime, mention and highlight files V104-01/V105-17,
//! live alerts, whispers) play here, natively, so Windows' volume mixer lists them as
//! "cyex deck", apart from streams ("cyex deck - streams", WebView2's own process), or with
//! them as one entry. The page decodes a sound file the first time (Chromium reads every
//! type in `SOUND_TYPES`; Windows can't read ogg and opus). Settings → Sound.

mod cache;
mod chime;
mod look;

pub use look::Look;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use cyex_settings::Partition;
use cyex_settings::schema::{General, Highlights, Mentions, OutputDevice, is_sound_file};
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
const SINK_READY_EVENT: &str = "sound:sink-ready";

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
    /// The speaker choice last given to WebView2, so other General changes leave it be.
    speakers: Mutex<Option<bool>>,
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
        speakers: Mutex::default(),
    });
    crate::sys::audio::prime();
}

/// General changed: the mixer's look and the streams' speaker permission follow.
pub fn on_settings(app: &AppHandle) {
    let s: General = app.state::<Settings>().read(Partition::General);
    let (streams, sounds) = look::looks(s.sound.separate_streams);
    if let Some(sounds_state) = app.try_state::<Sounds>() {
        sounds_state.watcher.restyle(streams, sounds);
    }
    allow_speakers(app);
}

/// The streams' speaker choice (Q2): allowed while a streams device is set. Needs the
/// main window's page; checked again on every General change and page load, set only
/// when it changes (remembered once WebView2 took it, so a failed try is retried).
pub fn allow_speakers(app: &AppHandle) {
    let s: General = app.state::<Settings>().read(Partition::General);
    let allow = s.sound.streams_device.is_some();
    let Some(main) = app.get_webview_window(MAIN) else {
        return;
    };
    if let Some(sounds) = app.try_state::<Sounds>()
        && *sounds.speakers.lock().unwrap_or_else(|p| p.into_inner()) == Some(allow)
    {
        return;
    }
    let handle = app.clone();
    crate::sys::allow_speaker_choice(&main, allow, move || {
        if let Some(sounds) = handle.try_state::<Sounds>() {
            *sounds.speakers.lock().unwrap_or_else(|p| p.into_inner()) = Some(allow);
        }
        let _ = handle.emit(SINK_READY_EVENT, ());
    });
}

/// U6, V104-01 A mention, highlight, live alert or whisper sound: its file, or the chime.
/// Streamer mode can mute them (U4).
pub fn alert(app: &AppHandle, file: Option<&str>) {
    if crate::streamer::hides(app, |s| s.mute_sounds) {
        return;
    }
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

/// The output devices for Settings → Sound.
pub fn devices() -> Vec<OutputDevice> {
    crate::sys::audio::outputs()
}

/// Whether `path` is a sound file one of your mentions or highlights names. Only those are
/// read or played.
pub fn named(app: &AppHandle, path: &str) -> bool {
    let settings = app.state::<Settings>();
    let m: Mentions = settings.read(Partition::Mentions);
    let h: Highlights = settings.read(Partition::Highlights);
    let named = std::iter::once(&m.me.alert)
        .chain(m.list.iter().map(|r| &r.alert))
        .chain(m.global_list.iter().map(|r| &r.alert))
        .any(|a| a.sound_file == path)
        || h.phrases
            .iter()
            .map(|r| &r.alert)
            .chain(h.users.iter().map(|r| &r.alert))
            .chain(h.badges.iter().map(|r| &r.alert))
            .chain(h.apps.iter().map(|r| &r.alert))
            .chain(
                [
                    &h.first_message,
                    &h.redeemed,
                    &h.subs,
                    &h.announcements,
                    &h.watch_streaks,
                    &h.own,
                ]
                .map(|b| &b.alert),
            )
            .any(|a| a.sound_file == path);
    named && is_sound_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_whole_stereo_frames() {
        let one: Vec<u8> = [0.5f32, -0.25].iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(samples(&one).unwrap(), vec![0.5, -0.25]);
        assert!(samples(&one[..4]).is_err());
        assert!(samples(&vec![0; MAX_SECONDS * RATE as usize * 8 + 8]).is_err());
    }
}
