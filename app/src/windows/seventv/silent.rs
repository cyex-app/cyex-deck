//! V108-7S The hidden try in the Twitch pages profile: Twitch approves on its own when 7TV
//! was authorized before. A Twitch login page, a consent page that stays, or 10 s without
//! 7TV's callback shows the window (once). Times are since the window opened.

use std::time::Duration;

use tauri::Url;

const HIDDEN_FOR: Duration = Duration::from_secs(10);
/// A consent page still there this long after it loaded waits for the user.
const CONSENT_AFTER: Duration = Duration::from_secs(2);

pub(super) enum Seen<'a> {
    Went(&'a Url),
    Loaded(&'a Url),
    CalledBack,
}

#[derive(Debug, Default)]
pub(super) struct Silent {
    shown: bool,
    called_back: bool,
    /// When Twitch's authorize page finished loading, while it's still there.
    consent_at: Option<Duration>,
}

fn is_login(url: &Url) -> bool {
    match url.host_str() {
        Some("passport.twitch.tv") => true,
        Some("www.twitch.tv" | "twitch.tv") => url.path().starts_with("/login"),
        _ => false,
    }
}

fn is_authorize(url: &Url) -> bool {
    url.host_str() == Some("id.twitch.tv") && url.path().starts_with("/oauth2/authorize")
}

impl Silent {
    /// `true`: show the window now.
    pub(super) fn seen(&mut self, e: Seen<'_>, t: Duration) -> bool {
        match e {
            Seen::CalledBack => self.called_back = true,
            Seen::Went(u) if is_login(u) => return self.show(),
            Seen::Went(u) if !is_authorize(u) => self.consent_at = None,
            Seen::Loaded(u) if is_authorize(u) => {
                self.consent_at.get_or_insert(t);
            }
            _ => {}
        }
        self.tick(t)
    }

    pub(super) fn tick(&mut self, t: Duration) -> bool {
        let waiting = self.consent_at.is_some_and(|at| t >= at + CONSENT_AFTER);
        let slow = !self.called_back && t >= HIDDEN_FOR;
        if waiting || slow { self.show() } else { false }
    }

    fn show(&mut self) -> bool {
        !std::mem::replace(&mut self.shown, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    const AUTH: &str = "https://id.twitch.tv/oauth2/authorize?client_id=a";
    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn twitch_approving_on_its_own_stays_hidden() {
        let mut s = Silent::default();
        assert!(!s.seen(Seen::Went(&u("https://api.7tv.app/v4/auth/login?platform=twitch")), MS(0)));
        assert!(!s.seen(Seen::Went(&u(AUTH)), MS(300)));
        assert!(!s.seen(Seen::Went(&u("https://7tv.app/login/callback?code=x")), MS(900)));
        assert!(!s.seen(Seen::CalledBack, MS(900)));
        // After the callback only the token poll is left.
        assert!(!s.tick(MS(12_000)));
    }

    #[test]
    fn a_login_page_shows_it_at_once_and_once() {
        let mut s = Silent::default();
        assert!(s.seen(Seen::Went(&u("https://www.twitch.tv/login?client_id=a")), MS(500)));
        assert!(!s.seen(Seen::Went(&u("https://passport.twitch.tv/x")), MS(600)));
        assert!(!s.tick(MS(20_000)));
        assert!(Silent::default().seen(Seen::Went(&u("https://passport.twitch.tv/login")), MS(100)));
        assert!(!Silent::default().seen(Seen::Went(&u("https://www.twitch.tv/directory")), MS(100)));
    }

    #[test]
    fn a_consent_page_that_stays_shows_it() {
        let mut s = Silent::default();
        assert!(!s.seen(Seen::Loaded(&u(AUTH)), MS(1_000)));
        assert!(!s.tick(MS(2_500)));
        assert!(s.tick(MS(3_000)));
        // Left before the grace: stays hidden.
        let mut q = Silent::default();
        assert!(!q.seen(Seen::Loaded(&u(AUTH)), MS(1_000)));
        assert!(!q.seen(Seen::Went(&u("https://7tv.app/login/callback?code=x")), MS(1_500)));
        assert!(!q.seen(Seen::CalledBack, MS(1_500)));
        assert!(!q.tick(MS(4_000)));
    }

    #[test]
    fn nothing_for_10_seconds_shows_it() {
        let mut s = Silent::default();
        assert!(!s.tick(MS(9_900)));
        assert!(s.tick(MS(10_000)));
        assert!(!s.tick(MS(11_000)));
    }
}
