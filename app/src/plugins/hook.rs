//! S5 Lines on their way into a chat go past plugins with a Message callback. Rooms wait
//! for the ones that may change lines (ChangeChat), all at once and for a few
//! milliseconds at most; plugins that only read get the line without anyone waiting.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use cyex_chat::model::{ChatMessage, Flag, plain};
use cyex_chat::{LineChange, LineHook};
use cyex_plugins::{Event, Flags, LINE, Line, ReplyTo, Sender};
use tauri::AppHandle;

/// A plugin's own time limit, plus the trip to its thread and back.
const WAIT: Duration = LINE.saturating_add(Duration::from_millis(5));
/// The most a page of history waits for plugins, however long it is.
const BATCH_WAIT: Duration = Duration::from_millis(250);

pub struct PluginHook {
    pub app: AppHandle,
}

impl LineHook for PluginHook {
    fn check(&self, channel: &str, m: &ChatMessage) -> Option<LineChange> {
        self.ask(channel, &[m], WAIT).pop().flatten()
    }

    /// History: every line goes to the plugins at once and the room waits once, a little
    /// longer for a long page; lines not answered by then stay as they are.
    fn check_all(&self, channel: &str, ms: &[&ChatMessage]) -> Vec<Option<LineChange>> {
        let per_line = Duration::from_millis(1).saturating_mul(u32::try_from(ms.len()).unwrap_or(u32::MAX));
        self.ask(channel, ms, WAIT.saturating_add(per_line).min(BATCH_WAIT))
    }
}

impl PluginHook {
    fn ask(&self, channel: &str, ms: &[&ChatMessage], wait: Duration) -> Vec<Option<LineChange>> {
        let mut out = vec![None; ms.len()];
        if !super::wants_lines(&self.app) {
            return out;
        }
        let plugins: Vec<_> = super::running(&self.app)
            .into_iter()
            .filter(|p| p.has(Event::Message))
            .collect();
        if plugins.is_empty() {
            return out;
        }
        let lines: Vec<Line> = ms.iter().map(|m| to_line(channel, m)).collect();
        let (tx, rx) = mpsc::channel();
        let mut waiting = 0;
        for (i, p) in plugins.iter().enumerate() {
            // One that's behind gets the lines to read, but nobody waits on it.
            let waits = p.granted.change_chat && !p.behind();
            for (n, line) in lines.iter().enumerate() {
                if waits {
                    let tx = tx.clone();
                    if p.line(line.clone(), Some(Box::new(move |c| drop(tx.send((n, i, c)))))) {
                        waiting += 1;
                    }
                } else {
                    p.line(line.clone(), None);
                }
            }
        }
        drop(tx);
        let until = Instant::now() + wait;
        let mut changes = Vec::new();
        while waiting > 0 {
            match rx.recv_timeout(until.saturating_duration_since(Instant::now())) {
                Ok((n, i, c)) => {
                    waiting -= 1;
                    changes.extend(c.map(|c| (n, i, c)));
                }
                Err(_) => break,
            }
        }
        // Plugins in folder order; a later one's text or color wins.
        changes.sort_by_key(|(n, i, _)| (*n, *i));
        for (n, _, c) in changes {
            let o = out[n].get_or_insert_with(LineChange::default);
            o.hide |= c.hide;
            if c.text.is_some() {
                o.text = c.text;
            }
            if c.highlight.is_some() {
                o.highlight = c.highlight;
            }
        }
        for o in &mut out {
            if o.as_ref().is_some_and(|c| *c == LineChange::default()) {
                *o = None;
            }
        }
        out
    }
}

/// A chat line as plugins see it.
pub fn to_line(channel: &str, m: &ChatMessage) -> Line {
    let mut text = plain(&m.body);
    if text.is_empty()
        && let Some(n) = &m.notice
    {
        text.clone_from(&n.text);
    }
    Line {
        id: m.id.clone(),
        channel: channel.to_owned(),
        time: m.ts,
        kind: if m.meta.plugin.is_some() {
            "plugin"
        } else if m.notice.is_some() {
            "notice"
        } else if m.has(Flag::Action) {
            "action"
        } else {
            "chat"
        },
        text,
        sender: m.sender.as_ref().map(|s| Sender {
            id: s.id.clone(),
            login: s.login.clone(),
            name: s.display.clone(),
            color: Some(s.color.dark.clone()).filter(|c| !c.is_empty()),
        }),
        badges: m
            .sender
            .as_ref()
            .map(|s| s.badges.iter().map(|b| b.set.clone()).collect())
            .unwrap_or_default(),
        notice: m.notice.as_ref().and_then(|n| {
            serde_json::to_value(n.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
        }),
        reply: m.reply.as_ref().map(|r| ReplyTo {
            login: r.parent_login.clone(),
            text: r.parent_body.clone(),
        }),
        reward: m.reward.as_ref().and_then(|r| r.title.clone()),
        flags: Flags {
            own: m.has(Flag::Own),
            history: m.has(Flag::History),
            mention: m.has(Flag::Mention),
            first: m.has(Flag::FirstMessage),
            deleted: m.has(Flag::Deleted),
        },
        plugin: m.meta.plugin.as_ref().map(|p| p.id.clone()),
    }
}
