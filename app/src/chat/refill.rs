//! V107-M1, V108-C01 Hidden rooms keep their scrollback until the line budget (`park`)
//! shrinks the ones shown longest ago to `cyex_chat::BACKGROUND`. Shown again, such a room
//! shows what it kept at once, then gets its older lines back from the history service in
//! one `History` event; the view draws them a piece per frame (historyDrip.ts). Only rooms
//! that actually dropped lines refill, and only up to the user's history lines above what
//! they kept.

use std::time::Duration;

use cyex_chat::model::RoomEvent;
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use cyex_twitch::pace::Lane;
use tauri::{AppHandle, Manager};

use super::{Chat, RoomKey, account_of, now_ms, queue};

/// The history service keeps no more.
const SERVICE_MAX: usize = 800;

/// The chat is shown again after the line budget shrank it.
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

/// The first history is in. Cut short in a shrunk room (`Room::trimmed`) and shown
/// meanwhile, the open skipped the refill, so it's due now.
pub fn first_history_in(room: &cyex_chat::Room, shown: bool) -> bool {
    shown && room.trimmed()
}

/// Whether a room is worth a refill: shown, lines dropped, room for more.
fn wanted(hidden: bool, room: &cyex_chat::Room) -> bool {
    !hidden && room.trimmed() && room.missing() > 0
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
        // Shrunk by the line budget; a view opens while the first history is out.
        let mut r = room(0);
        r.set_background(true);
        assert!(!on_open(false, false, &r), "loading: no refill yet");
        // The history lands cut short in the shrunk room, then it's shown: refill now.
        let lines: Vec<String> =
            (0..400).map(|i| format!("@id=h{i};tmi-sent-ts={i};user-id=7 :a!a@a PRIVMSG #chan :hi")).collect();
        r.history(&lines, 9_000, &NoEmotes);
        r.set_background(false);
        assert!(first_history_in(&r, true));
        // Still hidden when it landed: the next open refills.
        assert!(!first_history_in(&r, false));
        assert!(on_open(false, true, &r));
        // V108-C01 Hidden at full size, full history, never trimmed: nothing to fetch.
        let r = room(0);
        assert!(!first_history_in(&r, true));
        assert!(!on_open(false, true, &r));
        assert!(!on_open(true, true, &r), "a fresh room loads its history itself");
    }
}
