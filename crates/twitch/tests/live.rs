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

#[tokio::test]
#[ignore = "network"]
async fn tab_channels() {
    let http = Http::new().unwrap();
    let ids: Vec<String> = ["71092938", "44445592", "1"].map(String::from).to_vec();
    let list = gql::channels_by_ids(&http, &ids).await.unwrap();
    assert!(list.iter().any(|c| c.login == "xqc"));
    println!("live: {:?}", list.iter().map(|c| (&c.login, c.stream.is_some())).collect::<Vec<_>>());
    let found = gql::lookup(&http, "https://www.twitch.tv/pokimane").await.unwrap();
    assert_eq!(found[0].id, "44445592");
    let both = gql::lookup(&http, "71092938").await.unwrap();
    assert_eq!(both.len(), 2);
    let none = gql::lookup(&http, "id:1").await.unwrap();
    println!("id 1: {:?}", none.first().map(|c| &c.login));
}
