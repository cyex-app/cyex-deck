//! M1 `/nuke`: preview everyone who matched in a window, then ban or time them out. The
//! match logic is in `cyex_moderation::nuke`; here we read the room buffer for lines and
//! run the hits through the mass-action pacer, skipping mods and the broadcaster.

use cyex_moderation::nuke::{Hit, Nuke, Punish};
use cyex_moderation::pacer::Pacer;
use cyex_core::number::grouped;
use cyex_chat::model::{ModRole, plain};
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use super::massban;

/// A nuke's preview: who it would hit and how.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NukePreview {
    pub hits: Vec<NukeHit>,
    /// Seconds looked back.
    pub window_secs: u64,
    /// Timeout length in seconds, or 0 for a ban.
    pub timeout_secs: u32,
    /// Mods and the broadcaster left out of the count.
    pub skipped: u32,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NukeHit {
    pub user_id: String,
    pub login: String,
    pub display: String,
    pub sample: String,
}

/// People in the channel we never nuke: the broadcaster, mods and VIPs.
fn protected(app: &AppHandle, login: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for m in crate::chat::messages_of(app, login) {
        if let Some(s) = &m.sender
            && (ModRole::of_badges(&s.badges).is_some() || s.badges.iter().any(|b| b.set == "vip"))
        {
            out.insert(s.id.clone());
        }
    }
    out
}

/// Builds a preview from a `/nuke` argument string.
pub fn preview(app: &AppHandle, account: &str, login: &str, args: &str) -> Result<NukePreview, String> {
    super::actor(app, account, login)?;
    let nuke = Nuke::parse(args)?;
    let now = crate::chat::now_ms();
    let protected = protected(app, login);
    let (hits, skipped) = scan(app, login, &nuke, now, &protected);
    Ok(NukePreview {
        hits: hits.into_iter().map(into_hit).collect(),
        window_secs: nuke.window_secs,
        timeout_secs: match nuke.punish {
            Punish::Ban => 0,
            Punish::Timeout(s) => s,
        },
        skipped,
    })
}

fn into_hit(h: Hit) -> NukeHit {
    NukeHit {
        user_id: h.user_id,
        login: h.login,
        display: h.display,
        sample: h.sample,
    }
}

/// Scans the room buffer, returning hits and how many protected users were skipped.
fn scan(app: &AppHandle, login: &str, nuke: &Nuke, now: i64, protected: &std::collections::HashSet<String>) -> (Vec<Hit>, u32) {
    let msgs = crate::chat::messages_of(app, login);
    let texts: Vec<(String, String, String, String, i64)> = msgs
        .iter()
        .filter_map(|m| {
            let s = m.sender.as_ref()?;
            Some((s.id.clone(), s.login.clone(), s.display.clone(), plain(&m.body), m.ts))
        })
        .collect();
    let lines: Vec<cyex_moderation::nuke::Line<'_>> = texts
        .iter()
        .map(|(id, login, display, text, ts)| cyex_moderation::nuke::Line {
            user_id: id,
            login,
            display,
            text,
            ts_ms: *ts,
        })
        .collect();
    let all = nuke.scan(&lines, now);
    let before = all.len();
    let hits: Vec<Hit> = all.into_iter().filter(|h| !protected.contains(&h.user_id)).collect();
    (hits.clone(), (before - hits.len()) as u32)
}

/// Runs a nuke: bans or times out every hit, paced like a mass action, and reports.
pub async fn run(app: &AppHandle, account: &str, login: &str, args: &str) -> Result<(), String> {
    let actor = super::actor(app, account, login)?;
    let nuke = Nuke::parse(args)?;
    let now = crate::chat::now_ms();
    let protected = protected(app, login);
    let (hits, _) = scan(app, login, &nuke, now, &protected);
    if hits.is_empty() {
        crate::chat::notice_line(app, login, cyex_chat::model::NoticeKind::Info, "Nuke: nothing matched.");
        return Ok(());
    }
    let logins: Vec<String> = hits.iter().map(|h| h.login.clone()).collect();
    // V107-R21 Refused before the "hitting" line: one mass action per account, 5.000 at most.
    if massban::running(app, account) {
        return Err(cyex_moderation::mass_jobs::BUSY.into());
    }
    cyex_moderation::mass::check_size(logins.len())?;
    let secs = match nuke.punish {
        Punish::Ban => None,
        Punish::Timeout(s) => Some(s),
    };
    let what = match nuke.punish {
        Punish::Ban => "Banned".to_owned(),
        Punish::Timeout(s) => format!("Timed out ({}s)", s),
    };
    crate::chat::notice_line(
        app,
        login,
        cyex_chat::model::NoticeKind::Info,
        &format!("Nuke: hitting {} user(s)…", grouped(logins.len() as u64)),
    );
    let order = massban::Order { secs, reason: "nuke", unban: false };
    let done = massban::run(app, &actor, &logins, order, &mut Pacer::default()).await?;
    crate::chat::notice_line(
        app,
        login,
        cyex_chat::model::NoticeKind::Info,
        &format!("Nuke done: {what} {} of {}.", done, logins.len()),
    );
    Ok(())
}

/// Used by the command router to know when a line is a `/nuke`.
pub fn is_nuke(name: &str) -> bool {
    name == "nuke"
}
