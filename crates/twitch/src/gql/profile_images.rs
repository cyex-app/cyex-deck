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
    /// The channel page's banner choice (`PRESET_1` the profile accent colour, `PRESET_2` the
    /// custom image). `None` when Twitch didn't say.
    pub hero: Option<String>,
}

/// V107-Mz Twitch keeps an uploaded banner (`bannerImageURL`) after the channel switches to
/// the accent colour; the channel page then shows none. Only `PRESET_1` hides it.
pub fn banner_shown(hero: Option<&str>) -> bool {
    hero != Some(GENERATED_BANNER)
}

impl Images {
    /// The channel page shows the accent colour instead of the banner image.
    pub fn banner_is_accent(&self) -> bool {
        !banner_shown(self.hero.as_deref())
    }

    /// What the channel page shows now: like `get`, but no banner while the page uses the
    /// accent colour. `get` stays the raw address, which upload checks compare.
    pub fn shown(&self, kind: ImageKind) -> &str {
        if kind == ImageKind::Banner && self.banner_is_accent() {
            return "";
        }
        self.get(kind)
    }

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
    let q = "query($id: ID!) { user(id: $id) { profileImageURL(width: 600) bannerImageURL offlineImageURL channel { home { preferences { heroPreset } } } } }";
    let user = run_at(http, Auth::PUBLIC, base, q, json!({ "id": user_id })).await?["user"].take();
    let s = |k: &str| user[k].as_str().unwrap_or_default().to_owned();
    Ok(Images {
        avatar: s("profileImageURL"),
        banner: s("bannerImageURL"),
        offline: s("offlineImageURL"),
        hero: user["channel"]["home"]["preferences"]["heroPreset"].as_str().map(str::to_owned),
    })
}

/// V107-Mh Back to Twitch's default, where twitch.tv has a way (static read, 2026/10/22):
/// the avatar's Delete in settings › profile and the dashboard's offline image ✕. V107-Mz5 The
/// banner has no delete; its "remove" is the brand page's Generated Background (`set_hero`),
/// which keeps the uploaded image.
pub fn removable(_kind: ImageKind) -> bool {
    true
}

/// One of Twitch's 7 default pictures (`profile_image_001`..`007`): the one twitch.tv shows
/// for an account without a picture, `id % 7`. twitch.tv's Delete picks one at random.
pub fn default_avatar(user_id: &str) -> String {
    let n = user_id.parse::<u64>().map_or(0, |n| n % 7);
    format!("profile_image_{:03}", n + 1)
}

/// Twitch's default pictures live under `user-default-pictures`.
pub fn is_default_avatar(url: &str) -> bool {
    url.contains("/user-default-pictures")
}

/// The payload field, the mutation text and its variables, as twitch.tv sends them.
fn remove_op(kind: ImageKind, user_id: &str) -> Option<(&'static str, &'static str, Value)> {
    match kind {
        ImageKind::Avatar => Some((
            "setUserDefaultProfileImage",
            "mutation EditProfile_SetUserDefaultProfileImage($input: SetUserDefaultProfileImageInput!) { setUserDefaultProfileImage(input: $input) { user { id } } }",
            json!({ "input": { "userID": user_id, "newDefaultProfileImage": default_avatar(user_id) } }),
        )),
        ImageKind::Offline => Some((
            "setUserDefaultChannelOfflineImage",
            "mutation DeleteUserChannelOfflineImage($input: SetUserDefaultChannelOfflineImageInput!) { setUserDefaultChannelOfflineImage(input: $input) { error { code } } }",
            json!({ "input": { "userID": user_id, "newDefaultChannelOfflineImage": "empty_channel_offline_image" } }),
        )),
        ImageKind::Banner => None,
    }
}

/// Removes the image. A payload `error` (or none at all) comes back as `Error::Refused`.
pub async fn remove(http: &Http, auth: Auth<'_>, base: Option<&str>, kind: ImageKind, user_id: &str) -> Result<()> {
    if kind == ImageKind::Banner {
        return set_hero(http, auth, base, user_id, GENERATED_BANNER).await;
    }
    let (field, text, vars) = remove_op(kind, user_id).ok_or_else(|| Error::Refused("NOT_REMOVABLE".into()))?;
    read_remove(field, &run_at(http, auth, base, text, vars).await?)
}

fn read_remove(field: &str, data: &Value) -> Result<()> {
    let out = &data[field];
    if out.is_null() {
        return Err(Error::Refused("NO_PAYLOAD".into()));
    }
    match out["error"]["code"].as_str() {
        Some(code) => Err(Error::Refused(code.to_owned())),
        None => Ok(()),
    }
}

/// V107-Mz3 The channel page's banner choice for "Custom Image" (`heroPreset`).
pub const CUSTOM_BANNER: &str = "PRESET_2";
/// V107-Mz5 … and for "Generated Background" (the profile accent colour).
pub const GENERATED_BANNER: &str = "PRESET_1";

/// V107-Mz3 Sets the channel page's banner choice, as the Save under twitch.tv's settings ›
/// channel › brand › Profile Banner does (`UpdateHeroPreset`, chunk
/// `settings-channel-page.components.brand`). A payload `error` comes back as `Error::Refused`.
pub async fn set_hero(http: &Http, auth: Auth<'_>, base: Option<&str>, user_id: &str, preset: &str) -> Result<()> {
    let text = "mutation UpdateHeroPreset($input: UpdateChannelHomePreferencesInput!) { updateChannelHomePreferences(input: $input) { channel { id home { preferences { heroPreset } } } error } }";
    let vars = json!({ "input": { "channelID": user_id, "heroPreset": preset } });
    read_hero(&run_at(http, auth, base, text, vars).await?)
}

fn read_hero(data: &Value) -> Result<()> {
    let out = &data["updateChannelHomePreferences"];
    if out.is_null() {
        return Err(Error::Refused("NO_PAYLOAD".into()));
    }
    match &out["error"] {
        Value::Null => Ok(()),
        e => Err(Error::Refused(e.as_str().map_or_else(|| e.to_string(), str::to_owned))),
    }
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
    fn hero_payload_reads_error_or_ok() {
        assert!(read_hero(&json!({ "updateChannelHomePreferences": { "channel": { "id": "1" }, "error": null } })).is_ok());
        assert!(matches!(read_hero(&json!({ "updateChannelHomePreferences": { "error": "FORBIDDEN" } })), Err(Error::Refused(c)) if c == "FORBIDDEN"));
        assert!(matches!(read_hero(&json!({ "updateChannelHomePreferences": null })), Err(Error::Refused(_))));
    }

    #[test]
    fn mutation_text_names_the_op() {
        let m = mutation(ImageKind::Banner);
        assert!(m.contains("createProfileBannerImageUploadURL(input: $input)"));
        assert!(m.contains("$input: CreateProfileBannerImageUploadURLInput!"));
        assert!(mutation(ImageKind::Avatar).starts_with("mutation EditProfile_CreateProfileImageUploadURL("));
    }

    #[test]
    fn remove_ops_match_twitch() {
        assert!(removable(ImageKind::Avatar) && removable(ImageKind::Offline) && removable(ImageKind::Banner));
        assert!(remove_op(ImageKind::Banner, "1").is_none());
        let (field, text, vars) = remove_op(ImageKind::Avatar, "141981764").unwrap();
        assert_eq!(field, "setUserDefaultProfileImage");
        assert!(text.contains("setUserDefaultProfileImage(input: $input) { user { id } }"));
        assert_eq!(vars, json!({ "input": { "userID": "141981764", "newDefaultProfileImage": "profile_image_002" } }));
        let (field, text, vars) = remove_op(ImageKind::Offline, "9").unwrap();
        assert_eq!(field, "setUserDefaultChannelOfflineImage");
        assert!(text.starts_with("mutation DeleteUserChannelOfflineImage($input: SetUserDefaultChannelOfflineImageInput!)"));
        assert_eq!(vars["input"]["newDefaultChannelOfflineImage"], "empty_channel_offline_image");
    }

    #[test]
    fn banner_shows_unless_the_page_uses_the_accent_colour() {
        assert!(banner_shown(Some("PRESET_2")) && banner_shown(None));
        assert!(!banner_shown(Some("PRESET_1")));
        let mut a = Images { banner: "b".into(), ..Images::default() };
        assert!(!a.banner_is_accent());
        a.hero = Some("PRESET_1".into());
        assert!(a.banner_is_accent());
        let mut i = Images { banner: "https://x/b.png".into(), ..Images::default() };
        assert_eq!(i.shown(ImageKind::Banner), "https://x/b.png");
        i.hero = Some("PRESET_1".into());
        assert_eq!(i.shown(ImageKind::Banner), "");
        assert_eq!(i.get(ImageKind::Banner), "https://x/b.png");
        i.avatar = "a".into();
        assert_eq!(i.shown(ImageKind::Avatar), "a");
    }

    #[test]
    fn default_avatar_is_id_mod_7() {
        assert_eq!(default_avatar("7"), "profile_image_001");
        assert_eq!(default_avatar("13"), "profile_image_007");
        assert_eq!(default_avatar("x"), "profile_image_001");
        assert!(is_default_avatar("https://static-cdn.jtvnw.net/user-default-pictures/ab-profile_image-300x300.jpg"));
        assert!(!is_default_avatar("https://static-cdn.jtvnw.net/jtv_user_pictures/ab-profile_image-600x600.png"));
    }

    #[test]
    fn reads_the_remove_answer() {
        let f = "setUserDefaultChannelOfflineImage";
        assert!(read_remove(f, &json!({ "setUserDefaultChannelOfflineImage": { "error": null } })).is_ok());
        assert!(read_remove("setUserDefaultProfileImage", &json!({ "setUserDefaultProfileImage": { "user": { "id": "1" } } })).is_ok());
        let e = read_remove(f, &json!({ "setUserDefaultChannelOfflineImage": { "error": { "code": "UNAUTHORIZED" } } })).unwrap_err();
        assert!(matches!(e, Error::Refused(c) if c == "UNAUTHORIZED"));
        assert!(matches!(read_remove(f, &json!({ "setUserDefaultChannelOfflineImage": null })), Err(Error::Refused(_))));
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
