//! V107-R39 Your channel's clips and videos, the way Twitch's dashboard manages them
//! (Content › Clips, Video Producer). Operation texts come from dashboard.twitch.tv's web
//! JS (2026/10/21, docs/research/twitch-api.md); lists were checked live as cyexbot, the
//! writes only against a local mock. `base` points every call at a mock (`{base}/gql`).

pub mod clips;
pub mod videos;

use serde_json::{Value, json};

use crate::gql::{Auth, Response, URL, request_at, unwrap};
use crate::http::json as send_json;
use crate::{Http, Result};

fn url(base: Option<&str>) -> String {
    base.map_or_else(|| URL.to_owned(), |b| format!("{b}/gql"))
}

/// One raw operation; the answer's `data`, or its first GQL error.
async fn run(http: &Http, auth: Auth<'_>, base: Option<&str>, text: &str, variables: Value) -> Result<Value> {
    let req = request_at(http, &url(base), auth).json(&json!({ "query": text, "variables": variables }));
    let res: Response<Value> = send_json(req).await?;
    unwrap(res)
}

#[cfg(test)]
pub(crate) mod mock {
    //! A one-shot GQL server: answers each request with the next reply and hands back what
    //! was sent, so every write is checked for its exact operation and input.

    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::JoinHandle;

    use serde_json::Value;

    pub fn serve(replies: Vec<&'static str>) -> (String, JoinHandle<Vec<Value>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for body in replies {
                let (mut s, _) = l.accept().unwrap();
                let mut got = Vec::new();
                let mut buf = [0u8; 4096];
                let (end, len) = loop {
                    let n = s.read(&mut buf).unwrap();
                    got.extend_from_slice(&buf[..n]);
                    if let Some(end) = got.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&got[..end]).to_lowercase();
                        let len = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if got.len() >= end + 4 + len {
                            break (end, len);
                        }
                    }
                    assert!(n > 0, "request cut short");
                };
                seen.push(serde_json::from_slice(&got[end + 4..end + 4 + len]).unwrap());
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                s.write_all(reply.as_bytes()).unwrap();
            }
            seen
        });
        (base, h)
    }

    pub const AUTH: crate::gql::Auth<'static> = crate::gql::Auth {
        client_id: "mock-client",
        token: Some("mock-token"),
    };
}
