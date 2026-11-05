//! V109-T01 A custom theme's tie to a link on cyex.app: shared by you (the edit key is in
//! the Credential Manager, never here) or added from someone's link, with the last few
//! versions you had kept here so you can go back.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::appearance::CustomTheme;

/// Earlier versions kept per theme added from a link.
pub const KEEP_COPIES: usize = 5;
/// Link ids: 8 characters, no look-alikes (0/o, 1/l/i). Same as the server.
pub const ID_LEN: usize = 8;
const ID_ALPHABET: &[u8] = b"23456789abcdefghjkmnpqrstuvwxyz";
const DATE_CHARS: usize = 10;

pub fn valid_link_id(s: &str) -> bool {
    s.len() == ID_LEN && s.bytes().all(|b| ID_ALPHABET.contains(&b))
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct ThemeLink {
    /// Shared by you: the link id. Empty when not shared.
    pub shared_id: String,
    pub shared_version: u32,
    /// The theme as last uploaded (`CustomTheme::fingerprint`).
    pub shared_hash: String,
    /// The wallpaper file last uploaded, so an update only sends a new picture.
    pub shared_wallpaper: String,
    /// Worked out on every save: the theme changed since the last upload.
    pub shared_changed: bool,
    /// Added from a link: its id. Empty for your own themes and theme codes.
    pub source_id: String,
    pub source_version: u32,
    /// `yyyy/mm/dd` of the version you have.
    pub source_date: String,
    /// The theme as that version came (`CustomTheme::fingerprint`).
    pub source_hash: String,
    /// Newest version the last check saw.
    pub latest_version: u32,
    /// "Ignore this version" for this one.
    pub ignored_version: u32,
    /// The author took the link down; your copy stays.
    pub removed: bool,
    /// Worked out on every save: you changed it since that version came.
    pub edited: bool,
    /// Versions you had before, newest first, up to `KEEP_COPIES`.
    pub history: Vec<ThemeCopy>,
}

/// One earlier version of a theme added from a link.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct ThemeCopy {
    pub version: u32,
    /// `yyyy/mm/dd`
    pub date: String,
    /// It had your own changes.
    pub edited: bool,
    /// Without id or link.
    pub theme: CustomTheme,
}

/// What the update check heard about one link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upstream {
    Version(u32),
    Removed,
}

/// A version fetched from cyex.app.
#[derive(Clone, Debug, PartialEq)]
pub struct Fetched {
    pub version: u32,
    /// `yyyy/mm/dd`
    pub date: String,
    pub theme: CustomTheme,
}

impl ThemeLink {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The version to offer, if a newer one is out and wasn't ignored.
    pub fn update_available(&self) -> Option<u32> {
        let newer = !self.source_id.is_empty()
            && !self.removed
            && self.latest_version > self.source_version
            && self.latest_version != self.ignored_version;
        newer.then_some(self.latest_version)
    }

    pub(super) fn sanitize(&mut self) {
        if !valid_link_id(&self.shared_id) {
            self.shared_id.clear();
            self.shared_version = 0;
            self.shared_hash.clear();
            self.shared_wallpaper.clear();
        }
        if !valid_link_id(&self.source_id) {
            self.source_id.clear();
            self.source_version = 0;
            self.source_date.clear();
            self.source_hash.clear();
            self.latest_version = 0;
            self.ignored_version = 0;
            self.removed = false;
            self.history.clear();
        }
        self.source_date = clean_date(&self.source_date);
        self.history.truncate(KEEP_COPIES);
        for c in &mut self.history {
            c.date = clean_date(&c.date);
            c.theme = c.theme.content();
            c.theme.sanitize();
        }
    }
}

fn clean_date(d: &str) -> String {
    let ok = d.len() == DATE_CHARS
        && d.bytes()
            .enumerate()
            .all(|(i, b)| if i == 4 || i == 7 { b == b'/' } else { b.is_ascii_digit() });
    if ok { d.to_owned() } else { String::new() }
}

/// `2026-11-05T10:00:00Z` → `2026/11/05`; anything else → empty.
pub fn date_of(rfc3339: &str) -> String {
    clean_date(&rfc3339.get(..DATE_CHARS).unwrap_or_default().replace('-', "/"))
}

/// FNV-1a, 64 bit: enough to tell two versions of one theme apart.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

impl CustomTheme {
    /// The look alone: no id, no link.
    pub fn content(&self) -> CustomTheme {
        CustomTheme {
            id: String::new(),
            link: ThemeLink::default(),
            ..self.clone()
        }
    }

    /// What goes to cyex.app or into a code: the look, without a wallpaper file's name.
    pub fn shareable(&self) -> CustomTheme {
        let mut t = self.content();
        if t.wallpaper.is_file() {
            t.wallpaper.source.clear();
        }
        t
    }

    /// Tells two versions of the look apart (id and link left out).
    pub fn fingerprint(&self) -> String {
        let json = serde_json::to_vec(&self.content()).expect("theme serializes");
        format!("{:016x}", fnv(&json))
    }

    /// The derived flags, after everything else is clean.
    pub(super) fn update_link_flags(&mut self) {
        let print = (!self.link.shared_id.is_empty() || !self.link.source_id.is_empty()).then(|| self.fingerprint());
        let l = &mut self.link;
        l.shared_changed = !l.shared_id.is_empty() && print.as_deref() != Some(l.shared_hash.as_str());
        l.edited = !l.source_id.is_empty() && print.as_deref() != Some(l.source_hash.as_str());
    }

    fn set_content(&mut self, c: &CustomTheme) {
        self.name.clone_from(&c.name);
        self.base = c.base;
        self.colors.clone_from(&c.colors);
        self.accent.clone_from(&c.accent);
        self.wallpaper = c.wallpaper.clone();
    }

    /// Uploaded (first share or an update): remembers what went up (`sent`, the theme as it
    /// was read before the upload; it may have changed since).
    pub fn record_share(&mut self, id: &str, version: u32, sent: &CustomTheme) {
        let hash = sent.fingerprint();
        let file = if sent.wallpaper.is_file() { sent.wallpaper.source.clone() } else { String::new() };
        let changed = hash != self.fingerprint();
        let l = &mut self.link;
        l.shared_id = id.to_owned();
        l.shared_version = version;
        l.shared_hash = hash;
        l.shared_wallpaper = file;
        l.shared_changed = changed;
    }

    /// Sharing stopped: the theme stays, the link goes.
    pub fn forget_share(&mut self) {
        let l = &mut self.link;
        l.shared_id.clear();
        l.shared_version = 0;
        l.shared_hash.clear();
        l.shared_wallpaper.clear();
        l.shared_changed = false;
    }

    /// Added from link `id`: remembers which version it is.
    pub fn link_source(&mut self, id: &str, version: u32, date: &str) {
        let hash = self.fingerprint();
        let l = &mut self.link;
        l.source_id = id.to_owned();
        l.source_version = version;
        l.source_date = clean_date(date);
        l.source_hash = hash;
        l.latest_version = version;
        l.edited = false;
    }

    /// The version you have now goes to the history (newest first, `KEEP_COPIES` at most).
    fn keep_current(&mut self) {
        let copy = ThemeCopy {
            version: self.link.source_version,
            date: self.link.source_date.clone(),
            edited: self.fingerprint() != self.link.source_hash,
            theme: self.content(),
        };
        self.link.history.insert(0, copy);
        self.link.history.truncate(KEEP_COPIES);
    }

    /// Update: keeps the version you have, then takes the fetched one.
    pub fn apply_fetched(&mut self, f: &Fetched) {
        self.keep_current();
        self.set_content(&f.theme);
        self.sanitize();
        let hash = self.fingerprint();
        let l = &mut self.link;
        l.source_version = f.version;
        l.source_date = clean_date(&f.date);
        l.source_hash = hash;
        l.latest_version = l.latest_version.max(f.version);
        l.removed = false;
        l.edited = false;
    }

    /// Go back to `history[index]`; what you had takes its place in the list. False when
    /// there's no such copy.
    pub fn restore_copy(&mut self, index: usize) -> bool {
        if index >= self.link.history.len() {
            return false;
        }
        let copy = self.link.history.remove(index);
        self.keep_current();
        self.set_content(&copy.theme);
        self.sanitize();
        let hash = self.fingerprint();
        let l = &mut self.link;
        l.source_version = copy.version;
        l.source_date = copy.date;
        // A copy with your changes stays "yours": updating asks first again.
        l.source_hash = if copy.edited { String::new() } else { hash };
        self.update_link_flags();
        true
    }

    /// What the update check heard. True when something changed.
    pub fn note_upstream(&mut self, up: Upstream) -> bool {
        let l = &mut self.link;
        if l.source_id.is_empty() {
            return false;
        }
        let before = (l.latest_version, l.removed);
        match up {
            Upstream::Version(v) => {
                l.latest_version = l.latest_version.max(v);
                l.removed = false;
            }
            Upstream::Removed => l.removed = true,
        }
        before != (l.latest_version, l.removed)
    }

    /// "Ignore this version": no notice until a newer one.
    pub fn ignore_latest(&mut self) -> bool {
        let l = &mut self.link;
        let changed = l.ignored_version != l.latest_version;
        l.ignored_version = l.latest_version;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Schema;
    use crate::schema::{Appearance, builtin_themes};

    fn fetched(version: u32, bg: &str) -> Fetched {
        let mut theme = builtin_themes()[0].content();
        theme.colors.insert("bg".into(), bg.into());
        Fetched {
            version,
            date: format!("2026/11/{version:02}"),
            theme,
        }
    }

    fn imported() -> CustomTheme {
        let f = fetched(1, "#101010");
        let mut t = f.theme.clone();
        t.id = "mine1".into();
        t.link_source("k3f9x2ab", f.version, &f.date);
        t
    }

    #[test]
    fn ids() {
        assert!(valid_link_id("k3f9x2ab"));
        for bad in ["k3f9x2a", "k3f9x2abc", "K3F9X2AB", "k3f9x2a0", "k3f9x2al", "k3f9x2a1", "k3f9-2ab"] {
            assert!(!valid_link_id(bad), "{bad}");
        }
    }

    #[test]
    fn dates() {
        assert_eq!(date_of("2026-11-05T10:00:00Z"), "2026/11/05");
        assert_eq!(date_of("yesterday"), "");
        assert_eq!(date_of(""), "");
    }

    #[test]
    fn fingerprint_ignores_id_and_link() {
        let a = imported();
        let mut b = a.clone();
        b.id = "other".into();
        b.link = ThemeLink::default();
        assert_eq!(a.fingerprint(), b.fingerprint());
        b.accent = "#ff0000".into();
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn updates_keep_the_last_five() {
        let mut t = imported();
        for v in 2..=8 {
            assert!(t.note_upstream(Upstream::Version(v)));
            assert_eq!(t.link.update_available(), Some(v));
            t.apply_fetched(&fetched(v, &format!("#0000{v:02}")));
            assert_eq!(t.link.update_available(), None);
        }
        assert_eq!(t.link.source_version, 8);
        assert_eq!(t.colors["bg"], "#000008");
        let versions: Vec<u32> = t.link.history.iter().map(|c| c.version).collect();
        assert_eq!(versions, [7, 6, 5, 4, 3]);
        assert_eq!(t.link.history[0].date, "2026/11/07");
        assert!(t.link.history.iter().all(|c| c.theme.id.is_empty() && c.theme.link.is_empty()));
        assert_eq!(t.id, "mine1", "the theme keeps its id");
    }

    #[test]
    fn going_back_keeps_what_you_had() {
        let mut t = imported();
        t.apply_fetched(&fetched(2, "#000002"));
        t.note_upstream(Upstream::Version(3));
        assert!(t.restore_copy(0));
        assert_eq!(t.link.source_version, 1);
        assert_eq!(t.colors["bg"], "#101010");
        assert_eq!(t.link.history[0].version, 2);
        assert!(!t.link.edited);
        assert_eq!(t.link.update_available(), Some(3), "going back doesn't stop notices");
        assert!(t.ignore_latest());
        assert_eq!(t.link.update_available(), None);
        t.note_upstream(Upstream::Version(4));
        assert_eq!(t.link.update_available(), Some(4), "ignoring is per version");
        assert!(!t.restore_copy(9));
    }

    #[test]
    fn own_changes_show_and_are_kept_as_a_copy() {
        let mut t = imported();
        t.colors.insert("text".into(), "#eeeeee".into());
        t.sanitize();
        assert!(t.link.edited);
        t.apply_fetched(&fetched(2, "#000002"));
        assert!(!t.link.edited);
        assert!(t.link.history[0].edited);
        assert!(t.restore_copy(0));
        assert!(t.link.edited, "your changes stay yours after going back");
    }

    #[test]
    fn removed_upstream() {
        let mut t = imported();
        assert!(t.note_upstream(Upstream::Removed));
        assert!(!t.note_upstream(Upstream::Removed));
        t.note_upstream(Upstream::Version(5));
        assert!(!t.link.removed);
        let mut own = builtin_themes()[0].content();
        assert!(!own.note_upstream(Upstream::Removed), "not from a link: nothing to note");
    }

    #[test]
    fn shared_changed_follows_edits() {
        let mut t = builtin_themes()[1].content();
        t.id = "a1".into();
        t.wallpaper.source = "abc123.png".into();
        let sent = t.clone();
        t.record_share("k3f9x2ab", 1, &sent);
        t.sanitize();
        assert!(!t.link.shared_changed);
        assert_eq!(t.link.shared_wallpaper, "abc123.png");
        t.accent = "#00ff00".into();
        t.sanitize();
        assert!(t.link.shared_changed);
        let sent = t.clone();
        t.record_share("k3f9x2ab", 2, &sent);
        assert!(!t.link.shared_changed);
        // Changed while the upload ran: still counts as changed.
        let sent = t.clone();
        t.accent = "#0000ff".into();
        t.record_share("k3f9x2ab", 3, &sent);
        assert!(t.link.shared_changed);
        t.forget_share();
        assert!(t.link.is_empty());
    }

    #[test]
    fn junk_links_are_dropped() {
        let mut t = imported();
        t.link.shared_id = "../x".into();
        t.link.source_date = "nope".into();
        t.link.history = (0..9).map(|v| ThemeCopy { version: v, ..ThemeCopy::default() }).collect();
        t.sanitize();
        assert!(t.link.shared_id.is_empty());
        assert_eq!(t.link.source_date, "");
        assert_eq!(t.link.history.len(), KEEP_COPIES);
        t.link.source_id = "bad".into();
        t.sanitize();
        assert!(t.link.is_empty());
    }

    #[test]
    fn old_settings_load_and_new_round_trip() {
        // Saved before V109-T01: no link anywhere.
        let old = r##"{"themes":[{"id":"abc","name":"Mine","base":"dark","colors":{},"accent":"#145aff","wallpaper":{"source":"","area":"window","dim":70,"blur":0,"fit":"cover"}}],"themeId":"abc"}"##;
        let mut a: Appearance = serde_json::from_str(old).unwrap();
        a.sanitize();
        assert!(a.themes[0].link.is_empty());
        assert!(!serde_json::to_string(&a).unwrap().contains("link"), "an empty link isn't written");

        a.themes[0] = imported();
        a.themes[0].apply_fetched(&fetched(2, "#000002"));
        a.theme_id = a.themes[0].id.clone();
        a.sanitize();
        let json = serde_json::to_string(&a).unwrap();
        let mut back: Appearance = serde_json::from_str(&json).unwrap();
        back.sanitize();
        assert_eq!(back, a);
        assert_eq!(back.themes[0].link.history.len(), 1);
    }

    #[test]
    fn shareable_drops_id_link_and_file() {
        let mut t = imported();
        t.wallpaper.source = "abc123.png".into();
        let s = t.shareable();
        assert!(s.id.is_empty() && s.link.is_empty() && s.wallpaper.source.is_empty());
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("link") && !json.contains("k3f9x2ab"));
    }
}
