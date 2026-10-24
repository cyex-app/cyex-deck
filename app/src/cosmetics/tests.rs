use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

use cyex_auth::secrets::MemoryStore;
use cyex_auth::{SevenTvToken, SevenTvTokens};
use cyex_emotes::seventv::account::Client;
use cyex_twitch::Http;

use super::select::{Pick, forget_if_same, Picked, SelectError, superseded};
use super::{Cosmetics, Kind, drop_session};

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
        consent: &|| true,
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
    assert!(t.rejected("7"));
    server.join().unwrap();
}

#[tokio::test]
async fn a_withdrawn_consent_sends_nothing() {
    let c = cosmetics("http://127.0.0.1:9");
    let t = tokens();
    let n = c.ticket("7", Kind::Paint);
    let mut p = pick(&t, "P", n);
    p.consent = &|| false;
    assert_eq!(c.send(p).await, Err(SelectError::NoConsent));
}

#[test]
fn a_rejected_token_that_was_replaced_stays() {
    let t = tokens();
    let old = SevenTvToken::parse(FAKE).unwrap();
    let fresh = SevenTvToken::parse("e30.eyJzdWIiOiIwMVgiLCJleHAiOjMwMDB9.c2ln").unwrap();
    t.put("7", &fresh).unwrap();
    assert!(!forget_if_same(&t, "7", &old));
    assert!(t.get("7").is_some());
    assert!(forget_if_same(&t, "7", &fresh));
    assert!(t.get("7").is_none());
    // combos 18: the second pick of the same burst finds it already rejected.
    assert!(forget_if_same(&t, "7", &fresh));
    // Signed out (no token, no mark) is not "Sign in again".
    let empty = SevenTvTokens::new(Arc::new(MemoryStore::default()));
    assert!(!forget_if_same(&empty, "7", &fresh));
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

// V108-7K "Paste a 7TV token".
mod paste {
    use super::*;
    use crate::cosmetics::paste::{Refused, check, read};

    fn me(twitch: &str) -> Vec<u8> {
        let body = format!(r#"{{"data":{{"users":{{"me":{{"id":"01X","connections":[{{"platform":"TWITCH","platformId":"{twitch}"}}]}}}}}}}}"#);
        reply("200 OK", &body)
    }

    fn empty() -> SevenTvTokens {
        SevenTvTokens::new(Arc::new(MemoryStore::default()))
    }

    #[test]
    fn format_check() {
        assert!(read(FAKE).is_some());
        assert!(read(&format!("  {FAKE}\r\n")).is_some());
        // The console shows the string in quotes.
        assert!(read(&format!("\"{FAKE}\"")).is_some());
        assert!(read(&format!("'{FAKE}'")).is_some());
        assert!(read("").is_none());
        assert!(read("null").is_none());
        assert!(read("not a token").is_none());
        assert!(read("a.b").is_none());
        assert!(read("a.b.c.d").is_none());
        assert!(read("a..c").is_none());
        assert!(read("a.b=.c").is_none());
        assert!(read(&format!("\"{FAKE}")).is_none());
        let long = format!("{}.{}.c", "a".repeat(600), "b".repeat(600));
        assert!(read(&long).is_none());
    }

    #[tokio::test]
    async fn bad_or_old_tokens_never_reach_7tv() {
        // No server: a request would come back as `Down`.
        let api = Client::at(Http::new().unwrap(), "http://127.0.0.1:9");
        let t = empty();
        assert_eq!(check(&api, &t, "7", "nope", 100).await, Err(Refused::Format));
        assert_eq!(check(&api, &t, "7", FAKE, 5_000).await, Err(Refused::RanOut));
        assert!(t.get("7").is_none());
    }

    #[tokio::test]
    async fn another_twitch_account_is_refused() {
        let (base, h) = serve(vec![me("8"), reply("200 OK", r#"{"data":{"users":{"me":{"id":"01X","connections":[]}}}}"#)]);
        let api = Client::at(Http::new().unwrap(), &base);
        let t = empty();
        assert_eq!(check(&api, &t, "7", FAKE, 100).await, Err(Refused::OtherAccount));
        // No Twitch connection at all: the same.
        assert_eq!(check(&api, &t, "7", FAKE, 100).await, Err(Refused::OtherAccount));
        assert!(t.get("7").is_none());
        // Refused, but not logged out on 7TV: it's the browser's own session.
        let seen = h.join().unwrap();
        assert!(seen.iter().all(|r| r.contains("/v4/gql")));
    }

    #[tokio::test]
    async fn rejected_by_7tv() {
        let (base, h) = serve(vec![reply("401 Unauthorized", "{}")]);
        let api = Client::at(Http::new().unwrap(), &base);
        let t = empty();
        assert_eq!(check(&api, &t, "7", FAKE, 100).await, Err(Refused::Rejected));
        assert!(t.get("7").is_none());
        h.join().unwrap();
    }

    #[tokio::test]
    async fn this_accounts_token_is_kept() {
        let (base, h) = serve(vec![me("7")]);
        let api = Client::at(Http::new().unwrap(), &base);
        let t = empty();
        assert_eq!(check(&api, &t, "7", &format!(" {FAKE}\n"), 100).await, Ok("01X".into()));
        assert_eq!(t.get("7").map(|k| k.secret().to_owned()).as_deref(), Some(FAKE));
        let seen = h.join().unwrap();
        assert!(seen[0].to_lowercase().contains(&format!("authorization: bearer {}", FAKE.to_lowercase())));
    }
}
