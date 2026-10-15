//! S7, S10 Plugins' own lines: built into chat lines and put in every open room of the
//! channel, one at a time in the order they came, so quick lines keep their order. Changes
//! to lines, taking them back and clearing a chat (S14) take the same queue. A plugin may call these inside its Message callback while
//! a room waits on it, so nothing here runs on the plugin's thread.

use std::sync::Arc;

use cyex_chat::LineChange;
use cyex_chat::build::Colors;
use cyex_chat::model::{
    Act, ChatMessage, Fragment, Highlight, HighlightKind, Meta, PluginTag, SpanColor, SpanStyle,
};
use cyex_chat::tokenize::{Input, detect_link, tokenize};
use cyex_plugins::{Change, Level, LineLink, NewLine, Piece, PieceColor, PieceStyle};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use super::{PluginState, logs};

pub enum Op {
    Add(NewLine),
    Replace(NewLine),
    Change(String, Change),
    /// S14 Takes back one of its own lines.
    Remove(String),
    /// S14 Empties the chat on screen.
    Clear,
}

pub struct Queued {
    pub plugin: String,
    pub login: String,
    pub op: Op,
}

/// The queue every plugin's lines go through.
pub fn start(app: &AppHandle) -> mpsc::UnboundedSender<Queued> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Queued>();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(q) = rx.recv().await {
            apply(&app, q);
        }
    });
    tx
}

pub fn queue(app: &AppHandle, plugin: &str, login: &str, op: Op) {
    if let Some(state) = app.try_state::<PluginState>() {
        let _ = state.lines.send(Queued {
            plugin: plugin.to_owned(),
            login: login.to_owned(),
            op,
        });
    }
}

fn apply(app: &AppHandle, q: Queued) {
    let found = match q.op {
        Op::Add(line) => {
            let m = build(app, &q.plugin, &q.login, line);
            crate::chat::plugin_line(app, &q.login, Arc::new(m))
        }
        Op::Replace(line) => {
            let m = build(app, &q.plugin, &q.login, line);
            crate::chat::plugin_replace(app, &q.login, Arc::new(m))
        }
        Op::Change(id, c) => {
            let change = LineChange {
                hide: c.hide,
                text: c.text,
                highlight: c.highlight,
            };
            crate::chat::plugin_change(app, &q.login, &id, &change)
        }
        Op::Remove(id) => crate::chat::plugin_remove(app, &q.login, &id),
        Op::Clear => {
            let text = format!("{} cleared this chat on your screen.", super::name_of(app, &q.plugin));
            crate::chat::plugin_clear(app, &q.login, &text)
        }
    };
    if !found {
        logs::push(app, &q.plugin, Level::Warning, &format!("The chat of {} isn’t open, or the line is gone.", q.login));
    }
}

/// A plugin's line as a chat line: no sender, the plugin's mark, its pieces as fragments.
pub fn build(app: &AppHandle, plugin: &str, login: &str, line: NewLine) -> ChatMessage {
    let parse = line.pieces.iter().any(|p| matches!(p, Piece::Text { parse: true, .. }));
    let emotes = parse.then(|| crate::emotes::room(app, None, login));
    let mut colors = Colors::default();
    let mut body = Vec::new();
    for piece in line.pieces {
        match piece {
            Piece::Text { text, parse: true, .. } => {
                let input = Input {
                    text: &text,
                    sender: "",
                    emotes: "",
                    gifs: "",
                    bits: 0,
                };
                match &emotes {
                    Some(e) => body.extend(tokenize(&input, &[], e).body),
                    None => body.push(Fragment::text(text)),
                }
            }
            Piece::Text {
                text,
                color,
                style,
                tip,
                link,
                ..
            } => body.push(Fragment::Span {
                text,
                color: color.map(|c| match c {
                    PieceColor::Link => SpanColor::Link,
                    PieceColor::System => SpanColor::System,
                    PieceColor::Rgb(raw) => SpanColor::Rgb(colors.get(&raw)),
                }),
                style: style.map(|s| match s {
                    PieceStyle::Bold => SpanStyle::Bold,
                    PieceStyle::Italic => SpanStyle::Italic,
                    PieceStyle::Button => SpanStyle::Button,
                }),
                tip,
                act: link.and_then(act),
            }),
            Piece::Mention { text, login } => body.push(Fragment::Mention { text, login }),
            Piece::Break => body.push(Fragment::Break),
            Piece::Image {
                urls,
                size,
                round,
                tip,
                link,
            } => {
                let Some((src, _)) = urls.first().cloned() else { continue };
                let srcset = (urls.len() > 1).then(|| {
                    urls.iter().map(|(u, d)| format!("{u} {d}x")).collect::<Vec<_>>().join(", ")
                });
                body.push(Fragment::Image {
                    src,
                    srcset,
                    alt: tip.clone().unwrap_or_else(|| "Picture".into()),
                    size,
                    round,
                    tip,
                    act: link.and_then(act),
                });
            }
        }
    }
    ChatMessage {
        id: line.id,
        ts: line.time.unwrap_or_else(crate::chat::now_ms),
        sender: None,
        body,
        notice: None,
        reply: None,
        reward: None,
        source: None,
        flags: Vec::new(),
        meta: Meta {
            highlight: line.highlight.map(|c| Highlight {
                kind: HighlightKind::Rule,
                color: (!c.is_empty()).then_some(c),
            }),
            plugin: Some(PluginTag {
                id: plugin.to_owned(),
                name: super::name_of(app, plugin),
            }),
            ..Meta::default()
        },
    }
}

/// A plugin link as what a click does. A `Url` link shows its real host (X2).
fn act(link: LineLink) -> Option<Act> {
    Some(match link {
        LineLink::Url(url) => {
            let l = detect_link(&url)?;
            Act::Url {
                url: l.url,
                host: l.host,
                target: l.target,
            }
        }
        LineLink::User(login) => Act::User { login },
        LineLink::Run(text) => Act::Run { text },
        LineLink::Jump(id) => Act::Jump { id },
        LineLink::Copy(text) => Act::Copy { text },
        LineLink::Insert(text) => Act::Insert { text },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_show_their_real_host() {
        let Some(Act::Url { host, .. }) = act(LineLink::Url("https://www.xn--80ak6aa92e.com/a".into())) else {
            panic!()
        };
        assert_eq!(host, "xn--80ak6aa92e.com");
        assert_eq!(act(LineLink::Url("not a link".into())), None);
        assert_eq!(act(LineLink::Run("/me hi".into())), Some(Act::Run { text: "/me hi".into() }));
    }
}
