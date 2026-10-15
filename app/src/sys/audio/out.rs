//! V1051-01 Plays the app's own sounds through WASAPI (shared mode) in the app's process, in
//! one session of its own, so the mixer lists them as the app and Windows remembers that
//! entry's volume, mute and device. 48 kHz stereo float; Windows converts to the device.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use windows::Win32::Media::Audio::{
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
    DEVICE_STATE_ACTIVE, IAudioClient, IAudioRenderClient, WAVEFORMATEX, eConsole, eRender,
};
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::core::{GUID, HSTRING};

use super::{com, enumerator};
use crate::sound::RATE;

/// The app's own session. Fixed, so Windows keeps its volume, mute and device between runs.
const SESSION: GUID = GUID::from_u128(0x5f0c2a91_3b7e_4d6a_9c11_2e8f4b7a6d01);
/// More sounds at once than this are dropped (a busy chat full of mentions).
const MAX_AT_ONCE: usize = 4;
/// WASAPI's buffer: 100 ms, in 100 ns units.
const BUFFER: i64 = 1_000_000;
static PLAYING: AtomicUsize = AtomicUsize::new(0);

/// Plays interleaved stereo at [`RATE`] on `device` (Windows' endpoint id), or on the
/// default output when it's None or gone. Returns at once.
pub fn play(pcm: Arc<[f32]>, device: Option<String>) {
    if PLAYING.fetch_add(1, Ordering::SeqCst) >= MAX_AT_ONCE {
        PLAYING.fetch_sub(1, Ordering::SeqCst);
        return;
    }
    let spawned = std::thread::Builder::new().name("sound".into()).spawn(move || {
        if let Err(e) = run(&pcm, device.as_deref()) {
            eprintln!("sound: {e}");
        }
        PLAYING.fetch_sub(1, Ordering::SeqCst);
    });
    if spawned.is_err() {
        PLAYING.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Opens the session once, so the mixer lists the app before its first sound.
pub fn prime() {
    let _ = std::thread::Builder::new().name("sound".into()).spawn(|| {
        if let Err(e) = open(None) {
            eprintln!("sound: {e}");
        }
    });
}

fn open(device: Option<&str>) -> windows::core::Result<IAudioClient> {
    com();
    let en = enumerator()?;
    // SAFETY: plain Core Audio calls on objects owned by this function.
    unsafe {
        let picked = device
            .and_then(|id| en.GetDevice(&HSTRING::from(id)).ok())
            .filter(|d| d.GetState().is_ok_and(|s| s == DEVICE_STATE_ACTIVE));
        let dev = match picked {
            Some(d) => d,
            // Windows' per-app device for the app applies to its default.
            None => en.GetDefaultAudioEndpoint(eRender, eConsole)?,
        };
        let client: IAudioClient = dev.Activate(CLSCTX_ALL, None)?;
        // WAVE_FORMAT_IEEE_FLOAT (3), stereo.
        let format = WAVEFORMATEX {
            wFormatTag: 3,
            nChannels: 2,
            nSamplesPerSec: RATE,
            nAvgBytesPerSec: RATE * 8,
            nBlockAlign: 8,
            wBitsPerSample: 32,
            cbSize: 0,
        };
        client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
            BUFFER,
            0,
            &format,
            Some(&SESSION),
        )?;
        Ok(client)
    }
}

fn run(pcm: &[f32], device: Option<&str>) -> windows::core::Result<()> {
    let client = open(device)?;
    // SAFETY: the render buffer Windows hands out is filled within its size and released.
    unsafe {
        let render: IAudioRenderClient = client.GetService()?;
        let size = client.GetBufferSize()? as usize;
        let frames = pcm.len() / 2;
        let mut at = 0;
        client.Start()?;
        while at < frames {
            let free = size - client.GetCurrentPadding()? as usize;
            let n = free.min(frames - at);
            if n > 0 {
                let buf = render.GetBuffer(n as u32)? as *mut f32;
                std::ptr::copy_nonoverlapping(pcm[at * 2..].as_ptr(), buf, n * 2);
                render.ReleaseBuffer(n as u32, 0)?;
                at += n;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        while client.GetCurrentPadding()? > 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
        client.Stop()?;
    }
    Ok(())
}
