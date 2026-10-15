use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

use cyex_auth::secrets::MemoryStore;
use cyex_auth::{SevenTvToken, SevenTvTokens};
use cyex_emotes::seventv::account::Client;
use cyex_twitch::Http;

use super::select::{Pick, Picked, SelectError, superseded};
use super::{Cosmetics, Kind, drop_session, mock_base};

/// A made-up token (`{"sub":"01X","exp":2000}`).
const FAKE: &str = "e30.eyJzdWIiOiIwMVgiLCJleHAiOjIwMDB9.c2ln";

/// Answers each connection with the next canned reply and returns the request heads.
fn serve(replies: Vec<Vec<u8>>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let h = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for r in replies {
            let (mut s, _) = l.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 2048];
            loop {
                let n = s.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&buf[..n]);
                if let Some(end) = got.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&got[..end]).to_lowercase();
                    let len = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if got.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            seen.push(String::from_utf8_lossy(&got).into_owned());
            s.write_all(&r).unwrap();
        }
        seen
    });
    (base, h)
}

fn reply(head: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {head}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn tokens() -> SevenTvTokens {
    let t = SevenTvTokens::new(Arc::new(MemoryStore::default()));
    t.put("7", &SevenTvToken::parse(FAKE).unwrap()).unwrap();
    t
}

fn pick<'a>(tokens: &'a SevenTvTokens, id: &str, mine: u64) -> Pick<'a> {
    Pick {
        tokens,
        account: "7",
        kind: Kind::Paint,
        id: Some(id.to_owned()),
        mine,
        user: Some("01X".into()),
        now: 100,
    }
}

fn cosmetics(base: &str) -> Cosmetics {
    Cosmetics::new(Client::at(Http::new().unwrap(), base))
}

#[test]
fn newer_picks_overtake() {
    assert!(superseded(2, 1));
    assert!(!superseded(2, 2));
    assert!(!superseded(1, 1));
}

#[tokio::test]
async fn two_quick_picks_send_only_the_second() {
    let body = r#"{"data":{"users":{"user":{"activePaint":{"style":{"activePaintId":"P2"}}}}}}"#;
    let (base, server) = serve(vec![reply("200 OK", body)]);
    let c = cosmetics(&base);
    let t = tokens();
    let (a, b) = (c.ticket("7", Kind::Paint), c.ticket("7", Kind::Paint));
    let (first, second) = tokio::join!(c.send(pick(&t, "P1", a)), c.send(pick(&t, "P2", b)));
    assert_eq!(first, Ok(Picked::Superseded));
    assert_eq!(
        second,
        Ok(Picked::Done {
            id: Some("P2".into())
        })
    );
    let seen = server.join().unwrap();
    assert_eq!(seen.len(), 1);
    assert!(seen[0].contains("\"P2\""));
}

#[tokio::test]
async fn a_401_forgets_the_token() {
    let (base, server) = serve(vec![reply("401 Unauthorized", "{}")]);
    let c = cosmetics(&base);
    let t = tokens();
    let n = c.ticket("7", Kind::Paint);
    let r = c.send(pick(&t, "P1", n)).await;
    assert_eq!(r, Err(SelectError::SignInAgain));
    assert!(t.get("7").is_none());
    assert!(c.expired.lock().unwrap().contains("7"));
    server.join().unwrap();
}

#[tokio::test]
async fn no_token_and_old_token_send_nothing() {
    // No server: any request would fail as `Down`, not as these errors.
    let c = cosmetics("http://127.0.0.1:9");
    let empty = SevenTvTokens::new(Arc::new(MemoryStore::default()));
    let n = c.ticket("7", Kind::Paint);
    assert_eq!(c.send(pick(&empty, "P", n)).await, Err(SelectError::SignedOut));
    let t = tokens();
    let mut late = pick(&t, "P", n);
    late.now = 5_000;
    assert_eq!(c.send(late).await, Err(SelectError::SignInAgain));
    assert!(t.get("7").is_some());
}

#[test]
fn removing_an_account_deletes_its_session() {
    let t = tokens();
    let other = SevenTvToken::parse(FAKE).unwrap();
    t.put("8", &other).unwrap();
    assert!(drop_session(&t, "7").is_some());
    assert!(t.get("7").is_none());
    assert!(t.get("8").is_some());
    assert!(drop_session(&t, "7").is_none());
}

#[test]
fn the_mock_is_only_ever_this_pc() {
    assert_eq!(mock_base("http://127.0.0.1:5199").as_deref(), Some("http://127.0.0.1:5199"));
    assert_eq!(mock_base("http://127.0.0.1:5199/").as_deref(), Some("http://127.0.0.1:5199"));
    assert_eq!(mock_base("http://127.0.0.1:5199@evil.example"), None);
    assert_eq!(mock_base("http://127.0.0.1:1:2@evil.example/"), None);
    assert_eq!(mock_base("http://127.0.0.1.evil.example:5199"), None);
    assert_eq!(mock_base("https://127.0.0.1:5199"), None);
    assert_eq!(mock_base("http://127.0.0.1:5199/x"), None);
    assert_eq!(mock_base("http://localhost:5199"), None);
    assert_eq!(mock_base("http://127.0.0.1"), None);
}
