//! Backup streams for ad breaks (1.2): the same stream asked for as another player
//! type, which Twitch often serves without ads, or through the user's proxy (P10). The
//! order follows the maintained vaft forks (2026): the backup that worked last first,
//! ones that had ads this break last, and 360p autoplay, the most reliable but the worst
//! picture, at the very end.

use std::time::{Duration, Instant};

use cyex_twitch::gql::Player;

use crate::fetch::Route;
use crate::hls::master::Variant;

/// The stream itself. vaft and its forks ask as the pop-out player, which gets fewer ads.
pub const MAIN: Player<'static> = Player {
    player_type: "popout",
    platform: "web",
};

const TYPES: [Player<'static>; 3] = [
    Player {
        player_type: "site",
        platform: "web",
    },
    Player {
        player_type: "embed",
        platform: "web",
    },
    Player {
        player_type: "mobile_web",
        platform: "web",
    },
];

const AUTOPLAY: Player<'static> = Player {
    player_type: "autoplay",
    platform: "android",
};

/// After every backup had ads: wait this long before asking again.
const PAUSE: Duration = Duration::from_secs(6);

#[derive(Debug)]
pub struct Backups {
    pub routes: Vec<Route>,
    /// Each backup's variants, kept across polls and breaks.
    pub masters: Vec<Option<Vec<Variant>>>,
    /// The backup in use right now.
    pub current: Option<usize>,
    last_good: Option<usize>,
    had_ads: Vec<usize>,
    pause_until: Option<Instant>,
}

impl Backups {
    /// The proxy, when there is one, goes before 360p autoplay.
    pub fn new(proxy: bool) -> Self {
        let mut routes: Vec<Route> = TYPES.iter().map(|p| Route::Backup(*p)).collect();
        if proxy {
            routes.push(Route::Proxy);
        }
        routes.push(Route::Backup(AUTOPLAY));
        Self {
            masters: vec![None; routes.len()],
            routes,
            current: None,
            last_good: None,
            had_ads: Vec::new(),
            pause_until: None,
        }
    }

    /// Backups to try this poll, best bet first. Only the one in use while paused.
    pub fn order(&self, now: Instant) -> Vec<usize> {
        if self.pause_until.is_some_and(|t| now < t) {
            return self.current.into_iter().collect();
        }
        let first = self.current.or(self.last_good);
        let fresh = (0..self.routes.len()).filter(|i| Some(*i) != first && !self.had_ads.contains(i));
        let tried = self.had_ads.iter().copied().filter(|i| Some(*i) != first);
        first.into_iter().chain(fresh).chain(tried).collect()
    }

    pub fn clean(&mut self, i: usize) {
        self.current = Some(i);
        self.last_good = Some(i);
        self.had_ads.retain(|&x| x != i);
    }

    pub fn has_ads(&mut self, i: usize) {
        if self.current == Some(i) {
            self.current = None;
        }
        if !self.had_ads.contains(&i) {
            self.had_ads.push(i);
        }
    }

    /// Nothing clean anywhere: stop asking for a moment.
    pub fn pause(&mut self, now: Instant) {
        self.current = None;
        self.pause_until = Some(now + PAUSE);
    }

    pub fn end_break(&mut self) {
        self.current = None;
        self.had_ads.clear();
        self.pause_until = None;
    }
}

/// The variant of a backup closest to `target`: same resolution and frame rate, else
/// the nearest pixel count (vaft). Audio only matches audio only.
pub fn closest<'a>(variants: &'a [Variant], target: &Variant) -> Option<&'a Variant> {
    let candidates = variants
        .iter()
        .filter(|v| !v.portrait && v.is_audio_only() == target.is_audio_only());
    if target.is_audio_only() {
        return candidates.into_iter().next();
    }
    let candidates: Vec<&Variant> = candidates.collect();
    let same_fps = |v: &Variant| v.frame_rate.map(f32::round) == target.frame_rate.map(f32::round);
    if let Some(exact) = candidates.iter().find(|v| v.resolution == target.resolution && same_fps(v)) {
        return Some(exact);
    }
    candidates.into_iter().min_by_key(|v| v.pixels().abs_diff(target.pixels()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::tests::route_name;

    fn v(id: &str, w: u32, h: u32, fps: f32) -> Variant {
        Variant {
            id: id.into(),
            name: id.into(),
            uri: format!("https://x/{id}.m3u8"),
            bandwidth: 1,
            resolution: (w > 0).then_some((w, h)),
            frame_rate: (fps > 0.0).then_some(fps),
            codecs: String::new(),
            source: false,
            portrait: false,
        }
    }

    #[test]
    fn closest_variant() {
        let backup = [
            v("1080p60", 1920, 1080, 60.0),
            v("720p30", 1280, 720, 30.0),
            v("360p30", 640, 360, 30.0),
            v("audio_only", 0, 0, 0.0),
        ];
        let pick = |t: Variant| closest(&backup, &t).map(|x| x.id.clone());
        assert_eq!(pick(v("720p30", 1280, 720, 30.0)).as_deref(), Some("720p30"));
        assert_eq!(pick(v("720p60", 1280, 720, 60.0)).as_deref(), Some("720p30"));
        assert_eq!(pick(v("936p60", 1664, 936, 60.0)).as_deref(), Some("1080p60"));
        assert_eq!(pick(v("audio_only", 0, 0, 0.0)).as_deref(), Some("audio_only"));
        assert_eq!(closest(&backup[..1], &v("audio_only", 0, 0, 0.0)), None);
    }

    #[test]
    fn the_proxy_comes_before_autoplay() {
        let names = |b: &Backups| b.routes.iter().map(|r| route_name(*r)).collect::<Vec<_>>();
        assert_eq!(names(&Backups::new(false)), ["site", "embed", "mobile_web", "autoplay"]);
        assert_eq!(names(&Backups::new(true)), ["site", "embed", "mobile_web", "proxy", "autoplay"]);
    }

    #[test]
    fn order_follows_what_worked() {
        let now = Instant::now();
        let mut b = Backups::new(false);
        assert_eq!(b.order(now), [0, 1, 2, 3]);
        b.has_ads(0);
        b.has_ads(1);
        b.clean(3);
        assert_eq!(b.order(now), [3, 2, 0, 1]);
        // Next break: the one that worked goes first, nothing is marked yet.
        b.end_break();
        assert_eq!(b.order(now), [3, 0, 1, 2]);
        // Every backup had ads: wait, then try again.
        for i in 0..4 {
            b.has_ads(i);
        }
        b.pause(now);
        assert!(b.order(now).is_empty());
        assert_eq!(b.order(now + PAUSE).len(), 4);
    }
}
