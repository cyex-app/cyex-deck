//! Lines we send to Twitch.

use super::line::escape;

/// Twitch's limit per chat message, in characters.
pub const MAX_MESSAGE_CHARS: usize = 500;

pub const CAPS: &str = "CAP REQ :twitch.tv/tags twitch.tv/commands";

/// `JOIN #a,#b`. Each channel still counts toward the join limit.
pub fn join(channels: &[String]) -> String {
    let list: Vec<String> = channels.iter().map(|c| format!("#{c}")).collect();
    format!("JOIN {}", list.join(","))
}

pub fn part(channel: &str) -> String {
    format!("PART #{channel}")
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outgoing {
    pub text: String,
    pub reply_to: Option<String>,
    /// Echoed back by Twitch so a send can be matched to its result.
    pub nonce: Option<String>,
    pub action: bool,
}

/// `@reply-parent-msg-id=…;client-nonce=… PRIVMSG #chan :text`. Line breaks become spaces.
pub fn privmsg(channel: &str, msg: &Outgoing) -> String {
    let mut tags = Vec::new();
    if let Some(id) = &msg.reply_to {
        tags.push(format!("reply-parent-msg-id={}", escape(id)));
    }
    if let Some(n) = &msg.nonce {
        tags.push(format!("client-nonce={}", escape(n)));
    }
    let text: String = msg
        .text
        .chars()
        .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
        .collect();
    let body = if msg.action {
        format!("\u{1}ACTION {text}\u{1}")
    } else {
        text
    };
    let head = if tags.is_empty() {
        String::new()
    } else {
        format!("@{} ", tags.join(";"))
    };
    format!("{head}PRIVMSG #{channel} :{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privmsg_lines() {
        let plain = Outgoing {
            text: "hi\nthere".into(),
            ..Default::default()
        };
        assert_eq!(privmsg("c", &plain), "PRIVMSG #c :hi there");
        let reply = Outgoing {
            text: "yo".into(),
            reply_to: Some("abc".into()),
            nonce: Some("n1".into()),
            action: true,
        };
        assert_eq!(
            privmsg("c", &reply),
            "@reply-parent-msg-id=abc;client-nonce=n1 PRIVMSG #c :\u{1}ACTION yo\u{1}"
        );
        assert_eq!(join(&["a".into(), "b".into()]), "JOIN #a,#b");
    }
}
