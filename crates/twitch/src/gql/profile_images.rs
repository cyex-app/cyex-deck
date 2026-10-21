//! V107-R19 Profile picture, banner and offline image uploads. Twitch's internal GQL, taken
//! from twitch.tv's settings chunk (no persisted hash, raw text; checked 2026/10/21, V107-R19b):
//! `EditProfile_Create…UploadURL($input: {userID, format: PNG|JPG|GIF})`, then a plain PUT of
//! the file to `decodeURI(uploadURL)` with no Content-Type (`new Blob([file])`).

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
    let name = input.trim_end_matches("Input");
    format!(
        "mutation EditProfile_{name}($input: {input}!) {{ {field}(input: $input) {{ uploadID uploadURL error {{ code }} }} }}"
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

    /// The `ImageFormat` enum value Twitch's web client sends.
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
        (Some(id), Some(url)) if !url.is_empty() => Ok(Upload { id, url: decode_uri(&url) }),
        _ => Err(Error::Refused("NO_UPLOAD_URL".into())),
    }
}

/// The PUT as twitch.tv sends it: the raw file, the signed URL untouched, no Content-Type
/// (the signature may cover headers, so none are added).
fn put_request(http: &Http, url: &str, bytes: Vec<u8>) -> Result<reqwest::Request> {
    http.0.put(url).body(bytes).build().map_err(|e| Error::Network(e.without_url()))
}

/// Sends the raw file. A refusal carries the storage's error `<Code>` (S3 XML), if any.
pub async fn put(http: &Http, url: &str, bytes: Vec<u8>) -> Result<()> {
    let req = put_request(http, url, bytes)?;
    let res = http.0.execute(req).await.map_err(|e| Error::Network(e.without_url()))?;
    let status = res.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(());
    }
    let body = res.text().await.unwrap_or_default();
    let message = match xml_code(&body) {
        Some(code) => format!("upload: {code}"),
        None => "upload".into(),
    };
    Err(Error::Api { status, message })
}

/// `<Code>X</Code>` from an S3-style error body.
fn xml_code(body: &str) -> Option<&str> {
    let start = body.find("<Code>")? + 6;
    let len = body[start..].find("</Code>")?;
    let code = body[start..start + len].trim();
    (!code.is_empty() && code.len() <= 64).then_some(code)
}

/// The URL without its query values, for logs: the signature and token never get printed.
pub fn redact(url: &str) -> String {
    let (base, query) = url.split_once('?').unwrap_or((url, ""));
    let keys: Vec<&str> = query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| p.split_once('=').map_or(p, |(k, _)| k))
        .collect();
    if keys.is_empty() { base.to_owned() } else { format!("{base}?{}", keys.join("&")) }
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

/// `decodeURI`, as twitch.tv applies to the upload URL: `%XX` to bytes except the reserved
/// `;/?:@&=+$,#`, which stay escaped (decoding `%2B` or `%2F` inside a signed query breaks the
/// signature). A broken escape stays literal.
pub fn decode_uri(s: &str) -> String {
    const KEEP: &[u8] = b";/?:@&=+$,#";
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
            && !KEEP.contains(&((h * 16 + l) as u8))
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
    fn decodes_like_decode_uri() {
        let signed = "https://s3.test/u/a%20b.png?X-Amz-Credential=AK%2F2026%2Fs3&X-Amz-Security-Token=ab%2Bc%3D%3D&s=1";
        assert_eq!(
            decode_uri(signed),
            "https://s3.test/u/a b.png?X-Amz-Credential=AK%2F2026%2Fs3&X-Amz-Security-Token=ab%2Bc%3D%3D&s=1"
        );
        assert_eq!(decode_uri("https%3A%2F%2Fx.test"), "https%3A%2F%2Fx.test");
        assert_eq!(decode_uri("a%2"), "a%2");
        assert_eq!(decode_uri("a%zz+b"), "a%zz+b");
        assert_eq!(decode_uri("plain"), "plain");
    }

    #[test]
    fn put_keeps_the_signed_url_and_adds_no_headers() {
        let http = Http::new().unwrap();
        let url = "https://uploads.test/p/u1?X-Amz-Credential=AK%2F2026%2Fus-west-2%2Fs3&X-Amz-Security-Token=ab%2Bc%3D%3D&X-Amz-Signature=f00";
        let req = put_request(&http, url, b"\x89PNG".to_vec()).unwrap();
        assert_eq!(req.method(), reqwest::Method::PUT);
        assert_eq!(req.url().as_str(), url);
        assert!(req.headers().get(reqwest::header::CONTENT_TYPE).is_none());
        assert_eq!(req.body().and_then(|b| b.as_bytes()), Some(&b"\x89PNG"[..]));
    }

    #[test]
    fn reads_the_storage_code_and_redacts() {
        let xml = "<?xml version=\"1.0\"?><Error><Code>SignatureDoesNotMatch</Code><Message>x</Message></Error>";
        assert_eq!(xml_code(xml), Some("SignatureDoesNotMatch"));
        assert_eq!(xml_code("nope"), None);
        assert_eq!(redact("https://h.test/p?X-Amz-Signature=secret&b=2"), "https://h.test/p?X-Amz-Signature&b");
        assert_eq!(redact("https://h.test/p"), "https://h.test/p");
    }

    #[test]
    fn mutation_text_names_the_op() {
        let m = mutation(ImageKind::Banner);
        assert!(m.contains("createProfileBannerImageUploadURL(input: $input)"));
        assert!(m.contains("$input: CreateProfileBannerImageUploadURLInput!"));
        assert!(mutation(ImageKind::Avatar).starts_with("mutation EditProfile_CreateProfileImageUploadURL("));
    }

    #[test]
    fn reads_the_upload_or_the_code() {
        let ok = json!({ "createProfileImageUploadURL": {
            "uploadID": "u1", "uploadURL": "http://127.0.0.1:1/put/u1?sig=a%2Bb", "error": null } });
        let up = read_create(ImageKind::Avatar, ok).unwrap();
        assert_eq!(up, Upload { id: "u1".into(), url: "http://127.0.0.1:1/put/u1?sig=a%2Bb".into() });
        let bad = json!({ "createChannelOfflineImageUploadURL": {
            "uploadID": null, "uploadURL": null, "error": { "code": "FILE_SIZE_VALIDATION_FAILED" } } });
        let e = read_create(ImageKind::Offline, bad).unwrap_err();
        assert!(matches!(e, Error::Refused(c) if c == "FILE_SIZE_VALIDATION_FAILED"));
        let empty = json!({ "createProfileBannerImageUploadURL": { "uploadID": "x", "uploadURL": "" } });
        assert!(matches!(read_create(ImageKind::Banner, empty), Err(Error::Refused(_))));
    }
}
