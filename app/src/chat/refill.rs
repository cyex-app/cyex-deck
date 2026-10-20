//! V107-M1 Rooms no view shows keep a small buffer (`cyex_chat::BACKGROUND`). Shown again,
//! a room shows what it kept at once, then gets its older lines back from the history
//! service in one `History` event; the view draws them a piece per frame (historyDrip.ts).
//! Only rooms that actually dropped lines refill, and only up to the user's history lines
//! above what they kept. Hidden again, a room shrinks after `SHRINK_AFTER`.

use std::time::Duration;

use cyex_chat::model::RoomEvent;
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use cyex_twitch::pace::Lane;
use tauri::{AppHandle, Manager};

use super::{Chat, RoomKey, account_of, now_ms, queue};

/// A chat hidden this long drops to the small buffer: coming back sooner costs nothing.
const SHRINK_AFTER: Duration = Duration::from_secs(45);
/// The history service keeps no more.
const SERVICE_MAX: usize = 800;

/// The chat is shown again after it was in the background.
pub fn start(app: &AppHandle, key: RoomKey) {
    let c = app.state::<crate::settings::Settings>().read::<ChatSettings>(Partition::Chat);
    if !c.history {
        return;
    }
    let history_lines = c.history_lines as usize;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::emotes::ready(&app, &key.1, Duration::from_secs(4)).await;
        let want = {
            let chat = app.state::<Chat>();
            let mut inner = chat.lock();
            match inner.rooms.get_mut(&key) {
                Some(e) if wanted(e.views.is_empty(), &e.room) => {
                    // Taken now, so a second show meanwhile doesn't fetch again.
                    e.room.set_trimmed(false);
                    e.room.refill_want(history_lines).min(SERVICE_MAX)
                }
                _ => return,
            }
        };
        let http = app.state::<crate::auth::Auth>().http.clone();
        let fetched = cyex_twitch::recent::recent_messages(&http, &key.1, want as u32, Lane::Front).await;
        let emotes = crate::emotes::room(&app, account_of(&key.0), &key.1);
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        let Some(entry) = inner.rooms.get_mut(&key) else { return };
        let lines = match fetched {
            Ok(lines) if !entry.views.is_empty() => lines,
            other => {
                if let Err(e) = other {
                    eprintln!("chat: no refill for {}: {}", key.1, e.describe("recent-messages"));
                }
                // Not filled: the next show tries again.
                entry.room.set_trimmed(true);
                return;
            }
        };
        let messages = entry.room.backfill(&lines, now_ms(), &emotes);
        if !messages.is_empty() {
            queue(entry, [RoomEvent::History { messages }], &chat.wake);
        }
    });
}

/// A view opens on a room: refill it now? Not while its first history is still out
/// (`loaded`), since the refill would race it; that load starts the refill itself.
pub fn on_open(fresh: bool, loaded: bool, room: &cyex_chat::Room) -> bool {
    !fresh && loaded && room.trimmed()
}

/// The first history is in. Cut short for the background (`cut`), it counts as trimmed;
/// shown meanwhile, the open skipped the refill, so it's due now.
pub fn first_history_in(room: &mut cyex_chat::Room, cut: bool, shown: bool) -> bool {
    if cut {
        room.set_trimmed(true);
    }
    shown && room.trimmed()
}

/// Whether a room is worth a refill: shown, lines dropped, room for more.
fn wanted(hidden: bool, room: &cyex_chat::Room) -> bool {
    !hidden && room.trimmed() && room.missing() > 0
}

/// The last view closed but a tab still holds the room: shrink it unless it's shown again
/// before `SHRINK_AFTER`. `shown`: the room's open count when its last view closed.
pub fn later(app: &AppHandle, key: RoomKey, shown: u64) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(SHRINK_AFTER).await;
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        if let Some(entry) = inner.rooms.get_mut(&key)
            && entry.views.is_empty()
            && entry.shown == shown
        {
            entry.room.set_background(true);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_chat::Room;
    use cyex_chat::irc::Line;
    use cyex_chat::tokenize::NoEmotes;

    fn room(lines: usize) -> Room {
        let mut r = Room::new("chan", None);
        r.set_keep(1000);
        for i in 0..lines {
            let raw = format!("@id=m{i};tmi-sent-ts={i};user-id=7 :a!a@a PRIVMSG #chan :hi");
            r.handle_line(&Line::parse(&raw).unwrap(), i as i64, &NoEmotes);
        }
        r
    }

    #[test]
    fn v107_m1_only_a_shown_trimmed_room_refills() {
        let mut never = room(100);
        never.set_background(true);
        never.set_background(false);
        assert!(!wanted(false, &never), "never trimmed: no fetch");
        let mut cut = room(300);
        cut.set_background(true);
        cut.set_background(false);
        assert!(wanted(false, &cut));
        assert!(!wanted(true, &cut), "hidden again");
        assert_eq!(cut.refill_want(500).min(SERVICE_MAX), 650, "history lines above the 150 kept");
    }

    #[test]
    fn v107_m1_a_refill_waits_for_the_first_history() {
        // Joined in the background; a view opens while the 100-line history is out.
        let mut r = room(0);
        r.set_background(true);
        assert!(!on_open(false, false, &r), "loading: no refill yet");
        r.set_background(false);
        // The history lands, cut short for the background: the refill starts now.
        assert!(first_history_in(&mut r, true, true));
        // Still hidden when it landed: the next open refills.
        let mut r = room(0);
        r.set_background(true);
        assert!(!first_history_in(&mut r, true, false));
        assert!(on_open(false, true, &r));
        // Shown with its full history and never trimmed: nothing to fetch.
        let mut r = room(0);
        assert!(!first_history_in(&mut r, false, true));
        assert!(!on_open(false, true, &r));
        assert!(!on_open(true, true, &r), "a fresh room loads its history itself");
    }
}
