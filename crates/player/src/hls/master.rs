//! Master playlists: one variant per quality. Usher's v2 format names them with
//! `STABLE-VARIANT-ID`/`IVS-NAME`; the older one with a `VIDEO` group (`chunked` is the
//! source) and an `EXT-X-MEDIA` label. Variants come in any order.

use std::collections::HashMap;
use std::fmt::Write;

use super::{attr, attrs};

#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    /// e.g. `1080p60`, `720p60`, `audio_only` (`chunked` in the old format).
    pub id: String,
    /// Label, e.g. `1080p60`, `480p`, `audio_only`.
    pub name: String,
    pub uri: String,
    pub bandwidth: u64,
    pub resolution: Option<(u32, u32)>,
    pub frame_rate: Option<f32>,
    pub codecs: String,
    /// The broadcaster's own encode rather than a transcode.
    pub source: bool,
    /// Only for phones: the vertical feed of a multitrack stream.
    pub portrait: bool,
}

impl Variant {
    pub fn is_audio_only(&self) -> bool {
        self.resolution.is_none()
    }

    pub fn height(&self) -> u32 {
        self.resolution.map_or(0, |(_, h)| h)
    }

    pub fn pixels(&self) -> u64 {
        self.resolution.map_or(0, |(w, h)| u64::from(w) * u64::from(h))
    }
}

/// Every variant, landscape ones first from the best down, audio only last.
pub fn parse(text: &str) -> Vec<Variant> {
    let mut names: HashMap<&str, &str> = HashMap::new();
    let mut inf: Option<&str> = None;
    let mut out = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(a) = line.strip_prefix("#EXT-X-MEDIA:") {
            if attr(a, "TYPE") == Some("VIDEO")
                && let (Some(group), Some(name)) = (attr(a, "GROUP-ID"), attr(a, "NAME"))
            {
                names.insert(group, name);
            }
        } else if let Some(a) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            inf = Some(a);
        } else if !line.is_empty()
            && !line.starts_with('#')
            && let Some(a) = inf.take()
        {
            out.push(variant(a, line, &names));
        }
    }
    out.sort_by(|a, b| {
        (a.portrait, a.is_audio_only())
            .cmp(&(b.portrait, b.is_audio_only()))
            .then(b.pixels().cmp(&a.pixels()))
            .then(b.frame_rate.unwrap_or(0.0).total_cmp(&a.frame_rate.unwrap_or(0.0)))
            .then(b.bandwidth.cmp(&a.bandwidth))
    });
    out
}

fn variant(list: &str, uri: &str, names: &HashMap<&str, &str>) -> Variant {
    let mut v = Variant {
        id: String::new(),
        name: String::new(),
        uri: uri.to_owned(),
        bandwidth: 0,
        resolution: None,
        frame_rate: None,
        codecs: String::new(),
        source: false,
        portrait: false,
    };
    let mut groups = None;
    for (key, value) in attrs(list) {
        match key {
            "BANDWIDTH" => v.bandwidth = value.parse().unwrap_or(0),
            "RESOLUTION" => {
                v.resolution = value
                    .split_once('x')
                    .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
            }
            "FRAME-RATE" => v.frame_rate = value.parse().ok(),
            "CODECS" => v.codecs = value.to_owned(),
            "STABLE-VARIANT-ID" => v.id = value.to_owned(),
            "VIDEO" if v.id.is_empty() => v.id = value.to_owned(),
            "IVS-NAME" => v.name = value.to_owned(),
            "IVS-VARIANT-SOURCE" => v.source = value == "source",
            "IVS-GROUPS" => groups = Some(value),
            _ => {}
        }
    }
    if v.id == "chunked" {
        v.source = true;
    }
    if v.name.is_empty() {
        v.name = names.get(v.id.as_str()).map_or_else(|| v.id.clone(), |n| (*n).to_owned());
    }
    v.portrait = groups.is_some_and(|g| g.split(',').all(|x| x == "portrait"));
    v
}

/// A master playlist listing `variants`, each at the address `uri` gives its index.
pub fn render(variants: &[Variant], uri: impl Fn(usize) -> String) -> String {
    let mut out = String::from("#EXTM3U\n");
    for (i, v) in variants.iter().enumerate() {
        let _ = write!(out, "#EXT-X-STREAM-INF:BANDWIDTH={}", v.bandwidth);
        if let Some((w, h)) = v.resolution {
            let _ = write!(out, ",RESOLUTION={w}x{h}");
        }
        if !v.codecs.is_empty() {
            let _ = write!(out, ",CODECS=\"{}\"", v.codecs);
        }
        if let Some(f) = v.frame_rate {
            let _ = write!(out, ",FRAME-RATE={f:.3}");
        }
        // hls.js reads NAME into the level's name.
        let _ = writeln!(out, ",NAME=\"{}\"", v.name.replace('"', ""));
        out.push_str(&uri(i));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Usher v2, in the shuffled order it sometimes sends.
    const V2: &str = r#"#EXTM3U
#EXT-X-SESSION-DATA:DATA-ID="NODE",VALUE="video-edge-x"
#EXT-X-SESSION-DATA:DATA-ID="USER-COUNTRY",VALUE="DE"
#EXT-X-STREAM-INF:BANDWIDTH=160000,CODECS="mp4a.40.2",STABLE-VARIANT-ID="audio_only",IVS-NAME="audio_only",IVS-VARIANT-SOURCE="transcode"
https://euc11.playlist.example/v1/playlist/audio.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=1427999,RESOLUTION=852x480,CODECS="avc1.4D401F,mp4a.40.2",FRAME-RATE=30.000,STABLE-VARIANT-ID="480p30",IVS-NAME="480p",IVS-VARIANT-SOURCE="transcode"
https://euc11.playlist.example/v1/playlist/480.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=8478954,RESOLUTION=1920x1080,CODECS="avc1.64002A,mp4a.40.2",FRAME-RATE=60.000,STABLE-VARIANT-ID="1080p60",IVS-NAME="1080p60",IVS-VARIANT-SOURCE="source"
https://euc11.playlist.example/v1/playlist/1080.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=6000000,RESOLUTION=1080x1920,CODECS="hev1.1.6.L123.90.0.0.0.0.0,mp4a.40.2",FRAME-RATE=60.000,STABLE-VARIANT-ID="1080p60-portrait",IVS-NAME="1080p60",IVS-VARIANT-SOURCE="source",IVS-GROUPS="portrait"
https://euc11.playlist.example/v1/playlist/portrait.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=3422999,RESOLUTION=1280x720,CODECS="avc1.4D401F,mp4a.40.2",FRAME-RATE=60.000,STABLE-VARIANT-ID="720p60",IVS-NAME="720p60",IVS-VARIANT-SOURCE="transcode",IVS-GROUPS="landscape"
https://euc11.playlist.example/v1/playlist/720.m3u8
"#;

    /// The older format, still sent on usher's v1 paths.
    const V1: &str = r#"#EXTM3U
#EXT-X-TWITCH-INFO:NODE="video-edge-x",MANIFEST-NODE-TYPE="weaver_cluster",USER-COUNTRY="DE"
#EXT-X-MEDIA:TYPE=VIDEO,GROUP-ID="chunked",NAME="1080p60 (source)",AUTOSELECT=YES,DEFAULT=YES
#EXT-X-STREAM-INF:BANDWIDTH=8534030,RESOLUTION=1920x1080,CODECS="avc1.64002A,mp4a.40.2",VIDEO="chunked",FRAME-RATE=60.000
https://video-weaver.example/v1/playlist/source.m3u8
#EXT-X-MEDIA:TYPE=VIDEO,GROUP-ID="audio_only",NAME="audio_only",AUTOSELECT=NO,DEFAULT=NO
#EXT-X-STREAM-INF:BANDWIDTH=160000,CODECS="mp4a.40.2",VIDEO="audio_only"
https://video-weaver.example/v1/playlist/audio.m3u8
"#;

    fn ids(v: &[Variant]) -> Vec<&str> {
        v.iter().map(|x| x.id.as_str()).collect()
    }

    #[test]
    fn v2_variants_sorted_best_first() {
        let v = parse(V2);
        assert_eq!(ids(&v), ["1080p60", "720p60", "480p30", "audio_only", "1080p60-portrait"]);
        assert!(v[0].source && !v[1].source);
        assert_eq!(v[2].name, "480p");
        assert_eq!(v[0].resolution, Some((1920, 1080)));
        assert_eq!(v[0].frame_rate, Some(60.0));
        assert!(v[3].is_audio_only() && !v[3].portrait);
        assert!(v[4].portrait && !v[1].portrait);
    }

    #[test]
    fn v1_variants_named_by_their_group() {
        let v = parse(V1);
        assert_eq!(ids(&v), ["chunked", "audio_only"]);
        assert_eq!(v[0].name, "1080p60 (source)");
        assert!(v[0].source);
        assert_eq!(v[0].codecs, "avc1.64002A,mp4a.40.2");
    }

    #[test]
    fn renders_what_it_parses() {
        let v: Vec<Variant> = parse(V2).into_iter().filter(|x| !x.portrait).collect();
        let out = render(&v, |i| format!("/v/{i}.m3u8"));
        assert!(out.contains("NAME=\"1080p60\"\n/v/0.m3u8\n"));
        let back = parse(&out);
        assert_eq!(back.len(), v.len());
        assert_eq!(back[1].resolution, v[1].resolution);
        assert!(!out.contains("USER-COUNTRY"), "Twitch's session data stays out");
    }
}
