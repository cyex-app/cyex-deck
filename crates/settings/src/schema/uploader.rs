//! V105-11 The image uploader (Chatterino's, 2.6). Its own partition and never exported:
//! the headers and the URL can hold keys.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Uploader {
    pub enabled: bool,
    /// Where images are POSTed.
    pub url: String,
    /// The multipart field the image goes in.
    pub form_field: String,
    /// `Name: value; Name2: value`.
    pub headers: String,
    /// Where the image link is in the answer, e.g. `{data.link}`. Empty: the whole answer.
    pub link: String,
    pub deletion_link: String,
    /// Ask before an image you paste or drop is uploaded (off by default).
    pub confirm: bool,
}

fn clip(s: &mut String, max: usize) {
    if s.chars().count() > max {
        *s = s.chars().take(max).collect();
    }
}

fn trim_clip(s: &mut String, max: usize) {
    *s = s.trim().to_string();
    clip(s, max);
}

impl Schema for Uploader {
    fn sanitize(&mut self) {
        trim_clip(&mut self.url, 2000);
        trim_clip(&mut self.form_field, 100);
        clip(&mut self.headers, 4000);
        trim_clip(&mut self.link, 500);
        trim_clip(&mut self.deletion_link, 500);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clips() {
        let mut u = Uploader {
            url: format!("  {}  ", "a".repeat(2500)),
            form_field: " file ".into(),
            headers: "h".repeat(5000),
            ..Uploader::default()
        };
        u.sanitize();
        assert_eq!(u.url.chars().count(), 2000);
        assert_eq!(u.form_field, "file");
        assert_eq!(u.headers.chars().count(), 4000);
    }

    #[test]
    fn confirm_is_off_by_default() {
        assert!(!Uploader::default().confirm);
        let u: Uploader = serde_json::from_str("{}").unwrap();
        assert!(!u.confirm);
    }

    #[test]
    fn old_trusted_hosts_field_is_ignored() {
        let u: Uploader =
            serde_json::from_str(r#"{"enabled":true,"url":"http://x/up","trustedHosts":["x"]}"#).unwrap();
        assert!(u.enabled && !u.confirm);
        assert_eq!(u.url, "http://x/up");
    }
}
