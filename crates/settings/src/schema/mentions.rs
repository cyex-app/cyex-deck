//! V104-01, V104-02 Mentions: your name, your own words or patterns, and global mentions
//! from logs.cyex.app's firehose. Each one has its own options. Patterns that don't
//! compile are kept, so the editor can show why, and simply never match.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;
use super::highlights::Highlights;

const MAX_MENTIONS: usize = 100;
const MAX_PATTERN_CHARS: usize = 500;
const MAX_PATH_CHARS: usize = 1000;
/// Sound files the window can play.
pub const SOUND_TYPES: [&str; 6] = ["mp3", "wav", "ogg", "opus", "flac", "m4a"];

/// What a match does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct MentionAlert {
    /// Copy the message into the Mentions tab.
    pub show: bool,
    /// Flash the taskbar while the app isn't in front.
    pub flash: bool,
    /// A Windows pop-up.
    pub notify: bool,
    pub sound: bool,
    /// A sound file played instead of the chime. Empty: the chime.
    pub sound_file: String,
    /// `#rrggbbaa`. Empty: the mention color.
    pub color: String,
}

impl Default for MentionAlert {
    fn default() -> Self {
        Self {
            show: true,
            flash: false,
            notify: false,
            sound: false,
            sound_file: String::new(),
            color: String::new(),
        }
    }
}

impl MentionAlert {
    fn sanitize(&mut self) {
        let hex = self.color.strip_prefix('#').unwrap_or("");
        if !(matches!(hex.len(), 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())) {
            self.color.clear();
        }
        self.color.make_ascii_lowercase();
        self.sound_file = self.sound_file.trim().to_owned();
        if !is_sound_file(&self.sound_file) || self.sound_file.chars().count() > MAX_PATH_CHARS {
            self.sound_file.clear();
        }
    }
}

/// A file name the sound option takes: one of [`SOUND_TYPES`].
pub fn is_sound_file(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, ext)| SOUND_TYPES.iter().any(|t| t.eq_ignore_ascii_case(ext)))
}

/// Your own name (and replies to you), found by the chat itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct YourName {
    pub enabled: bool,
    #[serde(flatten)]
    #[ts(flatten)]
    pub alert: MentionAlert,
}

impl Default for YourName {
    fn default() -> Self {
        Self {
            enabled: true,
            alert: MentionAlert::default(),
        }
    }
}

/// Which regex syntax a pattern is written in. One engine runs them all (V104-01).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum RegexFlavor {
    #[default]
    Rust,
    JavaScript,
    Python,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct MentionRule {
    pub enabled: bool,
    /// Words, or a regex when `regex` is on.
    pub pattern: String,
    /// Not inside a longer word.
    pub whole_word: bool,
    pub case_sensitive: bool,
    /// Look-alike letters count (Cyrillic а for a, 𝐟𝐚𝐧𝐜𝐲 text).
    pub lookalikes: bool,
    pub regex: bool,
    pub flavor: RegexFlavor,
    #[serde(flatten)]
    #[ts(flatten)]
    pub alert: MentionAlert,
}

impl Default for MentionRule {
    fn default() -> Self {
        Self {
            enabled: true,
            pattern: String::new(),
            whole_word: true,
            case_sensitive: false,
            lookalikes: false,
            regex: false,
            flavor: RegexFlavor::Rust,
            alert: MentionAlert::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Mentions {
    pub me: YourName,
    /// Words that ping you in any open chat.
    pub list: Vec<MentionRule>,
    /// V104-02 Global mentions from logs.cyex.app, matched here.
    pub global: bool,
    pub global_list: Vec<MentionRule>,
}

impl Schema for Mentions {
    fn sanitize(&mut self) {
        self.me.alert.sanitize();
        for list in [&mut self.list, &mut self.global_list] {
            list.truncate(MAX_MENTIONS);
            for m in list.iter_mut() {
                if m.pattern.chars().count() > MAX_PATTERN_CHARS {
                    m.pattern = m.pattern.chars().take(MAX_PATTERN_CHARS).collect();
                }
                m.alert.sanitize();
            }
        }
    }
}

impl Mentions {
    /// Global mentions to look for: on, with something to find.
    pub fn global_active(&self) -> bool {
        self.global
            && self
                .global_list
                .iter()
                .any(|m| m.enabled && !m.pattern.trim().is_empty())
    }

    /// Before v1.04 your name was a highlight: its switch, color, sound and flash carry
    /// over once, and a feed it went to counts as the Mentions tab.
    pub fn from_highlights(h: &Highlights) -> Self {
        let old = &h.mention;
        Self {
            me: YourName {
                enabled: old.enabled,
                alert: MentionAlert {
                    show: true,
                    flash: old.alert.flash,
                    notify: false,
                    sound: old.alert.sound,
                    sound_file: String::new(),
                    color: old.alert.color.clone(),
                },
            },
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_cleans_mentions() {
        let mut m = Mentions {
            list: vec![MentionRule {
                pattern: "x".repeat(600),
                alert: MentionAlert {
                    color: "#ABCDEF".into(),
                    sound_file: r"C:\a\ping.exe".into(),
                    ..MentionAlert::default()
                },
                ..MentionRule::default()
            }],
            ..Mentions::default()
        };
        m.me.alert.sound_file = r" C:\Sounds\Ping.WAV ".into();
        m.sanitize();
        assert_eq!(m.list[0].pattern.len(), 500);
        assert_eq!(m.list[0].alert.color, "#abcdef");
        assert_eq!(m.list[0].alert.sound_file, "", "not a sound file");
        assert_eq!(m.me.alert.sound_file, r"C:\Sounds\Ping.WAV");
    }

    #[test]
    fn global_needs_a_pattern() {
        let mut m = Mentions {
            global: true,
            ..Mentions::default()
        };
        assert!(!m.global_active());
        m.global_list.push(MentionRule {
            pattern: "  ".into(),
            ..MentionRule::default()
        });
        assert!(!m.global_active());
        m.global_list[0].pattern = "cyex".into();
        assert!(m.global_active());
        m.global = false;
        assert!(!m.global_active());
    }

    #[test]
    fn your_name_carries_over() {
        let mut h = Highlights::default();
        h.mention.alert.sound = true;
        h.mention.alert.color = "#ff000080".into();
        let m = Mentions::from_highlights(&h);
        assert!(m.me.enabled && m.me.alert.sound && m.me.alert.show);
        assert_eq!(m.me.alert.color, "#ff000080");
    }
}
