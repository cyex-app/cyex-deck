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

pub struct PluginHook {
    pub app: AppHandle,
}

impl LineHook for PluginHook {
    fn check(&self, channel: &str, m: &ChatMessage) -> Option<LineChange> {
        if !super::wants_lines(&self.app) {
            return None;
        }
        let plugins: Vec<_> = super::running(&self.app)
            .into_iter()
            .filter(|p| p.has(Event::Message))
            .collect();
        if plugins.is_empty() {
            return None;
        }
        let line = to_line(channel, m);
        let (tx, rx) = mpsc::channel();
        let mut waiting = 0;
        for (i, p) in plugins.iter().enumerate() {
            // One that's behind gets the line to read, but nobody waits on it.
            if p.granted.change_chat && !p.behind() {
                let tx = tx.clone();
                if p.line(line.clone(), Some(Box::new(move |c| drop(tx.send((i, c)))))) {
                    waiting += 1;
                }
            } else {
                p.line(line.clone(), None);
            }
        }
        drop(tx);
        let until = Instant::now() + WAIT;
        let mut changes = Vec::new();
        while waiting > 0 {
            match rx.recv_timeout(until.saturating_duration_since(Instant::now())) {
                Ok((i, c)) => {
                    waiting -= 1;
                    changes.extend(c.map(|c| (i, c)));
                }
                Err(_) => break,
            }
        }
        // Plugins in folder order; a later one's text or color wins.
        changes.sort_by_key(|(i, _)| *i);
        let mut out = LineChange::default();
        for (_, c) in changes {
            out.hide |= c.hide;
            if c.text.is_some() {
                out.text = c.text;
            }
            if c.highlight.is_some() {
                out.highlight = c.highlight;
            }
        }
        (out != LineChange::default()).then_some(out)
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
        kind: if m.notice.is_some() {
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
    }
}
