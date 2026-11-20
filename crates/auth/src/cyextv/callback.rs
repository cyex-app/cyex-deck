//! V112-14 The loopback redirect: cyex.tv sends the browser to `/callback?code=…&state=…` on a
//! one-shot listener bound to 127.0.0.1 (a free port). The code is in the query, so the page
//! needs no script.

use std::net::TcpStream;
use std::time::{Duration, Instant};

use super::CyextvError;
use crate::browser::{Listener, decode, host_ok, read_request, respond};

pub const TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, PartialEq, Eq)]
pub enum Callback {
    Code { code: String, state: String },
    /// `error=access_denied` and the like.
    Error { error: String, state: String },
}

/// Reads `code=…&state=…` or `error=…&state=…`. A code is short printable text.
pub fn parse(query: &str) -> Option<Callback> {
    let (mut code, mut state, mut error) = (None, None, None);
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = decode(v)?;
        match k {
            "code" => code = Some(v),
            "state" => state = Some(v),
            "error" => error = Some(v),
            _ => {}
        }
    }
    let state = state?;
    match (code, error) {
        (_, Some(error)) => Some(Callback::Error { error, state }),
        (Some(code), None) if super::token_ok(&code) => Some(Callback::Code { code, state }),
        _ => None,
    }
}

/// Waits for the callback with `state` and returns its code; the port closes then. Blocking:
/// run it off the async runtime. `cancelled` is asked about 10 times a second.
pub fn wait(
    listener: Listener,
    state: &str,
    timeout: Duration,
    cancelled: impl Fn() -> bool,
) -> Result<String, CyextvError> {
    let deadline = Instant::now() + timeout;
    loop {
        if cancelled() {
            return Err(CyextvError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(CyextvError::TimedOut);
        }
        match listener.0.accept() {
            Ok((stream, peer)) => {
                if !peer.ip().is_loopback() {
                    continue;
                }
                if let Some(done) = serve(stream, state) {
                    return done;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(CyextvError::Said(e.to_string())),
        }
    }
}

/// One request. `Some` ends the sign-in.
fn serve(mut stream: TcpStream, state: &str) -> Option<Result<String, CyextvError>> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let Some(req) = read_request(&stream) else {
        respond(&mut stream, "400 Bad Request", "text/plain", "Bad request.");
        return None;
    };
    if !host_ok(req.host.as_deref()) {
        respond(&mut stream, "421 Misdirected Request", "text/plain", "Wrong host.");
        return None;
    }
    if (req.method.as_str(), req.path.as_str()) != ("GET", "/callback") {
        respond(&mut stream, "404 Not Found", "text/plain", "Not found.");
        return None;
    }
    let html = "text/html; charset=utf-8";
    match parse(&req.query) {
        Some(Callback::Code { code, state: s }) if s == state => {
            respond(&mut stream, "200 OK", html, &page("Signed in to cyex.tv. You can close this tab."));
            Some(Ok(code))
        }
        Some(Callback::Error { error, state: s }) if s == state => {
            respond(&mut stream, "200 OK", html, &page("Sign-in cancelled. You can close this tab."));
            Some(Err(if error == "access_denied" {
                CyextvError::Denied
            } else {
                CyextvError::Said(error.chars().take(60).collect())
            }))
        }
        // An old tab or a forged link: refused, the sign-in keeps waiting.
        _ => {
            respond(&mut stream, "400 Bad Request", html, &page("This sign-in is old. Start again in cyex deck."));
            None
        }
    }
}

fn page(line: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>cyex deck</title><style>:root{{color-scheme:dark light}}body{{margin:0;min-height:100vh;display:grid;place-items:center;font:15px/1.5 "IBM Plex Sans","Segoe UI",system-ui,sans-serif;background:#141414;color:#e8e8e8}}@media(prefers-color-scheme:light){{body{{background:#f5f5f4;color:#1b1b1b}}}}p{{margin:0}}</style></head><body><p>{line}</p><script>history.replaceState(null,'',location.pathname)</script></body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn callbacks_parse() {
        assert_eq!(
            parse("code=abc%2F1&state=xyz"),
            Some(Callback::Code { code: "abc/1".into(), state: "xyz".into() })
        );
        assert_eq!(
            parse("error=access_denied&state=xyz"),
            Some(Callback::Error { error: "access_denied".into(), state: "xyz".into() })
        );
        assert_eq!(parse("code=abc"), None, "no state");
        assert_eq!(parse("state=x"), None, "no code");
        assert_eq!(parse("code=a+b&state=x"), None, "odd code");
        assert_eq!(parse("code=a&state=%zz"), None, "bad escape");
        assert_eq!(parse(""), None);
    }

    fn get(port: u16, target: &str, host: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.write_all(format!("GET {target} HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes()).unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out
    }

    #[test]
    fn takes_the_right_callback_then_closes() {
        let l = Listener::bind_on(0).unwrap();
        let port = l.port();
        let done = std::thread::spawn(move || wait(l, "good", Duration::from_secs(10), || false));
        let me = format!("127.0.0.1:{port}");
        assert!(get(port, "/x", &me).starts_with("HTTP/1.1 404"));
        assert!(get(port, "/callback?code=a&state=good", "evil.example").starts_with("HTTP/1.1 421"));
        assert!(get(port, "/callback?code=a&state=bad", &me).contains("This sign-in is old"));
        assert!(get(port, "/callback?code=c0de&state=good", &me).contains("Signed in to cyex.tv"));
        assert_eq!(done.join().unwrap().unwrap(), "c0de");
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
    }

    #[test]
    fn denied_cancelled_and_timed_out() {
        let l = Listener::bind_on(0).unwrap();
        let port = l.port();
        let done = std::thread::spawn(move || wait(l, "s", Duration::from_secs(10), || false));
        get(port, "/callback?error=access_denied&state=s", &format!("localhost:{port}"));
        assert_eq!(done.join().unwrap(), Err(CyextvError::Denied));
        let l = Listener::bind_on(0).unwrap();
        assert_eq!(wait(l, "s", Duration::from_secs(10), || true), Err(CyextvError::Cancelled));
        let l = Listener::bind_on(0).unwrap();
        assert_eq!(wait(l, "s", Duration::from_millis(200), || false), Err(CyextvError::TimedOut));
    }
}
