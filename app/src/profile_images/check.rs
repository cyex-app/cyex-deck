//! V107-R19 What a file must be before any call, and Twitch's answers as short text.

use cyex_twitch::Error;
use cyex_twitch::gql::profile_images::{Format, MAX_BYTES};
use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ImageErrorKind {
    Format,
    TooBig,
    Rejected,
    /// Integrity or authorization: the UI offers Twitch's own settings page.
    Refused,
    /// The login ran out (HTTP 401): sign in again.
    SignIn,
    Network,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImageError {
    pub kind: ImageErrorKind,
    pub message: String,
}

impl ImageError {
    pub fn new(kind: ImageErrorKind) -> Self {
        let message = match kind {
            ImageErrorKind::Format => "Only PNG, JPG or GIF.",
            ImageErrorKind::TooBig => "That file is over 10 MB.",
            ImageErrorKind::Rejected => "Twitch didn’t take that image.",
            ImageErrorKind::Refused => "Twitch didn’t allow this from the app.",
            ImageErrorKind::SignIn => "Sign in to Twitch again.",
            ImageErrorKind::Network => "Couldn’t reach Twitch.",
            ImageErrorKind::Failed => "That didn’t work. Try again in a bit.",
        };
        Self { kind, message: message.into() }
    }

    pub fn with(kind: ImageErrorKind, message: &str) -> Self {
        Self { kind, message: message.into() }
    }
}

/// The file's format, or why it can't go up. Checked before any call.
pub fn validate(bytes: &[u8]) -> Result<Format, ImageError> {
    if bytes.len() > MAX_BYTES {
        return Err(ImageError::new(ImageErrorKind::TooBig));
    }
    Format::sniff(bytes).ok_or_else(|| ImageError::new(ImageErrorKind::Format))
}

/// Twitch's answer as one of the kinds above.
pub fn map(e: &Error) -> ImageError {
    use ImageErrorKind as K;
    let kind = match e {
        Error::Network(_) => K::Network,
        Error::Refused(code) => code_kind(code),
        Error::Api { status: 401, .. } => K::SignIn,
        Error::Api { status: 403, .. } => K::Refused,
        Error::Api { message, .. } if refused_text(message) => K::Refused,
        Error::Api { status: 413, .. } => K::TooBig,
        Error::Api { status: 400..=499, .. } => K::Rejected,
        _ => K::Failed,
    };
    ImageError::new(kind)
}

fn code_kind(code: &str) -> ImageErrorKind {
    let c = code.to_ascii_uppercase();
    if refused_text(&c) {
        ImageErrorKind::Refused
    } else if c.contains("FORMAT") {
        ImageErrorKind::Format
    } else if c.contains("SIZE") {
        ImageErrorKind::TooBig
    } else {
        ImageErrorKind::Rejected
    }
}

/// GQL's integrity and auth refusals, e.g. "failed integrity check", "unauthorized".
fn refused_text(s: &str) -> bool {
    let s = s.to_ascii_lowercase();
    ["integrity", "unauthorized", "unauthenticated", "forbidden", "not authorized", "permission"]
        .iter()
        .any(|w| s.contains(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest";

    #[test]
    fn validates_before_any_call() {
        assert_eq!(validate(PNG), Ok(Format::Png));
        assert_eq!(validate(b"GIF89a").unwrap(), Format::Gif);
        assert_eq!(validate(b"RIFF0000WEBP").unwrap_err().kind, ImageErrorKind::Format);
        assert_eq!(validate(b"").unwrap_err().kind, ImageErrorKind::Format);
        let mut big = PNG.to_vec();
        big.resize(MAX_BYTES, 0);
        assert!(validate(&big).is_ok());
        big.push(0);
        assert_eq!(validate(&big).unwrap_err().kind, ImageErrorKind::TooBig);
    }

    #[test]
    fn maps_twitch_answers() {
        use ImageErrorKind as K;
        let api = |status, m: &str| Error::Api { status, message: m.into() };
        let k = |e: Error| map(&e).kind;
        assert_eq!(k(api(200, "failed integrity check")), K::Refused);
        assert_eq!(k(api(401, "x")), K::SignIn);
        assert_eq!(map(&api(401, "x")).message, "Sign in to Twitch again.");
        assert_eq!(k(api(403, "upload")), K::Refused);
        assert_eq!(k(api(200, "Unauthorized")), K::Refused);
        assert_eq!(k(Error::Refused("UNAUTHORIZED".into())), K::Refused);
        assert_eq!(k(Error::Refused("IMAGE_FORMAT_VALIDATION_FAILED".into())), K::Format);
        assert_eq!(k(Error::Refused("FILE_SIZE_VALIDATION_FAILED".into())), K::TooBig);
        assert_eq!(k(Error::Refused("IS_IMAGE_VALIDATION_FAILED".into())), K::Rejected);
        assert_eq!(k(api(413, "upload")), K::TooBig);
        assert_eq!(k(api(400, "upload")), K::Rejected);
        assert_eq!(k(api(500, "upload")), K::Failed);
        assert_eq!(k(api(200, "service timeout")), K::Failed);
        assert_eq!(map(&api(200, "failed integrity check")).message, "Twitch didn’t allow this from the app.");
    }
}
