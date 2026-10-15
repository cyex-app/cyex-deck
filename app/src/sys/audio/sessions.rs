//! V1051-01 Names every audio session of the app and of its WebView2 processes,
//! as they appear. Windows forgets a session's name, so each new one is named again.

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender, channel};

use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{
    DEVICE_STATE, DEVICE_STATE_ACTIVE, EDataFlow, ERole, IAudioSessionControl, IAudioSessionControl2,
    IAudioSessionManager2, IAudioSessionNotification, IAudioSessionNotification_Impl, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, eRender,
};
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::core::{GUID, HSTRING, Interface, PCWSTR, Ref, implement};

use super::{com, enumerator, take};
use crate::sound::Look;

enum Msg {
    /// Something may have a new session: look again.
    Nudge,
    /// The setting changed: streams' look, then the app's own.
    Looks(Look, Look),
}

/// The styler's thread. Lives as long as the app.
pub struct Watcher(Sender<Msg>);

impl Watcher {
    /// `icon`: the exe's own icon (`path,0`).
    pub fn start(icon: String, streams: Look, sounds: Look) -> Self {
        let (tx, rx) = channel();
        let nudges = tx.clone();
        let _ = std::thread::Builder::new()
            .name("audio-sessions".into())
            .spawn(move || {
                if let Err(e) = run(&rx, nudges, &icon, streams, sounds) {
                    eprintln!("audio sessions: {e}");
                }
            });
        Self(tx)
    }

    pub fn restyle(&self, streams: Look, sounds: Look) {
        let _ = self.0.send(Msg::Looks(streams, sounds));
    }
}

#[implement(IAudioSessionNotification)]
struct Created(Sender<Msg>);

impl IAudioSessionNotification_Impl for Created_Impl {
    fn OnSessionCreated(&self, _new: Ref<IAudioSessionControl>) -> windows::core::Result<()> {
        let _ = self.0.send(Msg::Nudge);
        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct Devices(Sender<Msg>);

impl IMMNotificationClient_Impl for Devices_Impl {
    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _state: DEVICE_STATE) -> windows::core::Result<()> {
        let _ = self.0.send(Msg::Nudge);
        Ok(())
    }
    fn OnDeviceAdded(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        let _ = self.0.send(Msg::Nudge);
        Ok(())
    }
    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnDefaultDeviceChanged(
        &self,
        _flow: EDataFlow,
        _role: ERole,
        _id: &PCWSTR,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> windows::core::Result<()> {
        Ok(())
    }
}

fn run(
    rx: &Receiver<Msg>,
    tx: Sender<Msg>,
    icon: &str,
    mut streams: Look,
    mut sounds: Look,
) -> windows::core::Result<()> {
    com();
    let en = enumerator()?;
    let devices: IMMNotificationClient = Devices(tx.clone()).into();
    let created: IAudioSessionNotification = Created(tx).into();
    // SAFETY: registering our own callback object with the enumerator we hold.
    unsafe { en.RegisterEndpointNotificationCallback(&devices)? };
    let mut watched: Vec<(String, IAudioSessionManager2)> = Vec::new();
    let root = std::process::id();
    loop {
        watch_new(&en, &created, &mut watched)?;
        let ours = crate::sys::webview_tree(root);
        for (_, manager) in &watched {
            style(manager, root, &ours, icon, &streams, &sounds);
        }
        // Wait for the next reason, then take every queued one in one pass.
        let Ok(first) = rx.recv() else { return Ok(()) };
        for msg in std::iter::once(first).chain(rx.try_iter()) {
            if let Msg::Looks(s, o) = msg {
                (streams, sounds) = (s, o);
            }
        }
    }
}

/// Registers for new sessions on every output that's on and not watched yet.
fn watch_new(
    en: &IMMDeviceEnumerator,
    created: &IAudioSessionNotification,
    watched: &mut Vec<(String, IAudioSessionManager2)>,
) -> windows::core::Result<()> {
    // SAFETY: plain Core Audio calls on objects we hold.
    unsafe {
        let all = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        for i in 0..all.GetCount()? {
            let dev = all.Item(i)?;
            let id = take(dev.GetId()?);
            if watched.iter().any(|(w, _)| *w == id) {
                continue;
            }
            let manager: IAudioSessionManager2 = dev.Activate(CLSCTX_ALL, None)?;
            // Windows only starts the notifications after one enumeration (documented).
            let _ = manager.GetSessionEnumerator()?;
            manager.RegisterSessionNotification(created)?;
            watched.push((id, manager));
        }
    }
    Ok(())
}

/// Gives the app's sessions their look: the app's own (`root`) the sounds' look, the
/// processes below it (WebView2) the streams' look. A session that went away is skipped.
fn style(
    manager: &IAudioSessionManager2,
    root: u32,
    ours: &HashSet<u32>,
    icon: &str,
    streams: &Look,
    sounds: &Look,
) {
    // SAFETY: plain Core Audio calls; every failure only skips that session.
    unsafe {
        let Ok(list) = manager.GetSessionEnumerator() else {
            return;
        };
        for j in 0..list.GetCount().unwrap_or(0) {
            let Some(s) = list
                .GetSession(j)
                .ok()
                .and_then(|s| s.cast::<IAudioSessionControl2>().ok())
            else {
                continue;
            };
            let pid = s.GetProcessId().unwrap_or(0);
            if !ours.contains(&pid) {
                continue;
            }
            let look = if pid == root { sounds } else { streams };
            let group = GUID::from_u128(look.group);
            let named = s.GetDisplayName().map(take).is_ok_and(|n| n == look.name);
            let grouped = s.GetGroupingParam().is_ok_and(|g| g == group);
            if named && grouped {
                continue;
            }
            let _ = s.SetDisplayName(&HSTRING::from(look.name.as_str()), std::ptr::null());
            let _ = s.SetIconPath(&HSTRING::from(icon), std::ptr::null());
            let _ = s.SetGroupingParam(&group, std::ptr::null());
        }
    }
}
