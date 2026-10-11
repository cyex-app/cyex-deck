//! V101-28: the dashboard parsers against real traffic. Listens anonymously to the alert
//! and moderator feed topics of a few big channels for a minute. Run by hand:
//! `cargo test -p cyex-eventsub --test live_alerts -- --ignored --nocapture`

use cyex_eventsub::alerts::{parse_alert, parse_streak};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

/// xqc, jynxzi, zackrawrr, kaicenat, summit1g, shroud, eliasn97, trymacs.
const CHANNELS: [&str; 8] = [
    "71092938",
    "411377640",
    "552120296",
    "641972806",
    "26490481",
    "37402112",
    "238813810",
    "64342766",
];

#[tokio::test]
#[ignore = "network"]
async fn alerts_and_streaks_parse() {
    let url = "wss://hermes.twitch.tv/v1?clientId=kimne78kx3ncx6brgo4mv6wki5h1ko";
    let (mut ws, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    let (mut alerts, mut parsed, mut feed, mut streaks) = (0, 0, 0, 0);
    let mut unparsed = std::collections::BTreeSet::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    while let Ok(Some(Ok(msg))) = tokio::time::timeout_at(deadline, ws.next()).await {
        let Message::Text(text) = msg else { continue };
        let v: Value = serde_json::from_str(&text).unwrap();
        match v["type"].as_str().unwrap_or_default() {
            "welcome" => {
                for (i, c) in CHANNELS.iter().enumerate() {
                    for (code, topic) in [
                        ("l", format!("activity-feed-alerts-v2.{c}")),
                        ("m", format!("activity-feed-moderator-v2.{c}")),
                    ] {
                        let s = json!({"type":"subscribe","id":format!("r{code}{i}"),
                            "subscribe":{"id":format!("{code}{i}"),"type":"pubsub","pubsub":{"topic":topic}},
                            "timestamp":"2026-10-11T00:00:00Z"});
                        ws.send(Message::Text(s.to_string().into())).await.unwrap();
                    }
                }
            }
            "notification" => {
                let sub = v["notification"]["subscription"]["id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                let inner: Value =
                    serde_json::from_str(v["notification"]["pubsub"].as_str().unwrap_or("{}")).unwrap();
                if sub.starts_with('l') {
                    alerts += 1;
                    match parse_alert(&inner) {
                        Some(a) => {
                            parsed += 1;
                            println!("alert: {a:?}");
                        }
                        None => {
                            unparsed.insert(format!("{} {}", inner["type"], inner["data"]["__typename"]));
                        }
                    }
                } else {
                    feed += 1;
                    if let Some((who, n, text)) = parse_streak(&inner) {
                        streaks += 1;
                        println!("streak: {} {n} {text:?}", who.name);
                    }
                }
            }
            "subscribeResponse" if v["subscribeResponse"]["result"] != "ok" => {
                println!("subscribe {}: {}", v["parentId"], v["subscribeResponse"]);
            }
            _ => {}
        }
    }
    println!("alerts {alerts} parsed {parsed}; feed {feed} streaks {streaks}; skipped {unparsed:?}");
}
