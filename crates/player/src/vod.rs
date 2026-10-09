//! A VOD for one player (1.1). Twitch's VOD servers don't let the app's origin in, so
//! playlists and segments both come through here. Segment names stay relative: hls.js
//! asks this proxy for them. Sub-only VODs: usher answers 403.

use cyex_twitch::Result;
use tokio::sync::Mutex;

use crate::fetch::{Fetch, Route};
use crate::hls::master::{self, Variant};
use crate::live::gone;

pub struct Vod<F> {
    fetch: F,
    id: String,
    variants: Mutex<Option<Vec<Variant>>>,
}

impl<F: Fetch> Vod<F> {
    pub fn new(fetch: F, id: &str) -> Self {
        Self {
            fetch,
            id: id.to_owned(),
            variants: Mutex::default(),
        }
    }

    /// Each variant at `<index>/index.m3u8`.
    pub async fn master(&self) -> Result<String> {
        let variants = self.load().await?;
        Ok(master::render(&variants, |i| format!("{i}/index.m3u8")))
    }

    /// P1 A master with only the audio.
    pub async fn audio_master(&self) -> Result<String> {
        let variants = self.load().await?;
        let (i, v) = variants
            .iter()
            .enumerate()
            .find(|(_, v)| v.is_audio_only())
            .ok_or_else(|| gone("no audio only"))?;
        Ok(master::render(std::slice::from_ref(v), |_| format!("{i}/index.m3u8")))
    }

    pub async fn media(&self, index: usize) -> Result<String> {
        let uri = self.uri(index).await?;
        let text = self.fetch.text(&uri, Route::Main).await?;
        Ok(relative(&text, base(&uri)))
    }

    /// A segment of variant `index`, by the name its playlist gives it.
    pub async fn segment(&self, index: usize, name: &str) -> Result<Vec<u8>> {
        if !safe_name(name) {
            return Err(gone("no such segment"));
        }
        let uri = self.uri(index).await?;
        self.fetch.bytes(&format!("{}{name}", base(&uri))).await
    }

    async fn load(&self) -> Result<Vec<Variant>> {
        let mut cached = self.variants.lock().await;
        if let Some(v) = cached.as_ref() {
            return Ok(v.clone());
        }
        let text = self.fetch.vod_master(&self.id).await?;
        let variants: Vec<Variant> = master::parse(&text).into_iter().filter(|v| !v.portrait).collect();
        if variants.is_empty() {
            return Err(gone("no qualities"));
        }
        *cached = Some(variants.clone());
        Ok(variants)
    }

    async fn uri(&self, index: usize) -> Result<String> {
        let variants = self.load().await?;
        variants.get(index).map(|v| v.uri.clone()).ok_or_else(|| gone("no such quality"))
    }
}

/// Up to and with the last `/`.
fn base(uri: &str) -> &str {
    uri[..uri.find('?').unwrap_or(uri.len())].rfind('/').map_or("", |i| &uri[..=i])
}

/// Addresses on the variant's own server become relative, so they come through here.
fn relative(text: &str, base: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if line.starts_with("#EXT-X-MAP:") {
            out.push_str(&line.replace(&format!("URI=\"{base}"), "URI=\""));
        } else if line.starts_with('#') {
            out.push_str(line);
        } else {
            out.push_str(line.strip_prefix(base).unwrap_or(line));
        }
        out.push('\n');
    }
    out
}

/// A plain file name, nothing that climbs out of the variant's folder.
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::tests::Script;

    const MASTER: &str = "#EXTM3U\n\
        #EXT-X-STREAM-INF:BANDWIDTH=6000000,RESOLUTION=1920x1080,FRAME-RATE=60.000,STABLE-VARIANT-ID=\"1080p60\",IVS-NAME=\"1080p60\"\n\
        https://cdn.example/abc_xqc_1_2/chunked/index-muted-X.m3u8\n\
        #EXT-X-STREAM-INF:BANDWIDTH=160000,CODECS=\"mp4a.40.2\",STABLE-VARIANT-ID=\"audio_only\",IVS-NAME=\"Audio Only\"\n\
        https://cdn.example/abc_xqc_1_2/audio_only/index-muted-X.m3u8\n";

    const MEDIA: &str = "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXT-X-PLAYLIST-TYPE:EVENT\n#EXT-X-MEDIA-SEQUENCE:0\n\
        #EXTINF:10.000,\n0.ts\n#EXTINF:10.000,\n1-muted.ts\n#EXTINF:10.000,\nhttps://cdn.example/abc_xqc_1_2/chunked/2.ts\n#EXT-X-ENDLIST\n";

    fn script() -> Script {
        let s = Script::default();
        let mut media = s.media.lock().unwrap();
        media.insert("vod:9".into(), MASTER.into());
        media.insert("https://cdn.example/abc_xqc_1_2/chunked/index-muted-X.m3u8".into(), MEDIA.into());
        media.insert("https://cdn.example/abc_xqc_1_2/chunked/1-muted.ts".into(), "bytes".into());
        drop(media);
        s
    }

    #[tokio::test]
    async fn playlists_and_segments_come_through_here() {
        let s = script();
        let vod = Vod::new(&s, "9");
        let master = vod.master().await.unwrap();
        assert!(master.contains("\n0/index.m3u8\n") && master.contains("NAME=\"Audio Only\"\n1/index.m3u8"));
        let media = vod.media(0).await.unwrap();
        assert!(media.contains("\n0.ts\n") && media.contains("\n2.ts\n#EXT-X-ENDLIST"));
        assert!(!media.contains("https://"));
        assert_eq!(vod.segment(0, "1-muted.ts").await.unwrap(), b"bytes");
        assert!(vod.segment(0, "../../x.ts").await.is_err());
        assert!(vod.segment(5, "0.ts").await.is_err());
        // The master is asked for once.
        assert_eq!(s.asked.lock().unwrap().iter().filter(|a| a.starts_with("vod:")).count(), 1);
    }

    #[test]
    fn names_and_bases() {
        assert_eq!(base("https://a.example/x/y/index.m3u8?t=1/2"), "https://a.example/x/y/");
        assert!(safe_name("123-muted.ts") && safe_name("init-0.mp4"));
        assert!(!safe_name("") && !safe_name("..") && !safe_name("a/b.ts") && !safe_name("a\\b.ts") && !safe_name("%2e%2e"));
    }
}
