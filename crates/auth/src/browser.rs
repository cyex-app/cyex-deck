//! V108-01 "Sign in with Twitch" in the user's own browser: OAuth implicit grant (no
//! secret) with `clients::SIGN_IN`, redirected to a one-shot listener on 127.0.0.1.
//! Twitch puts the token in the URL fragment, which never reaches a server, so the page
//! the listener serves posts `location.hash` back to it.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use crate::{AuthError, scopes};

/// Fixed: the redirect URL has to be registered for the client in Twitch's console.
pub const PORT: u16 = 17563;
pub const REDIRECT: &str = "http://localhost:17563/auth";
pub const TIMEOUT: Duration = Duration::from_secs(5 * 60);
const AUTHORIZE: &str = "https://id.twitch.tv/oauth2/authorize";
/// Largest request the listener reads (the fragment is about 200 bytes).
const MAX_REQUEST: usize = 16 * 1024;

/// A random `state`: 128 bits from two OS-seeded SipHash keys. Only has to be unguessable
/// for the few minutes a sign-in runs.
pub fn new_state() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::with_capacity(32);
    for i in 0..2u8 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u8(i);
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

/// Twitch's sign-in page for `client_id` with the Helix scopes. `force_verify=false`: a
/// browser already signed in to Twitch just approves.
pub fn authorize_url(client_id: &str, state: &str) -> String {
    format!(
        "{AUTHORIZE}?response_type=token&client_id={}&redirect_uri={}&scope={}&state={}&force_verify=false",
        encode(client_id),
        encode(REDIRECT),
        encode(&scopes::HELIX.join(" ")),
        encode(state),
    )
}

/// Whether `url` is Twitch's sign-in page, the only thing the browser sign-in opens.
pub fn is_authorize_url(url: &str) -> bool {
    url.strip_prefix(AUTHORIZE).is_some_and(|rest| rest.starts_with('?'))
}

/// What the redirect's fragment says.
#[derive(Debug, PartialEq, Eq)]
pub enum Fragment {
    Token { access: String, state: String },
    /// `error=access_denied` and the like.
    Error { error: String, state: String },
}

/// Reads `#access_token=…&state=…` (the leading `#` is optional).
pub fn parse_fragment(raw: &str) -> Option<Fragment> {
    let raw = raw.trim().trim_start_matches('#');
    let mut access = None;
    let mut state = None;
    let mut error = None;
    for pair in raw.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = decode(v)?;
        match k {
            "access_token" => access = Some(v),
            "state" => state = Some(v),
            "error" => error = Some(v),
            _ => {}
        }
    }
    let state = state?;
    match (access, error) {
        (_, Some(error)) => Some(Fragment::Error { error, state }),
        (Some(access), None) if !access.is_empty() && access.bytes().all(|b| b.is_ascii_alphanumeric()) => {
            Some(Fragment::Token { access, state })
        }
        _ => None,
    }
}

/// The listener for one sign-in, on 127.0.0.1 only.
pub struct Listener(TcpListener);

impl Listener {
    pub fn bind() -> Result<Self, AuthError> {
        Self::bind_on(PORT)
    }

    /// Port 0 picks a free one (tests).
    pub fn bind_on(port: u16) -> Result<Self, AuthError> {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let l = TcpListener::bind(addr).map_err(|e| match e.kind() {
            std::io::ErrorKind::AddrInUse | std::io::ErrorKind::PermissionDenied => AuthError::PortBusy(port),
            _ => AuthError::Listener(e.to_string()),
        })?;
        l.set_nonblocking(true).map_err(|e| AuthError::Listener(e.to_string()))?;
        Ok(Self(l))
    }

    pub fn port(&self) -> u16 {
        self.0.local_addr().map_or(0, |a| a.port())
    }

    /// Serves the page and waits for its post. Returns the token once one with `state`
    /// arrives; the listener is consumed, so the port closes then. Blocking: run it off the
    /// async runtime. `cancelled` is asked about 10 times a second.
    pub fn wait(self, state: &str, timeout: Duration, cancelled: impl Fn() -> bool) -> Result<String, AuthError> {
        let deadline = Instant::now() + timeout;
        loop {
            if cancelled() {
                return Err(AuthError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(AuthError::TimedOut);
            }
            match self.0.accept() {
                Ok((stream, peer)) => {
                    if !peer.ip().is_loopback() {
                        continue;
                    }
                    if let Some(done) = self.serve(stream, state) {
                        return done;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(AuthError::Listener(e.to_string())),
            }
        }
    }

    /// One request. `Some` ends the sign-in.
    fn serve(&self, mut stream: TcpStream, state: &str) -> Option<Result<String, AuthError>> {
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
        match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/auth") => {
                respond(&mut stream, "200 OK", "text/html; charset=utf-8", PAGE);
                None
            }
            ("POST", "/token") => {
                // Another site's page can post here too; only ours is same-origin.
                if req.origin.as_deref().is_some_and(|o| !origin_ok(o)) {
                    respond(&mut stream, "403 Forbidden", "text/plain", "Wrong origin.");
                    return None;
                }
                match parse_fragment(&req.body) {
                    Some(Fragment::Token { access, state: s }) if s == state => {
                        respond(&mut stream, "200 OK", "text/plain", "ok");
                        Some(Ok(access))
                    }
                    Some(Fragment::Error { error, state: s }) if s == state => {
                        respond(&mut stream, "200 OK", "text/plain", "denied");
                        Some(Err(if error == "access_denied" {
                            AuthError::Denied
                        } else {
                            AuthError::Twitch(error)
                        }))
                    }
                    // An old tab or a forged post: refused, the sign-in keeps waiting.
                    _ => {
                        respond(&mut stream, "400 Bad Request", "text/plain", "stale");
                        None
                    }
                }
            }
            _ => {
                respond(&mut stream, "404 Not Found", "text/plain", "Not found.");
                None
            }
        }
    }
}

struct Request {
    method: String,
    path: String,
    host: Option<String>,
    origin: Option<String>,
    body: String,
}

fn read_request(stream: &TcpStream) -> Option<Request> {
    let mut r = BufReader::new(stream.take(MAX_REQUEST as u64));
    let mut line = String::new();
    r.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_owned();
    let target = parts.next()?;
    let path = target.split(['?', '#']).next().unwrap_or(target).to_owned();
    let (mut host, mut origin, mut length) = (None, None, 0usize);
    loop {
        let mut h = String::new();
        if r.read_line(&mut h).ok()? == 0 {
            return None;
        }
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        let (k, v) = h.split_once(':')?;
        let v = v.trim().to_owned();
        match k.trim().to_ascii_lowercase().as_str() {
            "host" => host = Some(v),
            "origin" => origin = Some(v),
            "content-length" => length = v.parse().ok()?,
            _ => {}
        }
    }
    if length > MAX_REQUEST {
        return None;
    }
    let mut body = vec![0; length];
    r.read_exact(&mut body).ok()?;
    Some(Request {
        method,
        path,
        host,
        origin,
        body: String::from_utf8(body).ok()?,
    })
}

/// DNS rebinding guard: only the names the redirect uses.
fn host_ok(host: Option<&str>) -> bool {
    host.is_some_and(|h| {
        let name = h.rsplit_once(':').map_or(h, |(n, _)| n);
        matches!(name, "localhost" | "127.0.0.1")
    })
}

fn origin_ok(origin: &str) -> bool {
    origin
        .strip_prefix("http://")
        .is_some_and(|h| host_ok(Some(h)))
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &str) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
         Referrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

/// Posts the fragment back, clears it from the address bar, says how it went.
const PAGE: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>cyex deck</title><style>:root{color-scheme:dark light}body{margin:0;min-height:100vh;display:grid;place-items:center;font:15px/1.5 "IBM Plex Sans","Segoe UI",system-ui,sans-serif;background:#141414;color:#e8e8e8}@media(prefers-color-scheme:light){body{background:#f5f5f4;color:#1b1b1b}}p{margin:0}</style></head><body><p id="m">Signing in…</p><script>(async()=>{const m=document.getElementById('m');const h=location.hash;history.replaceState(null,'',location.pathname);try{const r=await fetch('/token',{method:'POST',headers:{'Content-Type':'text/plain'},body:h});const t=await r.text();m.textContent=t==='ok'?'Signed in. You can close this tab.':t==='denied'?'Sign-in cancelled. You can close this tab.':'This sign-in is old. Start again in cyex deck.'}catch(e){m.textContent='cyex deck isn’t waiting for this. Start again there.'}})()</script></body></html>"#;

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn url_has_client_redirect_scopes_and_state() {
        let u = authorize_url("abc123", "s1");
        assert!(is_authorize_url(&u));
        assert!(u.starts_with("https://id.twitch.tv/oauth2/authorize?response_type=token&client_id=abc123&"));
        assert!(u.contains("&redirect_uri=http%3A%2F%2Flocalhost%3A17563%2Fauth&"));
        assert!(u.contains("&scope=chat%3Aedit%20chat%3Aread%20"));
        assert!(u.ends_with("&state=s1&force_verify=false"));
        assert!(!is_authorize_url("https://id.twitch.tv/oauth2/authorizeX?a"));
        assert!(!is_authorize_url("https://evil.example/?https://id.twitch.tv/oauth2/authorize?"));
    }

    #[test]
    fn states_differ() {
        let (a, b) = (new_state(), new_state());
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
    }

    #[test]
    fn fragments_parse() {
        assert_eq!(
            parse_fragment("#access_token=abc123&scope=chat%3Aread+chat%3Aedit&state=xyz&token_type=bearer"),
            Some(Fragment::Token { access: "abc123".into(), state: "xyz".into() })
        );
        assert_eq!(
            parse_fragment("error=access_denied&error_description=The+user+denied&state=xyz"),
            Some(Fragment::Error { error: "access_denied".into(), state: "xyz".into() })
        );
        assert_eq!(parse_fragment("#access_token=abc"), None, "no state");
        assert_eq!(parse_fragment("#state=x"), None, "no token");
        assert_eq!(parse_fragment("#access_token=a%22b&state=x"), None, "odd token");
        assert_eq!(parse_fragment("#access_token=a&state=%zz"), None, "bad escape");
        assert_eq!(parse_fragment(""), None);
    }

    fn send(port: u16, raw: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.write_all(raw.as_bytes()).unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out
    }

    fn post(port: u16, body: &str, origin: &str) -> String {
        send(
            port,
            &format!(
                "POST /token HTTP/1.1\r\nHost: localhost:{port}\r\nOrigin: {origin}\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            ),
        )
    }

    fn start(state: &'static str, timeout: Duration) -> (u16, std::thread::JoinHandle<Result<String, AuthError>>) {
        let l = Listener::bind_on(0).unwrap();
        let port = l.port();
        (port, std::thread::spawn(move || l.wait(state, timeout, || false)))
    }

    #[test]
    fn serves_page_rejects_bad_posts_takes_the_right_one_then_closes() {
        let (port, done) = start("good", Duration::from_secs(10));
        let page = send(port, &format!("GET /auth HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n"));
        assert!(page.starts_with("HTTP/1.1 200") && page.contains("fetch('/token'"));
        assert!(send(port, &format!("GET /x HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n")).starts_with("HTTP/1.1 404"));
        // DNS rebinding: another name pointing at 127.0.0.1.
        assert!(send(port, "GET /auth HTTP/1.1\r\nHost: evil.example\r\n\r\n").starts_with("HTTP/1.1 421"));
        let me = format!("http://localhost:{port}");
        assert!(post(port, "#access_token=abc&state=bad", &me).contains("stale"), "state mismatch");
        assert!(post(port, "#access_token=abc&state=good", "https://evil.example").starts_with("HTTP/1.1 403"));
        assert!(post(port, "#access_token=abc123&state=good", &me).ends_with("ok"));
        assert_eq!(done.join().unwrap().unwrap(), "abc123");
        // One-shot: the port is closed now.
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
    }

    #[test]
    fn denied_ends_the_sign_in() {
        let (port, done) = start("s", Duration::from_secs(10));
        post(port, "#error=access_denied&state=s", &format!("http://127.0.0.1:{port}"));
        assert!(matches!(done.join().unwrap(), Err(AuthError::Denied)));
    }

    #[test]
    fn times_out_and_cancels() {
        let (_, done) = start("s", Duration::from_millis(300));
        assert!(matches!(done.join().unwrap(), Err(AuthError::TimedOut)));
        let flag = Arc::new(AtomicBool::new(false));
        let l = Listener::bind_on(0).unwrap();
        let f = flag.clone();
        let t = std::thread::spawn(move || l.wait("s", Duration::from_secs(10), || f.load(Ordering::Relaxed)));
        flag.store(true, Ordering::Relaxed);
        assert!(matches!(t.join().unwrap(), Err(AuthError::Cancelled)));
    }

    #[test]
    fn binds_to_127_0_0_1_only_and_reports_a_busy_port() {
        let l = Listener::bind_on(0).unwrap();
        let addr = l.0.local_addr().unwrap();
        assert_eq!(addr.ip(), std::net::IpAddr::from(Ipv4Addr::LOCALHOST));
        assert!(matches!(Listener::bind_on(addr.port()), Err(AuthError::PortBusy(_))));
    }

    #[test]
    fn hosts() {
        assert!(host_ok(Some("localhost:17563")));
        assert!(host_ok(Some("127.0.0.1:17563")));
        assert!(!host_ok(Some("localhost.evil.example:17563")));
        assert!(!host_ok(None));
        assert!(origin_ok("http://localhost:17563"));
        assert!(!origin_ok("https://localhost:17563"));
        assert!(!origin_ok("null"));
    }
}
