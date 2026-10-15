//! V105-11 The pure parts of the image uploader (Chatterino's, 2.6): header text, the
//! multipart body, the link templates and error text. No network here.

use serde_json::Value;

/// `Name: value; Name2: value`. Split on `;`, then on the first `:`; both trimmed, empty skipped.
pub fn headers(text: &str) -> Vec<(String, String)> {
    text.split(';')
        .filter_map(|part| {
            let (k, v) = part.split_once(':')?;
            let (k, v) = (k.trim(), v.trim());
            (!k.is_empty() && !v.is_empty()).then(|| (k.to_string(), v.to_string()))
        })
        .collect()
}

fn plain(s: &str) -> String {
    s.replace(['"', '\r', '\n'], "_")
}

/// One-part multipart form. Returns the `Content-Type` value and the body.
pub fn multipart(field: &str, filename: &str, mime: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    let boundary = format!("cyex-{}", crate::random::hex(8));
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {mime}\r\n\r\n",
        plain(field),
        plain(filename)
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

fn lookup<'a>(json: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = json;
    for part in path.split('.') {
        cur = match cur {
            Value::Object(m) => m.get(part)?,
            Value::Array(a) => a.get(part.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cur)
}

/// Fills `{a.b.0}` placeholders from the JSON answer; empty template: the answer itself.
pub fn fill(template: &str, body: &[u8]) -> String {
    if template.is_empty() {
        return String::from_utf8_lossy(body).trim().to_string();
    }
    let json: Option<Value> = serde_json::from_slice(body).ok();
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start + 1..].find('}') else {
            break;
        };
        out.push_str(&rest[..start]);
        let path = &rest[start + 1..start + 1 + len];
        if let Some(Value::String(s)) = json.as_ref().and_then(|j| lookup(j, path)) {
            out.push_str(s);
        }
        rest = &rest[start + len + 2..];
    }
    out.push_str(rest);
    out
}

/// `413`, plus ` - code: X` and ` - error: Y` when the answer is JSON that has them.
pub fn error_text(status: u16, body: &[u8]) -> String {
    let mut out = status.to_string();
    let Ok(Value::Object(m)) = serde_json::from_slice::<Value>(body) else {
        return out;
    };
    let text = |v: &Value| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    };
    if let Some(c) = m.get("code").and_then(text) {
        out.push_str(&format!(" - code: {}", c.chars().take(20).collect::<String>()));
    }
    if let Some(e) = m.get("error").and_then(text) {
        out.push_str(&format!(" - error: {}", e.chars().take(300).collect::<String>()));
    }
    out
}

pub fn mime(ext: &str) -> Option<&'static str> {
    match ext.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Http;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn header_text() {
        assert_eq!(
            headers("A: b; ;C:d:e;  : x"),
            vec![
                ("A".to_string(), "b".to_string()),
                ("C".to_string(), "d:e".to_string())
            ]
        );
    }

    #[test]
    fn multipart_layout() {
        let (ct, body) = multipart("file", "a\"b.png", "image/png", b"DATA");
        let b = ct.strip_prefix("multipart/form-data; boundary=").unwrap();
        let want = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a_b.png\"\r\nContent-Type: image/png\r\n\r\nDATA\r\n--{b}--\r\n"
        );
        assert_eq!(String::from_utf8(body).unwrap(), want);
    }

    #[test]
    fn templates() {
        let b = br#"{"link":"https://x/a","ext":".png","n":3,"data":{"link":"d"},"files":[{"url":"f0"}]}"#;
        assert_eq!(fill("{link}", b), "https://x/a");
        assert_eq!(fill("{data.link}", b), "d");
        assert_eq!(fill("{files.0.url}", b), "f0");
        assert_eq!(fill("{link}{ext}", b), "https://x/a.png");
        assert_eq!(fill("{nope}", b), "");
        assert_eq!(fill("{n}", b), "");
        assert_eq!(fill("", b"  plain https://x  "), "plain https://x");
        assert_eq!(fill("{link}", b"not json"), "");
    }

    #[test]
    fn errors() {
        assert_eq!(
            error_text(413, b"{\"error\":\"too big\"}"),
            "413 - error: too big"
        );
        assert_eq!(
            error_text(400, b"{\"code\":42,\"error\":\"x\"}"),
            "400 - code: 42 - error: x"
        );
        assert_eq!(error_text(500, b"oops"), "500");
    }

    #[test]
    fn mimes() {
        assert_eq!(mime("PNG"), Some("image/png"));
        assert_eq!(mime("jpeg"), Some("image/jpeg"));
        assert_eq!(mime("txt"), None);
    }

    /// Answers one request on a local port and returns what it received.
    fn serve(reply: String) -> (String, std::thread::JoinHandle<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/up", l.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = s.read(&mut buf).unwrap();
                got.extend_from_slice(&buf[..n]);
                if n == 0 || got.ends_with(b"--\r\n") {
                    break;
                }
            }
            s.write_all(reply.as_bytes()).unwrap();
            String::from_utf8_lossy(&got).into_owned()
        });
        (url, h)
    }

    fn reply(status: &str, json: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
            json.len()
        )
    }

    #[tokio::test]
    async fn posts_to_a_local_host() {
        let (url, h) = serve(reply("200 OK", r#"{"data":{"link":"https://x/a.png"}}"#));
        let (ct, body) = multipart("file", "control_v.png", "image/png", b"PNGDATA");
        let hdrs = vec![("X-Key".to_string(), "test".to_string())];
        let (status, answer) = Http::new()
            .unwrap()
            .post_body(&url, &hdrs, &ct, body, 65536)
            .await
            .unwrap();
        assert_eq!(status, 200);
        assert_eq!(fill("{data.link}", &answer), "https://x/a.png");
        let req = h.join().unwrap();
        assert!(req.starts_with("POST "));
        assert!(req.to_ascii_lowercase().contains("x-key: test"));
        assert!(req.contains("name=\"file\"; filename=\"control_v.png\""));
    }

    #[tokio::test]
    async fn error_answer() {
        let (url, h) = serve(reply("413 Payload Too Large", r#"{"error":"too big"}"#));
        let (ct, body) = multipart("file", "control_v.png", "image/png", b"x");
        let (status, answer) = Http::new()
            .unwrap()
            .post_body(&url, &[], &ct, body, 65536)
            .await
            .unwrap();
        h.join().unwrap();
        assert_eq!(error_text(status, &answer), "413 - error: too big");
    }

    // V105-11 review: only `max` bytes of the answer are kept.
    #[tokio::test]
    async fn answer_is_capped() {
        let (url, h) = serve(reply("200 OK", "0123456789"));
        let (ct, body) = multipart("file", "control_v.png", "image/png", b"x");
        let (_, answer) = Http::new()
            .unwrap()
            .post_body(&url, &[], &ct, body, 4)
            .await
            .unwrap();
        h.join().unwrap();
        assert_eq!(answer, b"0123");
    }
}
