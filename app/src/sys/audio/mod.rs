//! V1051-01 Windows' audio for the app: its own sounds play here (`out`), and every audio
//! session of the app and its WebView2 processes gets the app's name, icon and grouping
//! (`sessions`), so Windows' volume mixer lists "cyex deck" (and "cyex deck - streams").
//! Why it's built this way: docs/changes/v1.051-plans/batch-1.md.

mod out;
mod sessions;

pub use out::{play, prime};
pub use sessions::Watcher;

use cyex_settings::schema::OutputDevice;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{
    DEVICE_STATE_ACTIVE, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, STGM_READ,
};
use windows::core::{GUID, PWSTR};

/// `PKEY_Device_FriendlyName` ("Speakers (Realtek(R) Audio)"), without another `windows` feature.
const FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 14,
};

/// COM joined on this thread (MTA) until dropped. Bind it first in a thread so every COM
/// object of that thread goes before it.
struct Com(bool);

/// Joins COM on this thread (MTA). Threads here are our own, so it's never another model.
fn com() -> Com {
    // SAFETY: initializing COM for the calling thread.
    Com(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: balances the successful CoInitializeEx above, on the same thread.
            unsafe { CoUninitialize() };
        }
    }
}

fn enumerator() -> windows::core::Result<IMMDeviceEnumerator> {
    // SAFETY: creating the system's device enumerator.
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
}

/// A string Windows allocated for us, freed.
fn take(p: PWSTR) -> String {
    // SAFETY: `p` came from a Core Audio getter and is freed once, here.
    unsafe {
        let s = p.to_string().unwrap_or_default();
        CoTaskMemFree(Some(p.0 as *const _));
        s
    }
}

/// The output devices that are plugged in and on, with Windows' names, and the name of the
/// one Windows plays on by default ("" when there is none).
pub fn outputs() -> (Vec<OutputDevice>, String) {
    // COM on a short thread of our own, not on a command's runtime thread.
    std::thread::spawn(|| {
        let _com = com();
        let en = enumerator().ok();
        let list = en.as_ref().and_then(|e| list(e).ok()).unwrap_or_default();
        let default = en.as_ref().and_then(|e| default_name(e).ok()).unwrap_or_default();
        (list, default)
    })
    .join()
    .unwrap_or_default()
}

fn name_of(dev: &IMMDevice) -> windows::core::Result<String> {
    // SAFETY: plain Core Audio read on a live device.
    Ok(unsafe { dev.OpenPropertyStore(STGM_READ)?.GetValue(&FRIENDLY_NAME)?.to_string() })
}

fn default_name(en: &IMMDeviceEnumerator) -> windows::core::Result<String> {
    // SAFETY: plain Core Audio read.
    name_of(&unsafe { en.GetDefaultAudioEndpoint(eRender, eConsole)? })
}

fn list(en: &IMMDeviceEnumerator) -> windows::core::Result<Vec<OutputDevice>> {
    // SAFETY: plain Core Audio reads on objects owned by this function.
    unsafe {
        let all = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let mut out = Vec::new();
        for i in 0..all.GetCount()? {
            let dev = all.Item(i)?;
            let id = take(dev.GetId()?);
            out.push(OutputDevice { id, name: name_of(&dev)? });
        }
        Ok(out)
    }
}
