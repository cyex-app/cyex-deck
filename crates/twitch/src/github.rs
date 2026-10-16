//! V105-20 The app's own releases on GitHub: which one is newer than the running version,
//! and the conditional request and download that go with it. Nothing about the user is sent.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use cyex_core::version::{self, Version};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Error, Http, Result};

pub const RELEASES: &str = "https://api.github.com/repos/cyex-app/cyex-deck/releases?per_page=20";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    #[serde(default)]
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Download {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// A release newer than the running app.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Update {
    pub version: String,
    pub prerelease: bool,
    pub notes_url: String,
    #[ts(skip)]
    #[serde(skip)]
    pub installer: Option<Download>,
    #[ts(skip)]
    #[serde(skip)]
    pub sha256_url: Option<String>,
    /// V105-20 The installer's minisign signature (`<installer>.sig`). Checked when the update
    /// runs, not for the offer, so an unsigned release says so instead of hiding.
    #[ts(skip)]
    #[serde(skip)]
    pub sig_url: Option<String>,
    /// Both the installer and its hash are there; otherwise only the release page opens.
    pub can_install: bool,
}

/// The newest release above `current`: no drafts, no unreadable tags, pre-releases only when
/// asked for, and not the `skip`ped version.
pub fn pick(
    json: &[u8],
    current: &Version,
    prereleases: bool,
    skip: Option<&Version>,
) -> std::result::Result<Option<Update>, serde_json::Error> {
    let releases: Vec<Release> = serde_json::from_slice(json)?;
    let best = releases
        .into_iter()
        .filter(|r| !r.draft && (prereleases || !r.prerelease))
        .filter_map(|r| version::parse(&r.tag_name).map(|v| (v, r)))
        .filter(|(v, _)| v > current && Some(v) != skip)
        .max_by(|a, b| a.0.cmp(&b.0));
    Ok(best.map(|(v, r)| {
        let installer = r
            .assets
            .iter()
            .find(|a| installer_matches(&a.name, &v))
            .map(|a| Download {
                name: a.name.clone(),
                url: a.browser_download_url.clone(),
                size: a.size,
            });
        let sha256_url = installer.as_ref().and_then(|i| {
            let want = format!("{}.sha256", i.name);
            r.assets
                .iter()
                .find(|a| a.name == want)
                .map(|a| a.browser_download_url.clone())
        });
        let sig_url = installer.as_ref().and_then(|i| {
            let want = format!("{}.sig", i.name);
            r.assets.iter().find(|a| a.name == want).map(|a| a.browser_download_url.clone())
        });
        Update {
            version: v.to_string(),
            prerelease: r.prerelease || !v.pre.is_empty(),
            notes_url: r.html_url,
            can_install: installer.is_some() && sha256_url.is_some(),
            installer,
            sha256_url,
            sig_url,
        }
    }))
}

/// The installer of release `v` is named exactly `cyex-deck_<v>_x64-setup.exe`: an older signed
/// installer attached to a newer tag would otherwise pass as an update (a downgrade).
fn installer_matches(name: &str, v: &Version) -> bool {
    name.strip_prefix("cyex-deck_")
        .and_then(|n| n.strip_suffix("_x64-setup.exe"))
        .and_then(version::parse)
        .is_some_and(|n| &n == v)
}

/// What a conditional request came back with.
#[derive(Debug, PartialEq, Eq)]
pub enum Fetched {
    NotModified,
    Body { etag: Option<String>, body: Vec<u8> },
    RateLimited { reset_unix: Option<i64> },
}

impl Http {
    /// GitHub's API with the last ETag: a 304 doesn't count against the rate limit.
    pub async fn get_conditional(&self, url: &str, etag: Option<&str>) -> Result<Fetched> {
        let mut req = self
            .0
            .get(url)
            .timeout(Duration::from_secs(10))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(e) = etag {
            req = req.header("If-None-Match", e);
        }
        let res = req.send().await.map_err(|e| Error::Network(e.without_url()))?;
        let status = res.status().as_u16();
        let header = |k: &str| res.headers().get(k).and_then(|v| v.to_str().ok()).map(str::to_owned);
        if status == 304 {
            return Ok(Fetched::NotModified);
        }
        if status == 429 || (status == 403 && header("x-ratelimit-remaining").as_deref() == Some("0")) {
            return Ok(Fetched::RateLimited {
                reset_unix: header("x-ratelimit-reset").and_then(|v| v.trim().parse().ok()),
            });
        }
        let new_etag = header("etag");
        let body = res.bytes().await.map_err(|e| Error::Network(e.without_url()))?;
        if !(200..300).contains(&status) {
            return Err(Error::api(status, &body));
        }
        Ok(Fetched::Body {
            etag: new_etag,
            body: body.to_vec(),
        })
    }

    /// Streams `url` into `dest`, reporting `(done, total)`. Fails when more than `expected`
    /// bytes arrive or the stream ends short of it; the caller deletes the file then.
    pub async fn download(
        &self,
        url: &str,
        dest: &Path,
        expected: u64,
        mut on_progress: impl FnMut(u64, u64),
    ) -> Result<()> {
        let fail = |m: &str| Error::Api {
            status: 0,
            message: m.into(),
        };
        let mut res = self
            .0
            .get(url)
            .timeout(Duration::from_secs(600))
            .send()
            .await
            .map_err(|e| Error::Network(e.without_url()))?;
        if !res.status().is_success() {
            return Err(Error::Api {
                status: res.status().as_u16(),
                message: "download".into(),
            });
        }
        let mut file = std::fs::File::create(dest).map_err(|_| fail("file"))?;
        let mut done = 0u64;
        while let Some(chunk) = res.chunk().await.map_err(|e| Error::Network(e.without_url()))? {
            done += chunk.len() as u64;
            if done > expected {
                return Err(fail("too big"));
            }
            file.write_all(&chunk).map_err(|_| fail("file"))?;
            on_progress(done, expected);
        }
        file.flush().map_err(|_| fail("file"))?;
        if done < expected {
            return Err(fail("incomplete"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;

    const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/github-releases.json");

    fn v(s: &str) -> Version {
        version::parse(s).unwrap()
    }

    fn pick_v(current: &str, pre: bool, skip: Option<&str>) -> Option<Update> {
        let skip = skip.map(v);
        pick(FIXTURE, &v(current), pre, skip.as_ref()).unwrap()
    }

    #[test]
    fn picks_the_newest_allowed() {
        let u = pick_v("0.1.5", true, None).unwrap();
        assert_eq!(u.version, "0.1.6-beta.1");
        assert!(u.prerelease && u.can_install);
        assert_eq!(u.installer.unwrap().name, "cyex-deck_0.1.6-beta.1_x64-setup.exe");
        assert!(pick_v("0.1.5", false, None).is_none());
        assert_eq!(pick_v("0.1.4", false, None).unwrap().version, "0.1.5");
        // V105-20 The signature asset is found when it's there, and not required for the offer.
        assert_eq!(
            pick_v("0.1.4", false, None).unwrap().sig_url.as_deref(),
            Some("https://example.test/d/cyex-deck_0.1.5_x64-setup.exe.sig")
        );
        assert!(u.sig_url.is_none() && u.can_install);
        assert_eq!(pick_v("0.1.4", true, Some("0.1.6-beta.1")).unwrap().version, "0.1.5");
        assert!(pick_v("0.1.7", true, None).is_none());
    }

    #[test]
    fn drafts_and_odd_tags_are_ignored() {
        // v0.2.0 is a draft, "nightly" doesn't parse.
        assert!(pick_v("0.1.5", true, None).unwrap().version.starts_with("0.1.6"));
        assert!(pick_v("0.1.99", true, None).is_none());
    }

    #[test]
    fn the_installer_must_match_the_release_version() {
        let json = br#"[{"tag_name":"v0.1.9","html_url":"https://x/r","draft":false,"prerelease":false,
            "assets":[{"name":"cyex-deck_0.1.2_x64-setup.exe","browser_download_url":"https://x/i","size":5},
                      {"name":"cyex-deck_0.1.2_x64-setup.exe.sha256","browser_download_url":"https://x/h","size":5}]}]"#;
        let u = pick(json, &v("0.1.3"), false, None).unwrap().unwrap();
        assert!(u.installer.is_none() && !u.can_install);
        assert!(installer_matches("cyex-deck_0.1.04_x64-setup.exe", &v("0.1.4")));
        assert!(!installer_matches("cyex-deck_0.1.5_x64-setup.exe", &v("0.1.4")));
        assert!(!installer_matches("evil_0.1.4_x64-setup.exe", &v("0.1.4")));
    }

    #[test]
    fn an_update_not_newer_is_not_offered() {
        let json = br#"[{"tag_name":"v0.1.3","draft":false,"prerelease":false,"assets":[]}]"#;
        assert!(pick(json, &v("0.1.3"), false, None).unwrap().is_none());
        assert!(pick(json, &v("0.1.4"), false, None).unwrap().is_none());
    }

    #[test]
    fn tag_0_1_05_takes_the_0_1_5_installer() {
        let json = br#"[{"tag_name":"v0.1.05","html_url":"https://x/r","draft":false,"prerelease":false,
            "assets":[{"name":"cyex-deck_0.1.5_x64-setup.exe","browser_download_url":"https://x/i","size":5},
                      {"name":"cyex-deck_0.1.5_x64-setup.exe.sha256","browser_download_url":"https://x/h","size":5}]}]"#;
        let u = pick(json, &v("0.1.4"), false, None).unwrap().unwrap();
        assert_eq!(u.version, "0.1.5");
        assert!(u.can_install);
    }

    #[test]
    fn a_release_without_a_hash_only_opens_its_page() {
        let json = br#"[{"tag_name":"v0.1.04","html_url":"https://x/r","draft":false,"prerelease":true,
            "assets":[{"name":"cyex-deck_0.1.04_x64-setup.exe","browser_download_url":"https://x/i","size":5}]}]"#;
        let u = pick(json, &v("0.1.3"), true, None).unwrap().unwrap();
        assert_eq!(u.version, "0.1.4");
        assert!(u.prerelease && !u.can_install);
        assert_eq!(u.notes_url, "https://x/r");
    }

    /// Answers each connection with the next canned reply (raw bytes), then closes.
    fn serve(replies: Vec<Vec<u8>>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/r", l.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for r in replies {
                let (mut s, _) = l.accept().unwrap();
                let mut got = Vec::new();
                let mut buf = [0u8; 2048];
                while !got.ends_with(b"\r\n\r\n") {
                    let n = s.read(&mut buf).unwrap();
                    if n == 0 {
                        break;
                    }
                    got.extend_from_slice(&buf[..n]);
                }
                seen.push(String::from_utf8_lossy(&got).into_owned());
                s.write_all(&r).unwrap();
            }
            seen
        });
        (url, h)
    }

    fn reply(head: &str, body: &[u8]) -> Vec<u8> {
        let mut out = format!(
            "HTTP/1.1 {head}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        out.extend_from_slice(body);
        out
    }

    #[tokio::test]
    async fn conditional_requests() {
        let (url, h) = serve(vec![
            reply("200 OK\r\nETag: \"abc\"", b"[]"),
            reply("304 Not Modified", b""),
            reply(
                "403 Forbidden\r\nx-ratelimit-remaining: 0\r\nx-ratelimit-reset: 1700000000",
                b"{}",
            ),
        ]);
        let http = Http::new().unwrap();
        let first = http.get_conditional(&url, None).await.unwrap();
        assert_eq!(
            first,
            Fetched::Body {
                etag: Some("\"abc\"".into()),
                body: b"[]".to_vec()
            }
        );
        assert_eq!(
            http.get_conditional(&url, Some("\"abc\"")).await.unwrap(),
            Fetched::NotModified
        );
        assert_eq!(
            http.get_conditional(&url, None).await.unwrap(),
            Fetched::RateLimited {
                reset_unix: Some(1_700_000_000)
            }
        );
        let seen = h.join().unwrap();
        assert!(seen[1].to_ascii_lowercase().contains("if-none-match: \"abc\""));
        assert!(seen[0].to_ascii_lowercase().contains("x-github-api-version: 2022-11-28"));
    }

    #[tokio::test]
    async fn text_is_capped_and_errors_are_refused() {
        let (url, _h) = serve(vec![reply("200 OK", &[b'a'; 5000]), reply("404 Not Found", b"nope")]);
        let http = Http::new().unwrap();
        assert_eq!(http.get_text(&url, &[], 100).await.unwrap().len(), 100);
        assert!(http.get_text(&url, &[], 100).await.is_err());
    }

    #[tokio::test]
    async fn offline_is_an_error() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/r", l.local_addr().unwrap());
        drop(l);
        assert!(Http::new().unwrap().get_conditional(&url, None).await.is_err());
    }

    #[tokio::test]
    async fn downloads_to_a_file() {
        let data: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
        let (url, _h) = serve(vec![reply("200 OK", &data)]);
        let dest = std::env::temp_dir().join(format!("cyex-gh-test-{}.bin", std::process::id()));
        let mut last = (0, 0);
        Http::new()
            .unwrap()
            .download(&url, &dest, data.len() as u64, |d, t| last = (d, t))
            .await
            .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), data);
        assert_eq!(last, (5000, 5000));
        let _ = std::fs::remove_file(&dest);
    }

    #[tokio::test]
    async fn an_interrupted_download_fails() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/r", l.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut buf = [0u8; 2048];
            let _ = s.read(&mut buf);
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4000\r\nConnection: close\r\n\r\n");
            let _ = s.write_all(&[7u8; 2000]);
        });
        let dest = std::env::temp_dir().join(format!("cyex-gh-test-cut-{}.bin", std::process::id()));
        let res = Http::new().unwrap().download(&url, &dest, 4000, |_, _| {}).await;
        assert!(res.is_err());
        let _ = std::fs::remove_file(&dest);
    }
}
