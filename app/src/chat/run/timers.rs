//! S3 `/timer`: runs a command later, in the chat it was set in. `-r` keeps it through a
//! restart (saved in the commands settings); missed ones are dropped on start.

use std::sync::Mutex;

use cyex_chat::commands::chain::{Timer, TimerStart, When, parse_timer};
use cyex_chat::model::NoticeKind;
use cyex_settings::Partition;
use cyex_settings::schema::{Commands, SavedTimer};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};

use super::super::{Identity, now_ms};
use super::{Place, line, local};
use crate::settings::Settings;

const MAX_TIMERS: usize = 50;

struct Running {
    id: String,
    place: Place,
    at_ms: i64,
    command: String,
    keep: bool,
    task: JoinHandle<()>,
}

#[derive(Default)]
struct Timers {
    running: Mutex<Vec<Running>>,
    next: Mutex<u32>,
}

pub fn restore(app: &AppHandle) {
    app.manage(Timers::default());
    let saved = app
        .state::<Settings>()
        .read::<Commands>(Partition::Commands)
        .timers;
    let now = now_ms();
    for t in saved {
        if t.at_ms <= now {
            continue;
        }
        let identity = t.account.clone().map_or(Identity::Anon, Identity::Account);
        let place = Place {
            view: None,
            key: (identity, t.channel.clone()),
            automation: t.automation,
            ui: false,
            plugins: Vec::new(),
        };
        schedule(app, place, t.id, t.at_ms, t.command, true);
    }
    save(app);
}

pub fn command(app: &AppHandle, place: &Place, args: &str) -> Result<(), String> {
    match parse_timer(args)? {
        Timer::List => {
            let now = now_ms();
            let lines: Vec<String> = timers(app)
                .running
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .iter()
                .map(|t| format!("{} in {}: {}", t.id, short((t.at_ms - now).max(0) / 1000), t.command))
                .collect();
            let text = if lines.is_empty() {
                "No timers running.".to_owned()
            } else {
                lines.join(" · ")
            };
            local(app, &place.key, NoticeKind::Info, &text);
            Ok(())
        }
        Timer::Stop(id) => {
            let stopped = stop(app, &id);
            if stopped == 0 {
                return Err(format!("No timer called {id}."));
            }
            local(
                app,
                &place.key,
                NoticeKind::Info,
                &if stopped == 1 { "Stopped 1 timer.".to_owned() } else { format!("Stopped {stopped} timers.") },
            );
            Ok(())
        }
        Timer::Start(start) => begin(app, place, start),
    }
}

fn timers(app: &AppHandle) -> &Timers {
    app.state::<Timers>().inner()
}

fn begin(app: &AppHandle, place: &Place, s: TimerStart) -> Result<(), String> {
    let at = at_ms(s.when);
    let id = match s.id {
        Some(id) if id.ends_with('*') => {
            let mut n = timers(app).next.lock().unwrap_or_else(|p| p.into_inner());
            *n += 1;
            format!("{}{}", id.trim_end_matches('*'), *n)
        }
        Some(id) => id,
        None => {
            let mut n = timers(app).next.lock().unwrap_or_else(|p| p.into_inner());
            *n += 1;
            format!("t{}", *n)
        }
    };
    {
        let running = timers(app).running.lock().unwrap_or_else(|p| p.into_inner());
        if running.iter().any(|t| t.id == id) && !s.overwrite {
            return Err(format!("A timer called {id} is running. -o replaces it."));
        }
        if running.len() >= MAX_TIMERS {
            return Err("Too many timers running.".into());
        }
    }
    stop(app, &id);
    schedule(app, place.clone(), id.clone(), at, s.command, s.keep);
    if s.keep {
        save(app);
    }
    if !s.silent {
        let secs = (at - now_ms()).max(0) / 1000;
        local(
            app,
            &place.key,
            NoticeKind::Info,
            &format!("Timer {id} runs in {}.", short(secs)),
        );
    }
    Ok(())
}

fn schedule(app: &AppHandle, place: Place, id: String, at: i64, command: String, keep: bool) {
    // Fires later, when no window is waiting for an answer.
    let place = Place { ui: false, ..place };
    let (app2, place2, id2, cmd2) = (app.clone(), place.clone(), id.clone(), command.clone());
    let task = tauri::async_runtime::spawn(async move {
        let wait = (at - now_ms()).max(0) as u64;
        tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        finish(&app2, &id2);
        if let Err(e) = line(&app2, &place2, &cmd2, None, true, 0).await {
            local(
                &app2,
                &place2.key,
                NoticeKind::Error,
                &format!("Timer {id2}: {e}"),
            );
        }
    });
    timers(app)
        .running
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(Running {
            id,
            place,
            at_ms: at,
            command,
            keep,
            task,
        });
}

/// A timer ran: forget it.
fn finish(app: &AppHandle, id: &str) {
    let kept = {
        let mut running = timers(app).running.lock().unwrap_or_else(|p| p.into_inner());
        let kept = running.iter().any(|t| t.id == id && t.keep);
        running.retain(|t| t.id != id);
        kept
    };
    if kept {
        save(app);
    }
}

/// Stops `id`, or all with `*`. Returns how many.
fn stop(app: &AppHandle, id: &str) -> usize {
    let (n, kept) = {
        let mut running = timers(app).running.lock().unwrap_or_else(|p| p.into_inner());
        let hit = |t: &Running| id == "*" || t.id == id;
        let mut n = 0;
        let mut kept = false;
        for t in running.iter().filter(|t| hit(t)) {
            t.task.abort();
            n += 1;
            kept |= t.keep;
        }
        running.retain(|t| !hit(t));
        (n, kept)
    };
    if kept {
        save(app);
    }
    n
}

/// Writes the `-r` timers to settings.
fn save(app: &AppHandle) {
    let saved: Vec<SavedTimer> = timers(app)
        .running
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .filter(|t| t.keep)
        .map(|t| SavedTimer {
            id: t.id.clone(),
            channel: t.place.key.1.clone(),
            account: t.place.account().map(str::to_owned),
            at_ms: t.at_ms,
            command: t.command.clone(),
            automation: t.place.automation,
        })
        .collect();
    let _ = app
        .state::<Settings>()
        .update::<Commands>(app, Partition::Commands, |c| {
            let changed = c.timers != saved;
            c.timers = saved;
            changed
        });
}

/// When a `When` comes next, in unix ms, local time.
fn at_ms(when: When) -> i64 {
    let now = now_ms();
    let offset = i64::from(time::UtcOffset::current_local_offset().map_or(0, |o| o.whole_seconds())) * 1000;
    let day = 86_400_000;
    match when {
        When::In { secs } => now + secs as i64 * 1000,
        When::At { day_secs } => {
            let local_now = now + offset;
            let midnight = local_now - local_now.rem_euclid(day);
            let mut at = midnight + i64::from(day_secs) * 1000;
            if at <= local_now {
                at += day;
            }
            at - offset
        }
        When::On { days, day_secs } => days * day + i64::from(day_secs) * 1000 - offset,
    }
}

/// `45s`, `5m`, `1m 30s`, `2h 5m`: the two biggest parts that aren't zero, like the UI's durations.
fn short(secs: i64) -> String {
    let s = secs.max(0);
    let parts: Vec<String> = [(s / 3600, 'h'), (s % 3600 / 60, 'm'), (s % 60, 's')]
        .into_iter()
        .skip_while(|(n, _)| *n == 0)
        .take(2)
        .filter(|(n, _)| *n > 0)
        .map(|(n, u)| format!("{n}{u}"))
        .collect();
    if parts.is_empty() { "0s".to_owned() } else { parts.join(" ") }
}

#[cfg(test)]
mod tests {
    use super::short;

    #[test]
    fn short_drops_zero_parts() {
        assert_eq!(short(0), "0s");
        assert_eq!(short(45), "45s");
        assert_eq!(short(300), "5m");
        assert_eq!(short(90), "1m 30s");
        assert_eq!(short(3600), "1h");
        assert_eq!(short(7505), "2h 5m");
    }
}
