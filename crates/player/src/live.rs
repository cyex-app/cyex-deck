//! A live stream for one player (1.1) with the ad filter (1.2). hls.js asks for the
//! master, then polls one media playlist; each poll fetches the stream's playlist and,
//! during a break, a backup without ads. What the player gets is stitched together, so it
//! plays on across the switches. Without a clean backup the break is cut out: the
//! picture holds until the stream is back.

use std::collections::HashMap;
use std::time::Instant;

use cyex_twitch::{Error, Result};
use serde::Serialize;
use tokio::sync::Mutex;
use ts_rs::TS;

use crate::ads;
use crate::backup::{self, Backups};
use crate::fetch::{Fetch, Route};
use crate::hls::master::{self, Variant};
use crate::hls::media::{self, Media};
use crate::stitch::Stitcher;

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// List the segments still being made, like Twitch's low latency mode.
    pub low_latency: bool,
    /// 1.2
    pub block_ads: bool,
}

/// Shown on the player during a break.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AdState {
    #[default]
    None,
    /// Playing a backup without ads, maybe in lower quality.
    Backup,
    /// No backup without ads: the break is cut out.
    Waiting,
    /// Blocking is off and an ad plays.
    Showing,
}

pub struct Live<F> {
    fetch: F,
    login: String,
    options: Options,
    state: Mutex<State>,
}

struct State {
    /// The stream's variants, best first.
    variants: Option<Vec<Variant>>,
    /// What each variant's player got so far.
    out: HashMap<usize, Stitcher>,
    backups: Backups,
    ad: AdState,
}

impl<F: Fetch> Live<F> {
    pub fn new(fetch: F, login: &str, options: Options) -> Self {
        let backups = Backups::new(fetch.has_proxy());
        Self {
            fetch,
            login: login.to_owned(),
            options,
            state: Mutex::new(State {
                variants: None,
                out: HashMap::new(),
                backups,
                ad: AdState::None,
            }),
        }
    }

    /// The master hls.js starts from, each variant at `<index>.m3u8`. Errors carry
    /// usher's reason, e.g. `Can not find channel` when offline.
    pub async fn master(&self) -> Result<String> {
        let mut st = self.state.lock().await;
        let variants = self.load(&mut st).await?;
        Ok(master::render(&variants, |i| format!("{i}.m3u8")))
    }

    /// P1 A master with only the audio, for players set to audio only.
    pub async fn audio_master(&self) -> Result<String> {
        let mut st = self.state.lock().await;
        let variants = match &st.variants {
            Some(v) => v.clone(),
            None => self.load(&mut st).await?,
        };
        let (i, v) = variants
            .iter()
            .enumerate()
            .find(|(_, v)| v.is_audio_only())
            .ok_or_else(|| gone("no audio only"))?;
        Ok(master::render(std::slice::from_ref(v), |_| format!("{i}.m3u8")))
    }

    /// The media playlist of variant `index`, and whether an ad break is on.
    pub async fn media(&self, index: usize) -> Result<(String, AdState)> {
        let mut st = self.state.lock().await;
        let variant = match &st.variants {
            Some(v) => v.get(index).cloned(),
            None => self.load(&mut st).await?.get(index).cloned(),
        }
        .ok_or_else(|| gone("no such quality"))?;
        let main = match self.fetch.text(&variant.uri, Route::Main).await {
            Ok(text) => media::parse(&text),
            // The stream restarted or the session ran out: ask usher again, once.
            Err(Error::Api { status: 403 | 404 | 410, .. }) => {
                let fresh = self.load(&mut st).await?;
                let v = fresh.get(index).ok_or_else(|| gone("no such quality"))?;
                media::parse(&self.fetch.text(&v.uri, Route::Main).await?)
            }
            Err(e) => return Err(e),
        };

        let in_break = ads::in_break(&main);
        let (source, from, ad) = if !in_break {
            st.backups.end_break();
            (0, main, AdState::None)
        } else if !self.options.block_ads {
            (0, main, AdState::Showing)
        } else {
            match self.backup(&mut st, &variant).await {
                Some((source, m)) => (source, m, AdState::Backup),
                None => (0, main, AdState::Waiting),
            }
        };
        st.ad = ad;
        let out = st.out.entry(index).or_default();
        if self.options.block_ads {
            out.push(source, from.target_duration, ads::live_segments(&from));
        } else {
            out.push(source, from.target_duration, &from.segments);
        }
        // Segments still being made could turn out to be ads: only when it's all stream.
        if self.options.low_latency && !ads::marks(&from).contains(&true) {
            out.push_prefetch(source, from.segments.last(), &from.prefetch);
        }
        Ok((out.render(), ad))
    }

    /// V1061 A segment of the mock HLS server (`seg/<variant>/<n>.ts`); no mock, no segment.
    pub async fn mock_segment(&self, path: &str) -> Result<Vec<u8>> {
        let base = self.fetch.mock_base().ok_or_else(|| gone("gone"))?;
        if path.contains("..") {
            return Err(gone("gone"));
        }
        self.fetch.bytes(&format!("{base}/seg/{path}")).await
    }

    async fn load(&self, st: &mut State) -> Result<Vec<Variant>> {
        let text = self.fetch.live_master(&self.login, Route::Main).await?;
        let variants: Vec<Variant> = master::parse(&text).into_iter().filter(|v| !v.portrait).collect();
        if variants.is_empty() {
            return Err(gone("no qualities"));
        }
        st.variants = Some(variants.clone());
        Ok(variants)
    }

    /// A backup's playlist that's all stream at its live edge. Source ids: 1 + backup.
    async fn backup(&self, st: &mut State, target: &Variant) -> Option<(u8, Media)> {
        let now = Instant::now();
        for i in st.backups.order(now) {
            let route = st.backups.routes[i];
            // A cached master may have gone stale: one fresh try.
            for fresh in [false, true] {
                if fresh || st.backups.masters[i].is_none() {
                    let Ok(text) = self.fetch.live_master(&self.login, route).await else { break };
                    st.backups.masters[i] = Some(master::parse(&text));
                }
                let Some(uri) = st.backups.masters[i]
                    .as_deref()
                    .and_then(|m| backup::closest(m, target))
                    .map(|v| v.uri.clone())
                else {
                    break;
                };
                match self.fetch.text(&uri, route).await {
                    Ok(text) => {
                        let m = media::parse(&text);
                        if m.segments.is_empty() || ads::in_break(&m) {
                            st.backups.has_ads(i);
                            break;
                        }
                        st.backups.clean(i);
                        return Some((i as u8 + 1, m));
                    }
                    Err(_) => st.backups.masters[i] = None,
                }
            }
        }
        st.backups.pause(now);
        None
    }
}

pub(crate) fn gone(message: &str) -> Error {
    Error::Api {
        status: 404,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::tests::Script;

    fn master_for(kind: &str) -> String {
        format!(
            "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=8000000,RESOLUTION=1920x1080,FRAME-RATE=60.000,STABLE-VARIANT-ID=\"1080p60\",IVS-NAME=\"1080p60\"\n\
             https://{kind}/1080.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=160000,CODECS=\"mp4a.40.2\",STABLE-VARIANT-ID=\"audio_only\"\nhttps://{kind}/audio.m3u8\n"
        )
    }

    /// Segments `from..to` at 2 s, from `host`; `ads` of them at the end are ads.
    fn playlist(host: &str, from: u64, to: u64, ads: u64) -> String {
        let mut s = format!("#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXT-X-MEDIA-SEQUENCE:{from}\n");
        for n in from..to {
            let title = if n >= to - ads { "Amazon|1" } else { "live" };
            let secs = n * 2;
            s += &format!(
                "#EXT-X-PROGRAM-DATE-TIME:2026-10-09T12:{:02}:{:02}.000Z\n#EXTINF:2.000,{title}\nhttps://{host}/{n}.ts\n",
                secs / 60,
                secs % 60
            );
        }
        s + &format!("#EXT-X-TWITCH-PREFETCH:https://{host}/{to}.ts\n")
    }

    fn script() -> Script {
        let s = Script::default();
        for kind in ["popout", "site", "embed", "mobile_web", "autoplay"] {
            s.masters.lock().unwrap().insert(kind, master_for(kind));
        }
        s
    }

    fn set(s: &Script, url: &str, text: String) {
        s.media.lock().unwrap().insert(url.into(), text);
    }

    fn uris(text: &str) -> Vec<&str> {
        text.lines().filter(|l| l.starts_with("https://")).collect()
    }

    const OPTIONS: Options = Options {
        low_latency: true,
        block_ads: true,
    };

    #[tokio::test]
    async fn mock_segment_needs_a_mock() {
        let s = script();
        let live = Live::new(&s, "xqc", OPTIONS);
        assert!(matches!(live.mock_segment("360p30/1.ts").await, Err(Error::Api { status: 404, .. })));
    }

    #[tokio::test]
    async fn master_lists_variants_by_index() {
        let s = script();
        let live = Live::new(&s, "xqc", OPTIONS);
        let text = live.master().await.unwrap();
        assert!(text.contains("\n0.m3u8\n") && text.contains("\n1.m3u8\n"));
        assert!(!text.contains("https://"), "the player never sees Twitch's addresses");
    }

    #[tokio::test]
    async fn a_break_plays_from_a_clean_backup_and_comes_back() {
        let s = script();
        let live = Live::new(&s, "xqc", OPTIONS);
        set(&s, "https://popout/1080.m3u8", playlist("main", 0, 5, 0));
        let (text, ad) = live.media(0).await.unwrap();
        assert_eq!(ad, AdState::None);
        assert_eq!(uris(&text).last(), Some(&"https://main/5.ts"), "prefetch listed");

        // Ads on the stream; site has ads too, embed is clean.
        set(&s, "https://popout/1080.m3u8", playlist("main", 2, 9, 3));
        set(&s, "https://site/1080.m3u8", playlist("site", 2, 9, 2));
        set(&s, "https://embed/1080.m3u8", playlist("embed", 2, 9, 0));
        let (text, ad) = live.media(0).await.unwrap();
        assert_eq!(ad, AdState::Backup);
        let list = uris(&text);
        assert!(!list.iter().any(|u| u.contains("main/6") || u.contains("Amazon")));
        assert_eq!(list[6..], ["https://embed/6.ts", "https://embed/7.ts", "https://embed/8.ts", "https://embed/9.ts"]);
        assert_eq!(text.matches("#EXT-X-DISCONTINUITY\n").count(), 1);

        // Next poll: the backup that worked is asked first, site isn't asked again.
        s.asked.lock().unwrap().clear();
        set(&s, "https://popout/1080.m3u8", playlist("main", 3, 10, 4));
        set(&s, "https://embed/1080.m3u8", playlist("embed", 3, 10, 0));
        let (_, ad) = live.media(0).await.unwrap();
        assert_eq!(ad, AdState::Backup);
        assert_eq!(*s.asked.lock().unwrap(), ["https://popout/1080.m3u8", "https://embed/1080.m3u8"]);

        // The break is over: back on the stream after what the backup played.
        set(&s, "https://popout/1080.m3u8", playlist("main", 8, 13, 0));
        let (text, ad) = live.media(0).await.unwrap();
        assert_eq!(ad, AdState::None);
        let list = uris(&text);
        assert_eq!(list[list.len() - 3..], ["https://main/11.ts", "https://main/12.ts", "https://main/13.ts"]);
        assert_eq!(text.matches("#EXT-X-DISCONTINUITY\n").count(), 2);
    }

    #[tokio::test]
    async fn without_a_clean_backup_the_break_is_cut_out() {
        let s = script();
        let live = Live::new(&s, "xqc", OPTIONS);
        set(&s, "https://popout/1080.m3u8", playlist("main", 0, 4, 0));
        live.media(0).await.unwrap();
        set(&s, "https://popout/1080.m3u8", playlist("main", 1, 7, 3));
        for kind in ["site", "embed", "mobile_web", "autoplay"] {
            set(&s, &format!("https://{kind}/1080.m3u8"), playlist(kind, 1, 7, 3));
        }
        let (text, ad) = live.media(0).await.unwrap();
        assert_eq!(ad, AdState::Waiting);
        assert!(!text.contains("Amazon") && !text.contains("main/6.ts"));
        assert!(!text.contains("/7.ts"), "no prefetch during a break");

        // Paused: the next poll doesn't ask the backups again.
        s.asked.lock().unwrap().clear();
        live.media(0).await.unwrap();
        assert_eq!(*s.asked.lock().unwrap(), ["https://popout/1080.m3u8"]);

        // Live again: playback goes on, marked as a jump.
        set(&s, "https://popout/1080.m3u8", playlist("main", 6, 11, 0));
        let (text, ad) = live.media(0).await.unwrap();
        assert_eq!(ad, AdState::None);
        assert!(text.contains("https://main/10.ts"));
        assert_eq!(text.matches("#EXT-X-DISCONTINUITY\n").count(), 1);
    }

    #[tokio::test]
    async fn the_proxy_goes_before_autoplay() {
        let mut s = script();
        s.proxy = true;
        s.masters.lock().unwrap().insert("proxy", master_for("proxy"));
        let live = Live::new(&s, "xqc", OPTIONS);
        set(&s, "https://popout/1080.m3u8", playlist("main", 0, 6, 2));
        for kind in ["site", "embed", "mobile_web", "autoplay"] {
            set(&s, &format!("https://{kind}/1080.m3u8"), playlist(kind, 0, 6, 2));
        }
        set(&s, "https://proxy/1080.m3u8", playlist("proxy", 0, 6, 0));
        let (text, ad) = live.media(0).await.unwrap();
        assert_eq!(ad, AdState::Backup);
        assert!(text.contains("https://proxy/5.ts"));
        assert!(!s.asked.lock().unwrap().iter().any(|a| a.contains("autoplay")));
    }

    #[tokio::test]
    async fn audio_only_master() {
        let s = script();
        let live = Live::new(&s, "xqc", OPTIONS);
        let text = live.audio_master().await.unwrap();
        assert_eq!(text.matches("#EXT-X-STREAM-INF").count(), 1);
        assert!(text.contains("CODECS=\"mp4a.40.2\"") && text.ends_with("\n1.m3u8\n"));
    }

    #[tokio::test]
    async fn blocking_off_shows_the_ads() {
        let s = script();
        let live = Live::new(&s, "xqc", Options { low_latency: false, block_ads: false });
        set(&s, "https://popout/1080.m3u8", playlist("main", 0, 4, 2));
        let (text, ad) = live.media(0).await.unwrap();
        assert_eq!(ad, AdState::Showing);
        assert_eq!(uris(&text).len(), 4);
    }

    #[tokio::test]
    async fn offline_and_restarted_streams() {
        let s = Script::default();
        let live = Live::new(&s, "xqc", OPTIONS);
        let e = live.master().await.unwrap_err();
        assert_eq!(e.api_message(), Some("Can not find channel"));

        // The media playlist went away: usher is asked again.
        let s = script();
        let live = Live::new(&s, "xqc", OPTIONS);
        live.master().await.unwrap();
        s.masters.lock().unwrap().insert("popout", master_for("popout2"));
        set(&s, "https://popout2/1080.m3u8", playlist("main", 0, 3, 0));
        let (text, _) = live.media(0).await.unwrap();
        assert!(text.contains("https://main/2.ts"));
    }
}
