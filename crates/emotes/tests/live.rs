//! Checks against the real providers, signed out. Run by hand:
//! `cargo test -p cyex-emotes --test live -- --ignored --nocapture`

use std::time::Duration;

use cyex_emotes::seventv::socket::{EventApi, Topic};
use cyex_emotes::{bttv, ffz, seventv};
use cyex_twitch::pace::Lane;
use cyex_twitch::ws;
use cyex_twitch::Http;

const XQC: &str = "71092938";
/// pajlada: has BTTV, FFZ and 7TV emotes.
const PAJLADA: &str = "11148817";

#[tokio::test]
#[ignore = "network"]
async fn seventv_sets() {
    let http = Http::new().unwrap();
    let global = seventv::api::global(&http).await.unwrap();
    assert!(global.emotes.iter().any(|e| e.zero_width));
    let ch = seventv::api::channel(&http, XQC).await.unwrap().unwrap();
    let set = ch.set.unwrap();
    println!("7tv xqc: {} emotes, user {}", set.emotes.len(), ch.user_id);
    assert!(set.emotes.len() > 100);
    assert!(
        seventv::api::channel(&http, "999999999999")
            .await
            .unwrap()
            .is_none()
    );
    let names = seventv::api::user_names(&http, std::slice::from_ref(&ch.user_id))
        .await
        .unwrap();
    println!("7tv names: {names:?}");
    assert_eq!(names.get(&ch.user_id).map(String::as_str), Some("xQc"));
}

#[tokio::test]
#[ignore = "network"]
async fn bttv_and_ffz() {
    let http = Http::new().unwrap();
    let g = bttv::api::global(&http).await.unwrap();
    assert!(g.emotes.len() > 30);
    let ch = bttv::api::channel(&http, PAJLADA, Lane::Front).await.unwrap().unwrap();
    println!("bttv pajlada: {}", ch.emotes.len());
    assert!(bttv::api::channel(&http, "999999999999", Lane::Front).await.unwrap().is_none());
    let (defs, users) = bttv::api::badges(&http).await.unwrap();
    assert!(!defs.is_empty() && users.len() > 50);
    let cmds = bttv::api::bot_commands(&http, XQC).await.unwrap();
    println!("bot commands xqc: {}", cmds.len());

    let f = ffz::api::global(&http).await.unwrap();
    assert!(
        f.sets
            .iter()
            .any(|s| s.emotes.iter().any(|e| e.modifier.is_some()))
    );
    let room = ffz::api::room(&http, PAJLADA, Lane::Front).await.unwrap().unwrap();
    println!(
        "ffz pajlada: {:?} emotes, mod badge {}",
        room.set.map(|s| s.emotes.len()),
        room.badges.moderator.is_some()
    );
    assert!(ffz::api::room(&http, "999999999999", Lane::Front).await.unwrap().is_none());
    let (defs, users) = ffz::api::badges(&http).await.unwrap();
    println!(
        "ffz badges: {} kinds, {} users",
        defs.len(),
        users.iter().map(|u| u.1.len()).sum::<usize>()
    );
}

/// Joins a few busy channels' cosmetics and prints what arrives in 40 s.
#[tokio::test]
#[ignore = "network"]
async fn seventv_events() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (topics, task) = ws::start(EventApi::new(), tx);
    tokio::spawn(task);
    for id in [XQC, "26490481", "96858382"] {
        for t in Topic::channel(id) {
            topics.add(t);
        }
    }
    let mut n = 0;
    let _ = tokio::time::timeout(Duration::from_secs(40), async {
        while let Some(e) = rx.recv().await {
            n += 1;
            if n <= 5 {
                println!("{e:?}");
            }
        }
    })
    .await;
    println!("7tv events in 40 s: {n}");
}
