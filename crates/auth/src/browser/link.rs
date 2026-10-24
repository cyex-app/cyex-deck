//! V108-7L "Send to cyex deck": a bookmarklet the user saves once and clicks on 7tv.app. It
//! navigates (top-level, so no Private Network Access block) to
//! `http://localhost:17563/7tv#t=<token>&n=<nonce>`; the fragment never reaches a server,
//! so the page served here posts it back, like the Twitch sign-in's `/auth`. The port is
//! only bound while the user waits for it in the app; the nonce is per install.

use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use super::{Listener, decode, host_ok, origin_ok, read_request, respond};
use crate::{AuthError, SevenTvToken};

pub const PATH: &str = "/7tv";
const POST: &str = "/7tv/token";

/// The bookmark's address. Runs only on 7tv.app; elsewhere it says so and does nothing.
pub fn bookmarklet(nonce: &str) -> String {
    format!(
        "javascript:(()=>{{if(location.host!=='7tv.app'){{alert('Open 7tv.app first.');return}}\
         const t=localStorage.getItem('7tv-token');\
         if(!t){{alert('Sign in on 7tv.app first.');return}}\
         location.href='http://localhost:{port}{PATH}#t='+encodeURIComponent(t)+'&n={nonce}'}})()",
        port = super::PORT,
    )
}

/// `t=<token>&n=<nonce>` (the leading `#` is optional). The token's shape is checked here.
pub fn parse(raw: &str) -> Option<(SevenTvToken, String)> {
    let raw = raw.trim().trim_start_matches('#');
    let (mut token, mut nonce) = (None, None);
    for pair in raw.split('&') {
        let (k, v) = pair.split_once('=')?;
        match k {
            "t" => token = Some(decode(v)?),
            "n" => nonce = Some(decode(v)?),
            _ => {}
        }
    }
    Some((SevenTvToken::parse(&token?)?, nonce?))
}

/// Same length and every byte compared, so the time taken says nothing about the nonce.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// The page's post, held open until the app has checked the token; `answer` tells the page.
pub struct Sent {
    pub token: SevenTvToken,
    stream: TcpStream,
}

impl Sent {
    /// The line the page shows.
    pub fn answer(mut self, text: &str) {
        respond(&mut self.stream, "200 OK", "text/plain; charset=utf-8", text);
    }
}

impl Listener {
    /// Serves the bookmarklet's page and waits for a post with this install's `nonce`.
    /// A wrong nonce or a bad token is refused and the wait goes on. Blocking.
    pub fn wait_7tv(self, nonce: &str, timeout: Duration, cancelled: impl Fn() -> bool) -> Result<Sent, AuthError> {
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
                    if let Some(sent) = serve(stream, nonce) {
                        return Ok(sent);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(AuthError::Listener(e.to_string())),
            }
        }
    }
}

fn serve(mut stream: TcpStream, nonce: &str) -> Option<Sent> {
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
        ("GET", PATH) => {
            respond(&mut stream, "200 OK", "text/html; charset=utf-8", PAGE);
            None
        }
        ("POST", POST) => {
            if req.origin.as_deref().is_some_and(|o| !origin_ok(o)) {
                respond(&mut stream, "403 Forbidden", "text/plain", "Wrong origin.");
                return None;
            }
            match parse(&req.body) {
                Some((token, n)) if same(&n, nonce) => {
                    let _ = stream.flush();
                    Some(Sent { token, stream })
                }
                Some(_) => {
                    respond(&mut stream, "403 Forbidden", "text/plain; charset=utf-8", "This bookmark is from another cyex deck. Copy it again from Settings.");
                    None
                }
                None => {
                    respond(&mut stream, "400 Bad Request", "text/plain; charset=utf-8", "That isn’t a 7TV sign-in. Sign in on 7tv.app and try again.");
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

/// Posts the fragment back, clears it from the address bar, shows what the app says.
const PAGE: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>cyex deck</title><style>:root{color-scheme:dark light}body{margin:0;min-height:100vh;display:grid;place-items:center;font:15px/1.5 "IBM Plex Sans","Segoe UI",system-ui,sans-serif;background:#141414;color:#e8e8e8}@media(prefers-color-scheme:light){body{background:#f5f5f4;color:#1b1b1b}}p{margin:0;max-width:32em;padding:0 16px;text-align:center}</style></head><body><p id="m">Connecting to 7TV…</p><script>(async()=>{const m=document.getElementById('m');const h=location.hash;history.replaceState(null,'',location.pathname);try{const r=await fetch('/7tv/token',{method:'POST',headers:{'Content-Type':'text/plain'},body:h});m.textContent=await r.text()}catch(e){m.textContent='cyex deck isn’t waiting for this. Click Wait for the bookmark there first.'}})()</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    const FAKE: &str = "e30.eyJzdWIiOiIwMVgifQ.c2ln";

    #[test]
    fn bookmarklet_runs_only_on_7tv_and_carries_the_nonce() {
        let b = bookmarklet("abc123");
        assert!(b.starts_with("javascript:(()=>{if(location.host!=='7tv.app'){alert('Open 7tv.app first.');return}"));
        assert!(b.contains("localStorage.getItem('7tv-token')"));
        assert!(b.contains("location.href='http://localhost:17563/7tv#t='+encodeURIComponent(t)+'&n=abc123'"));
        // No fetch: a top-level navigation isn't blocked as a private network request.
        assert!(!b.contains("fetch"));
        assert!(!b.contains('\n') && !b.contains('"'));
    }

    #[test]
    fn fragments() {
        let (t, n) = parse(&format!("#t={FAKE}&n=abc")).unwrap();
        assert_eq!((t.secret(), n.as_str()), (FAKE, "abc"));
        assert!(parse(&format!("t={FAKE}")).is_none(), "no nonce");
        assert!(parse("n=abc").is_none(), "no token");
        assert!(parse("t=not-a-token&n=abc").is_none());
        assert!(parse(&format!("t={FAKE}%zz&n=a")).is_none());
        assert!(parse("").is_none());
        assert!(same("abc", "abc") && !same("abc", "abd") && !same("abc", "ab"));
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
            &format!("POST /7tv/token HTTP/1.1\r\nHost: localhost:{port}\r\nOrigin: {origin}\r\nContent-Length: {}\r\n\r\n{body}", body.len()),
        )
    }

    #[test]
    fn serves_the_page_refuses_wrong_nonces_and_takes_the_right_one() {
        let l = Listener::bind_on(0).unwrap();
        let port = l.port();
        let done = std::thread::spawn(move || {
            let sent = l.wait_7tv("good", Duration::from_secs(10), || false).unwrap();
            let secret = sent.token.secret().to_owned();
            sent.answer("Connected to 7TV. You can close this tab.");
            secret
        });
        let page = send(port, &format!("GET /7tv HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n"));
        assert!(page.starts_with("HTTP/1.1 200") && page.contains("fetch('/7tv/token'"));
        // The Twitch sign-in's page isn't served here.
        assert!(send(port, &format!("GET /auth HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n")).starts_with("HTTP/1.1 404"));
        assert!(send(port, "GET /7tv HTTP/1.1\r\nHost: evil.example\r\n\r\n").starts_with("HTTP/1.1 421"));
        let me = format!("http://localhost:{port}");
        assert!(post(port, &format!("#t={FAKE}&n=bad"), &me).starts_with("HTTP/1.1 403"), "wrong nonce");
        assert!(post(port, &format!("#t={FAKE}&n=good"), "https://evil.example").starts_with("HTTP/1.1 403"));
        assert!(post(port, "#t=junk&n=good", &me).starts_with("HTTP/1.1 400"));
        let ok = post(port, &format!("#t={FAKE}&n=good"), &me);
        assert!(ok.ends_with("Connected to 7TV. You can close this tab."));
        assert_eq!(done.join().unwrap(), FAKE);
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err(), "closed once taken");
    }

    #[test]
    fn nothing_listens_unless_waiting_and_a_wait_ends() {
        let l = Listener::bind_on(0).unwrap();
        let port = l.port();
        drop(l);
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
        let l = Listener::bind_on(0).unwrap();
        assert!(matches!(l.wait_7tv("n", Duration::from_millis(200), || false), Err(AuthError::TimedOut)));
        let l = Listener::bind_on(0).unwrap();
        assert!(matches!(l.wait_7tv("n", Duration::from_secs(5), || true), Err(AuthError::Cancelled)));
    }
}
