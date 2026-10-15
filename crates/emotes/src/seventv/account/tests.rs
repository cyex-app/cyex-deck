use std::io::{Read, Write};
use std::net::TcpListener;

use cyex_twitch::Http;

use super::*;

/// Answers each connection with the next canned reply, returning the request texts.
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

fn client(base: &str) -> Client {
    Client::at(Http::new().unwrap(), base)
}

const FAKE: &str = "aaa.bbb.ccc";

#[tokio::test]
async fn inventory_sends_no_token() {
    let body = r#"{"data":{"users":{"userByConnection":{"id":"01X"}}}}"#;
    let (base, h) = serve(vec![reply("200 OK", body)]);
    let inv = client(&base).inventory("123").await.unwrap().read().unwrap().unwrap();
    assert_eq!(inv.seventv_id, "01X");
    let req = h.join().unwrap().remove(0).to_lowercase();
    assert!(!req.contains("authorization"));
    assert!(req.contains("x-ignore-auth-failure: true"));
    assert!(req.starts_with("post /v4/gql "));
}

#[tokio::test]
async fn no_user_is_none() {
    let (base, _h) = serve(vec![reply(
        "200 OK",
        r#"{"data":{"users":{"userByConnection":null}}}"#,
    )]);
    let answer = client(&base).inventory("1").await.unwrap();
    assert!(answer.read().unwrap().is_none());
}

#[tokio::test]
async fn clearing_a_paint_sends_null_and_the_bearer() {
    let body = r#"{"data":{"users":{"user":{"activePaint":{"id":"01X","style":{"activePaintId":null}}}}}}"#;
    let (base, h) = serve(vec![reply("200 OK", body)]);
    let got = client(&base).set_paint(FAKE, "01X", None).await.unwrap();
    assert_eq!(got, None);
    let req = h.join().unwrap().remove(0);
    assert!(req.contains(r#""paint":null"#));
    assert!(
        req.to_lowercase()
            .contains(&format!("authorization: bearer {FAKE}"))
    );
    assert!(!req.to_lowercase().contains("x-ignore-auth-failure"));
}

#[tokio::test]
async fn setting_a_badge_returns_the_new_id() {
    let body = r#"{"data":{"users":{"user":{"activeBadge":{"id":"01X","style":{"activeBadgeId":"B1"}}}}}}"#;
    let (base, _h) = serve(vec![reply("200 OK", body)]);
    let got = client(&base).set_badge(FAKE, "01X", Some("B1")).await.unwrap();
    assert_eq!(got.as_deref(), Some("B1"));
}

#[tokio::test]
async fn me_finds_the_twitch_connection() {
    let body = r#"{"data":{"users":{"me":{"id":"01X","connections":[
        {"platform":"DISCORD","platformId":"9"},{"platform":"TWITCH","platformId":"123"}]}}}}"#;
    let (base, _h) = serve(vec![reply("200 OK", body)]);
    let me = client(&base).me(FAKE).await.unwrap();
    assert_eq!(me.id, "01X");
    assert_eq!(me.twitch_id.as_deref(), Some("123"));
}

#[tokio::test]
async fn errors_map_to_kinds() {
    let limited = r#"{"errors":[{"message":"x","extensions":{"code":"RATE_LIMIT_EXCEEDED"}}],"data":null}"#;
    let load = r#"{"errors":[{"message":"x","extensions":{"code":"LOAD_ERROR"}}],"data":null}"#;
    let (base, _h) = serve(vec![
        reply("401 Unauthorized", "{}"),
        reply("200 OK", limited),
        reply("429 Too Many Requests\r\nretry-after: 7", "{}"),
        reply("503 Service Unavailable", ""),
        reply("200 OK", load),
    ]);
    let c = client(&base);
    assert_eq!(c.set_paint(FAKE, "u", None).await, Err(Error::SignInAgain));
    assert_eq!(
        c.set_paint(FAKE, "u", None).await,
        Err(Error::RateLimited { seconds: 30 })
    );
    assert_eq!(
        c.set_paint(FAKE, "u", None).await,
        Err(Error::RateLimited { seconds: 7 })
    );
    assert_eq!(c.set_paint(FAKE, "u", None).await, Err(Error::Down));
    assert_eq!(c.set_paint(FAKE, "u", Some("p")).await, Err(Error::NotAllowed));
}

#[tokio::test]
async fn a_closed_port_is_down() {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    drop(l);
    assert_eq!(client(&base).inventory("1").await.err(), Some(Error::Down));
}

#[tokio::test]
async fn logout_treats_401_as_done() {
    let (base, h) = serve(vec![reply("401 Unauthorized", ""), reply("200 OK", "{}")]);
    let c = client(&base);
    assert_eq!(c.logout(FAKE).await, Ok(()));
    assert_eq!(c.logout(FAKE).await, Ok(()));
    assert!(h.join().unwrap()[0].starts_with("POST /v4/auth/logout "));
}

#[test]
fn login_url_follows_the_base() {
    assert_eq!(
        client("http://127.0.0.1:5199/").login_url(),
        "http://127.0.0.1:5199/v4/auth/login?platform=twitch"
    );
}
