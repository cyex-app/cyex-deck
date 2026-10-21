//! V107-R19 Profile picture, banner and offline image uploads. Twitch's internal GQL, taken
//! from twitch.tv's web JS (no persisted hash, raw text). Not run live yet: the mutation
//! names, input fields and format strings all live in `OPS` / `Format::gql` below.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Auth, run_at};
use crate::{Error, Http, Result};

/// Twitch's upload limit for all three.
pub const MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageKind {
    Avatar,
    Banner,
    Offline,
}

/// The mutation field and input type for each kind. Change names here only.
const OPS: [(ImageKind, &str, &str); 3] = [
    (ImageKind::Avatar, "createProfileImageUploadURL", "CreateProfileImageUploadURLInput"),
    (ImageKind::Banner, "createProfileBannerImageUploadURL", "CreateProfileBannerImageUploadURLInput"),
    (ImageKind::Offline, "createChannelOfflineImageUploadURL", "CreateChannelOfflineImageUploadURLInput"),
];

fn op(kind: ImageKind) -> (&'static str, &'static str) {
    let (_, field, input) = OPS.iter().find(|(k, ..)| *k == kind).copied().unwrap_or(OPS[0]);
    (field, input)
}

/// The mutation text for a kind, as twitch.tv sends it.
pub fn mutation(kind: ImageKind) -> String {
    let (field, input) = op(kind);
    format!(
        "mutation EditProfile_{field}($input: {input}!) {{ {field}(input: $input) {{ uploadID uploadURL error {{ code }} }} }}"
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpg,
    Gif,
}

impl Format {
    /// From the file's first bytes, not its name.
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(Self::Jpg)
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            Some(Self::Gif)
        } else {
            None
        }
    }

    /// The enum string in the mutation input (not confirmed live).
    pub fn gql(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpg => "JPG",
            Self::Gif => "GIF",
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Gif => "gif",
        }
    }
}

/// Where the upload goes, already decoded.
#[derive(Debug, PartialEq, Eq)]
pub struct Upload {
    pub id: String,
    pub url: String,
}

/// Asks for an upload address. A payload `error` comes back as `Error::Refused(code)`.
pub async fn create(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    kind: ImageKind,
    user_id: &str,
    format: Format,
) -> Result<Upload> {
    let vars = json!({ "input": { "userID": user_id, "format": format.gql() } });
    read_create(kind, run_at(http, auth, base, &mutation(kind), vars).await?)
}

fn read_create(kind: ImageKind, data: Value) -> Result<Upload> {
    let out: Created = serde_json::from_value(data[op(kind).0].clone()).map_err(Error::Decode)?;
    if let Some(e) = out.error {
        return Err(Error::Refused(e.code));
    }
    match (out.upload_id, out.upload_url) {
        (Some(id), Some(url)) if !url.is_empty() => Ok(Upload { id, url: percent_decode(&url) }),
        _ => Err(Error::Refused("NO_UPLOAD_URL".into())),
    }
}

/// Sends the raw file, no headers (as twitch.tv does).
pub async fn put(http: &Http, url: &str, bytes: Vec<u8>) -> Result<()> {
    let res = http
        .0
        .put(url)
        .body(bytes)
        .send()
        .await
        .map_err(|e| Error::Network(e.without_url()))?;
    let status = res.status().as_u16();
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(Error::Api { status, message: "upload".into() })
    }
}

/// The user's current images, full size. Empty when not set.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Images {
    pub avatar: String,
    pub banner: String,
    pub offline: String,
}

impl Images {
    pub fn get(&self, kind: ImageKind) -> &str {
        match kind {
            ImageKind::Avatar => &self.avatar,
            ImageKind::Banner => &self.banner,
            ImageKind::Offline => &self.offline,
        }
    }
}

/// Public read, no token.
pub async fn images(http: &Http, base: Option<&str>, user_id: &str) -> Result<Images> {
    let q = "query($id: ID!) { user(id: $id) { profileImageURL(width: 600) bannerImageURL offlineImageURL } }";
    let user = run_at(http, Auth::PUBLIC, base, q, json!({ "id": user_id })).await?["user"].take();
    let s = |k: &str| user[k].as_str().unwrap_or_default().to_owned();
    Ok(Images {
        avatar: s("profileImageURL"),
        banner: s("bannerImageURL"),
        offline: s("offlineImageURL"),
    })
}

/// An image as a `data:` URL (test runs: the window may not load images from this PC).
pub fn data_url(bytes: &[u8]) -> String {
    use base64::Engine;
    let mime = match Format::sniff(bytes) {
        Some(Format::Png) => "image/png",
        Some(Format::Jpg) => "image/jpeg",
        Some(Format::Gif) => "image/gif",
        None => "application/octet-stream",
    };
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// `decodeURIComponent`: `%XX` to bytes, the rest as is. A broken escape stays literal.
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Created {
    #[serde(rename = "uploadID")]
    upload_id: Option<String>,
    #[serde(rename = "uploadURL")]
    upload_url: Option<String>,
    error: Option<Code>,
}

#[derive(Deserialize)]
struct Code {
    code: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_formats() {
        assert_eq!(Format::sniff(b"\x89PNG\r\n\x1a\n...."), Some(Format::Png));
        assert_eq!(Format::sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(Format::Jpg));
        assert_eq!(Format::sniff(b"GIF89a.."), Some(Format::Gif));
        assert_eq!(Format::sniff(b"RIFF....WEBP"), None);
        assert_eq!(Format::sniff(b""), None);
    }

    #[test]
    fn decodes_like_decode_uri_component() {
        assert_eq!(percent_decode("https%3A%2F%2Fx.test%2Fa%3Fb%3D1"), "https://x.test/a?b=1");
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(percent_decode("a%zz+b"), "a%zz+b");
        assert_eq!(percent_decode("plain"), "plain");
    }

    #[test]
    fn mutation_text_names_the_op() {
        let m = mutation(ImageKind::Banner);
        assert!(m.contains("createProfileBannerImageUploadURL(input: $input)"));
        assert!(m.contains("$input: CreateProfileBannerImageUploadURLInput!"));
        assert!(mutation(ImageKind::Avatar).starts_with("mutation EditProfile_createProfileImageUploadURL("));
    }

    #[test]
    fn reads_the_upload_or_the_code() {
        let ok = json!({ "createProfileImageUploadURL": {
            "uploadID": "u1", "uploadURL": "http%3A%2F%2F127.0.0.1%3A1%2Fput%2Fu1", "error": null } });
        let up = read_create(ImageKind::Avatar, ok).unwrap();
        assert_eq!(up, Upload { id: "u1".into(), url: "http://127.0.0.1:1/put/u1".into() });
        let bad = json!({ "createChannelOfflineImageUploadURL": {
            "uploadID": null, "uploadURL": null, "error": { "code": "FILE_SIZE_VALIDATION_FAILED" } } });
        let e = read_create(ImageKind::Offline, bad).unwrap_err();
        assert!(matches!(e, Error::Refused(c) if c == "FILE_SIZE_VALIDATION_FAILED"));
        let empty = json!({ "createProfileBannerImageUploadURL": { "uploadID": "x", "uploadURL": "" } });
        assert!(matches!(read_create(ImageKind::Banner, empty), Err(Error::Refused(_))));
    }
}
