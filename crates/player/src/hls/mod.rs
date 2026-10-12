//! HLS playlists (RFC 8216), as far as Twitch uses them.

pub mod master;
pub mod media;

/// A tag's attribute list (RFC 8216 4.2): `A=1,B="x, y",C=NO`. Quotes are removed.
pub(crate) fn attrs(list: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut rest = list;
    while let Some(eq) = rest.find('=') {
        let key = rest[..eq].trim();
        rest = &rest[eq + 1..];
        let value;
        if let Some(quoted) = rest.strip_prefix('"') {
            let end = quoted.find('"').unwrap_or(quoted.len());
            value = &quoted[..end];
            rest = quoted.get(end + 1..).unwrap_or("");
        } else {
            let end = rest.find(',').unwrap_or(rest.len());
            value = rest[..end].trim();
            rest = &rest[end..];
        }
        out.push((key, value));
        match rest.find(',') {
            Some(i) => rest = &rest[i + 1..],
            None => break,
        }
    }
    out
}

pub(crate) fn attr<'a>(list: &'a str, key: &str) -> Option<&'a str> {
    attrs(list).into_iter().find(|(k, _)| *k == key).map(|(_, v)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_lists_keep_commas_in_quotes() {
        let list = r#"BANDWIDTH=8534030,RESOLUTION=1920x1080,CODECS="avc1.64002A,mp4a.40.2",VIDEO="chunked""#;
        assert_eq!(
            attrs(list),
            [
                ("BANDWIDTH", "8534030"),
                ("RESOLUTION", "1920x1080"),
                ("CODECS", "avc1.64002A,mp4a.40.2"),
                ("VIDEO", "chunked"),
            ]
        );
        assert_eq!(attr(list, "VIDEO"), Some("chunked"));
        assert_eq!(attr(list, "NAME"), None);
        assert_eq!(attrs(r#"A="open"#), [("A", "open")]);
        assert!(attrs("").is_empty());
    }
}
