//! Checks against the real services, signed out. Run by hand:
//! `cargo test -p cyex-twitch --test live -- --ignored`

use cyex_twitch::{Http, gql, recent};

#[tokio::test]
#[ignore = "network"]
async fn chat_catalogs() {
    let http = Http::new().unwrap();
    let global = gql::global_chat(&http).await.unwrap();
    assert!(global.badges.len() > 100);
    assert!(global.cheermotes.iter().any(|c| c.prefix == "Cheer"));
    let ch = gql::channel_chat(&http, "xqc", &global.cheer_colors).await.unwrap().unwrap();
    assert_eq!(ch.channel_id, "71092938");
    assert!(ch.badges.iter().any(|b| b.set == "subscriber"));
    let points = ch.points.unwrap();
    println!("points: {:?} {} custom, {} automatic", points.name, points.custom.len(), points.automatic.len());
    let pin = gql::pinned(&http, &ch.channel_id).await.unwrap();
    println!("pinned: {:?}", pin.map(|p| p.kind));
    let p = gql::profile(&http, Some("twitch"), None).await.unwrap().unwrap();
    assert!(p.partner && p.followers.unwrap() > 0);
    let lines = recent::recent_messages(&http, "twitch").await.unwrap();
    assert!(lines.iter().all(|l| l.contains("historical=1")));
}
