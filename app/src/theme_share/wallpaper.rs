//! V110-A7 A theme code can carry a wallpaper link. On apply the picture is fetched once, from
//! the public web only (`reach`), as a picture of a kind wallpapers allow and no bigger than they
//! may be, and kept in the wallpapers folder; the theme then points at that file.

use std::time::Duration;

use crate::chat::run::reach;

const TIMEOUT: Duration = Duration::from_secs(15);
/// The same cap as a picked wallpaper (`theme::MAX_WALLPAPER_BYTES`).
pub const MAX_BYTES: usize = 20 * 1024 * 1024;
const FAILED: &str = "Couldn’t download it.";

/// The file extension for a PNG, JPEG, GIF or WebP, by what the bytes start with.
pub fn picture_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// The picture at `link` and its extension. Errors are sentences for the user.
pub async fn download(link: &str) -> Result<(Vec<u8>, &'static str), String> {
    let mut url = url::Url::parse(link).map_err(|_| "That isn’t a web address.".to_owned())?;
    if url.scheme() != "https" {
        return Err("The wallpaper link isn’t secure (https).".into());
    }
    for _ in 0..=reach::MAX_HOPS {
        let at = url.clone();
        let addr = tokio::task::spawn_blocking(move || reach::resolve(&at))
            .await
            .map_err(|_| FAILED.to_owned())??;
        let client = reqwest::Client::builder()
            .user_agent(cyex_core::app::user_agent())
            .timeout(TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            // V110-N1 No system proxy: the pinned public address must decide where the connection goes.
            .no_proxy()
            .resolve(url.host_str().unwrap_or_default(), addr)
            .build()
            .map_err(|_| FAILED.to_owned())?;
        let mut res = client.get(url.as_str()).send().await.map_err(|_| FAILED.to_owned())?;
        let status = res.status();
        if status.is_redirection() {
            let to = res
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| FAILED.to_owned())?;
            let (next, _) = reach::next_hop(&url, to)?;
            if next.scheme() != "https" {
                return Err("The wallpaper link isn’t secure (https).".into());
            }
            url = next;
            continue;
        }
        if !status.is_success() {
            return Err(FAILED.into());
        }
        if res.content_length().is_some_and(|n| n > MAX_BYTES as u64) {
            return Err("The wallpaper is over 20 MB.".into());
        }
        let mut out = Vec::new();
        while let Some(chunk) = res.chunk().await.map_err(|_| FAILED.to_owned())? {
            out.extend_from_slice(&chunk);
            if out.len() > MAX_BYTES {
                return Err("The wallpaper is over 20 MB.".into());
            }
        }
        let ext = picture_ext(&out).ok_or_else(|| "The wallpaper isn’t a PNG, JPG, GIF or WebP picture.".to_owned())?;
        return Ok((out, ext));
    }
    Err("Too many redirects.".into())
}

/// The sentence for a theme added without its wallpaper.
pub fn without(reason: &str) -> String {
    format!("Added without its wallpaper. {reason}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_are_known_by_their_first_bytes() {
        assert_eq!(picture_ext(b"\x89PNG\r\n\x1a\n...."), Some("png"));
        assert_eq!(picture_ext(&[0xff, 0xd8, 0xff, 0xe0, 0]), Some("jpg"));
        assert_eq!(picture_ext(b"GIF89a.."), Some("gif"));
        assert_eq!(picture_ext(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(picture_ext(b"<svg xmlns=...>"), None);
        assert_eq!(picture_ext(b"<html>"), None);
        assert_eq!(picture_ext(b""), None);
    }

    /// Only the public web: this PC, the home network and plain http are refused before any
    /// request is made.
    #[tokio::test]
    async fn local_and_insecure_links_are_refused() {
        for link in ["https://localhost/a.png", "https://192.168.1.1/a.png", "https://[::1]/a.png", "https://169.254.169.254/a.png"] {
            assert_eq!(download(link).await.unwrap_err(), reach::REFUSED, "{link}");
        }
        assert!(download("http://example.com/a.png").await.unwrap_err().contains("secure"));
        assert!(download("nonsense").await.is_err());
    }

    #[test]
    fn the_note_says_why() {
        assert_eq!(without("The wallpaper is over 20 MB."), "Added without its wallpaper. The wallpaper is over 20 MB.");
    }
}
