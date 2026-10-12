//! Sender, badges and name colors from IRC tags.

use std::collections::HashMap;

use cyex_core::color::{Rgb, Theme, readable_name};

use crate::irc::{Line, unescape};
use crate::model::{Badge, NameColor, Sender};

/// Twitch's colors for users who never picked one.
const DEFAULT_COLORS: [&str; 15] = [
    "#ff0000", "#0000ff", "#00ff00", "#b22222", "#ff7f50", "#9acd32", "#ff4500", "#2e8b57",
    "#daa520", "#d2691e", "#5f9ea0", "#1e90ff", "#ff69b4", "#8a2be2", "#00ff7f",
];

/// C8 Readable variants per raw color. Few distinct colors appear, so this stays small.
#[derive(Default)]
pub struct Colors(HashMap<String, NameColor>);

impl Colors {
    const MAX: usize = 4096;

    pub fn get(&mut self, raw: &str) -> NameColor {
        if let Some(c) = self.0.get(raw) {
            return c.clone();
        }
        let rgb = Rgb::parse(raw).unwrap_or(Rgb::new(0x80, 0x80, 0x80));
        let color = NameColor {
            dark: readable_name(rgb, Theme::Dark).to_hex(),
            light: readable_name(rgb, Theme::Light).to_hex(),
        };
        if self.0.len() >= Self::MAX {
            self.0.clear();
        }
        self.0.insert(raw.to_owned(), color.clone());
        color
    }
}

/// The color Twitch shows for a user without one, stable per user.
pub fn default_color(user_id: &str, login: &str) -> &'static str {
    let seed = user_id
        .parse::<u64>()
        .unwrap_or_else(|_| login.bytes().map(u64::from).sum());
    DEFAULT_COLORS[(seed % DEFAULT_COLORS.len() as u64) as usize]
}

/// Sender of a PRIVMSG or USERNOTICE. `None` without a user id.
pub fn sender(line: &Line<'_>, colors: &mut Colors) -> Option<Sender> {
    let id = line.tag_nonempty("user-id")?.to_owned();
    let login = line
        .tag_nonempty("login")
        .map(str::to_owned)
        .unwrap_or_else(|| line.nick().to_owned());
    let display = line
        .tag("display-name")
        .map(|d| d.trim().to_owned())
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| login.clone());
    let raw = line
        .tag_nonempty("color")
        .unwrap_or_else(|| default_color(&id, &login));
    Some(Sender {
        color: colors.get(raw),
        badges: badges(line.raw_tag("badges"), line.raw_tag("badge-info")),
        id,
        login,
        display,
    })
}

/// `badges=subscriber/12,premium/1` with `badge-info=subscriber/14`.
pub fn badges(badges: Option<&str>, info: Option<&str>) -> Vec<Badge> {
    let info: Vec<(&str, &str)> = info
        .unwrap_or("")
        .split(',')
        .filter_map(|b| b.split_once('/'))
        .collect();
    badges
        .unwrap_or("")
        .split(',')
        .filter_map(|b| b.split_once('/'))
        .map(|(set, version)| Badge {
            set: set.to_owned(),
            version: version.to_owned(),
            info: info
                .iter()
                .find(|(s, _)| *s == set)
                .map(|(_, v)| unescape(v).into_owned())
                .filter(|v| !v.is_empty()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badges_with_info() {
        let b = badges(
            Some("predictions/blue-1,subscriber/3012,premium/1"),
            Some(r"predictions/Yes\sthey\swill,subscriber/26"),
        );
        assert_eq!(b.len(), 3);
        assert_eq!(b[0].info.as_deref(), Some("Yes they will"));
        assert_eq!((b[1].set.as_str(), b[1].version.as_str()), ("subscriber", "3012"));
        assert_eq!(b[1].info.as_deref(), Some("26"));
        assert_eq!(b[2].info, None);
        assert!(badges(Some(""), None).is_empty());
    }

    #[test]
    fn sender_falls_back_to_login_and_default_color() {
        let line = Line::parse("@user-id=7;display-name= :foo!foo@foo.tmi.twitch.tv PRIVMSG #c :hi").unwrap();
        let s = sender(&line, &mut Colors::default()).unwrap();
        assert_eq!((s.login.as_str(), s.display.as_str()), ("foo", "foo"));
        assert_eq!(default_color("7", "foo"), DEFAULT_COLORS[7]);
        assert!(s.color.dark.starts_with('#'));
        assert!(!s.localized());
    }
}
