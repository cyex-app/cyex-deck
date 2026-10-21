//! V107-R41 One local GQL for every dashboard module's tests: answers each request with the
//! next reply and hands back what was sent, so each write is checked for its exact
//! operation and variables.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use serde_json::Value;

use super::Channel;
use crate::Http;
use crate::gql::Auth;

pub const AUTH: Auth<'static> = Auth {
    client_id: "mock-client",
    token: Some("mock-token"),
};
/// The channel every test works on.
pub const ID: &str = "9";

/// A running mock: `base` to aim calls at, `sent()` for what came in.
pub struct Mock {
    pub base: String,
    pub http: Http,
    handle: JoinHandle<Vec<Value>>,
}

impl Mock {
    pub fn channel(&self) -> Channel<'_> {
        Channel {
            http: &self.http,
            auth: AUTH,
            base: Some(&self.base),
            id: ID,
        }
    }

    /// Every request body, in order. Waits until every reply went out.
    pub fn sent(self) -> Vec<Value> {
        self.handle.join().unwrap()
    }
}

/// Serves `replies` one per connection, in order.
pub fn serve<S: Into<String>>(replies: impl IntoIterator<Item = S>) -> Mock {
    let replies: Vec<String> = replies.into_iter().map(Into::into).collect();
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
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
    Mock {
        base,
        http: Http::new().unwrap(),
        handle,
    }
}

/// The operation text a request sent.
pub fn query(sent: &Value) -> &str {
    sent["query"].as_str().unwrap_or_default()
}

/// A channel aimed at a closed port: for calls that must never leave the app.
pub fn off() -> Channel<'static> {
    static HTTP: std::sync::OnceLock<Http> = std::sync::OnceLock::new();
    Channel {
        http: HTTP.get_or_init(|| Http::new().unwrap()),
        auth: AUTH,
        base: Some("http://127.0.0.1:9"),
        id: ID,
    }
}
