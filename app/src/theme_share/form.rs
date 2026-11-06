//! V109-T01 The multipart body for `POST`/`PUT /themes/v1`: the theme JSON, maybe a picture,
//! maybe `wallpaperRemove`.

/// What happens to the stored picture on an upload.
#[derive(Debug, PartialEq, Eq)]
pub enum Picture {
    /// Leave it as it is (an update with the same file, or a link or none on a first share).
    Keep,
    /// This file: (extension, bytes).
    File(String, Vec<u8>),
    /// Drop it.
    Remove,
}

pub fn mime(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    }
}

/// `(content type, body)`.
pub fn build(theme_json: &str, picture: &Picture, boundary: &str) -> (String, Vec<u8>) {
    let mut body = Vec::new();
    let mut part = |head: String, bytes: &[u8]| {
        body.extend_from_slice(format!("--{boundary}\r\n{head}\r\n\r\n").as_bytes());
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    };
    part(
        "Content-Disposition: form-data; name=\"theme\"\r\nContent-Type: application/json".into(),
        theme_json.as_bytes(),
    );
    match picture {
        Picture::Keep => {}
        Picture::File(ext, bytes) => part(
            format!(
                "Content-Disposition: form-data; name=\"wallpaper\"; filename=\"wallpaper.{ext}\"\r\nContent-Type: {}",
                mime(ext)
            ),
            bytes,
        ),
        Picture::Remove => part("Content-Disposition: form-data; name=\"wallpaperRemove\"".into(), b"1"),
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(p: &Picture) -> String {
        String::from_utf8_lossy(&build("{\"a\":1}", p, "B").1).into_owned()
    }

    #[test]
    fn theme_only() {
        let (ct, _) = build("{}", &Picture::Keep, "B");
        assert_eq!(ct, "multipart/form-data; boundary=B");
        assert_eq!(
            text(&Picture::Keep),
            "--B\r\nContent-Disposition: form-data; name=\"theme\"\r\nContent-Type: application/json\r\n\r\n{\"a\":1}\r\n--B--\r\n"
        );
    }

    #[test]
    fn with_a_picture_or_a_removal() {
        let t = text(&Picture::File("webp".into(), b"PIC".to_vec()));
        assert!(t.contains("name=\"wallpaper\"; filename=\"wallpaper.webp\"\r\nContent-Type: image/webp\r\n\r\nPIC\r\n--B--"));
        let t = text(&Picture::Remove);
        assert!(t.contains("name=\"wallpaperRemove\"\r\n\r\n1\r\n--B--"));
        assert_eq!(mime("jpg"), "image/jpeg");
    }
}
