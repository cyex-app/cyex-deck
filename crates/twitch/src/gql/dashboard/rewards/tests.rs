use super::*;

fn raw_reward(id: &str, pricing: &str) -> Value {
    json!({
        "id": id, "title": "Hydrate", "prompt": "", "cost": 500, "pricingType": pricing,
        "backgroundColor": "#00c7ac", "isEnabled": true, "isPaused": false, "isInStock": true,
        "isSubOnly": false, "isUserInputRequired": true, "shouldRedemptionsSkipRequestQueue": false,
        "cooldownExpiresAt": null, "redemptionsRedeemedCurrentStream": 3,
        "image": null, "defaultImage": { "url": "https://x/1.png", "url2x": "https://x/2.png" },
        "maxPerStreamSetting": { "isEnabled": true, "maxPerStream": 10 },
        "maxPerUserPerStreamSetting": { "isEnabled": false, "maxPerUserPerStream": 0 },
        "globalCooldownSetting": { "isEnabled": true, "globalCooldownSeconds": 300 }
    })
}

#[test]
fn rewards_read_with_limits_and_waiting_counts() {
    let data = json!({ "user": { "channel": { "communityPointsSettings": {
        "isEnabled": true,
        "customRewards": [raw_reward("a", "POINTS"), raw_reward("b", "BITS")],
        "summarizedRewards": [{ "count": 7, "isCountAtMaximum": false, "node": { "id": "a" } }]
    } } } });
    let r = read_rewards(&data).unwrap();
    assert!(r.enabled);
    assert_eq!(r.rewards.len(), 1, "Bits rewards are power-ups, not listed");
    let a = &r.rewards[0];
    assert_eq!(a.color, "#00C7AC");
    assert_eq!(a.image.as_deref(), Some("https://x/2.png"));
    assert_eq!(
        (a.cooldown, a.max_per_stream, a.max_per_user),
        (Some(300), Some(10), None)
    );
    assert_eq!((a.waiting, a.redeemed_this_stream), (7, Some(3)));
    assert!(a.input_required && !a.skip_queue && a.prompt.is_empty());
}

#[test]
fn no_points_settings_is_an_empty_list() {
    let r = read_rewards(&json!({ "user": { "channel": { "communityPointsSettings": null } } })).unwrap();
    assert_eq!(r, Rewards::default());
    assert_eq!(
        read_rewards(&json!({ "user": null })).unwrap(),
        Rewards::default()
    );
}

#[test]
fn a_saved_reward_or_its_refusal() {
    let ok = json!({ "createCommunityPointsCustomReward": { "error": null, "reward": raw_reward("n", "POINTS") } });
    assert_eq!(
        read_saved(&ok, "createCommunityPointsCustomReward").unwrap().id,
        "n"
    );
    let no = json!({ "createCommunityPointsCustomReward": { "error": { "code": "DUPLICATE_REWARD" }, "reward": null } });
    let e = read_saved(&no, "createCommunityPointsCustomReward").unwrap_err();
    assert!(matches!(e, Error::Refused(c) if c == "DUPLICATE_REWARD"));
}

#[test]
fn queue_reads_newest_first_with_a_cursor_when_more() {
    let data = json!({ "user": { "channel": { "communityPointsRedemptionQueue": {
        "edges": [
            { "cursor": "c1", "node": { "id": "r1", "input": "play x", "timestamp": "2026-10-21T10:00:00Z",
              "reward": { "id": "a", "title": "Song", "cost": 100 },
              "user": { "id": "1", "login": "cyexbot", "displayName": "CyexBot" } } },
            { "cursor": "c2", "node": { "id": "r2", "input": " ", "timestamp": "2026-10-21T09:00:00Z",
              "reward": { "id": "a", "title": "Song", "cost": 100 }, "user": null } }
        ],
        "pageInfo": { "hasNextPage": true }
    } } } });
    let q = read_queue(&data).unwrap();
    assert_eq!(q.next.as_deref(), Some("c2"));
    assert_eq!(q.items[0].login.as_deref(), Some("cyexbot"));
    assert_eq!(q.items[0].input.as_deref(), Some("play x"));
    assert_eq!(q.items[1].input, None, "blank input is no input");
    assert_eq!(q.items[1].name, None);
    let last = json!({ "user": { "channel": { "communityPointsRedemptionQueue": { "edges": [], "pageInfo": { "hasNextPage": false } } } } });
    assert_eq!(read_queue(&last).unwrap(), Queue::default());
}

fn edit() -> RewardEdit {
    RewardEdit {
        title: " Test ".into(),
        prompt: "".into(),
        cost: 1,
        color: "#145aff".into(),
        enabled: true,
        input_required: false,
        skip_queue: true,
        cooldown: Some(60),
        max_per_stream: None,
        max_per_user: Some(2),
    }
}

#[test]
fn inputs_match_twitchs_dashboard() {
    let c = edit_input("9", None, &edit());
    assert_eq!(c["title"], "Test");
    assert_eq!(c["prompt"], Value::Null);
    assert_eq!(c["backgroundColor"], "#145AFF");
    assert_eq!(c["pricingType"], "POINTS");
    assert_eq!(c["isSubOnly"], false);
    assert!(c.get("rewardID").is_none() && c.get("isEnabled").is_none());
    assert_eq!(
        c["globalCooldownSetting"],
        json!({ "isEnabled": true, "globalCooldownSeconds": 60 })
    );
    assert_eq!(
        c["maxPerStreamSetting"],
        json!({ "isEnabled": false, "maxPerStream": 0 })
    );
    assert_eq!(
        c["maxPerUserPerStreamSetting"],
        json!({ "isEnabled": true, "maxPerUserPerStream": 2 })
    );
    let u = edit_input("9", Some("r"), &edit());
    assert_eq!((&u["rewardID"], &u["isEnabled"]), (&json!("r"), &json!(true)));
    assert!(u.get("pricingType").is_none() && u.get("isSubOnly").is_none());
    assert_eq!(
        switch_input("9", "r", Switch::Paused(true)),
        json!({ "channelID": "9", "rewardID": "r", "isPaused": true })
    );
    assert_eq!(
        delete_input("9", "r", None),
        json!({ "channelID": "9", "rewardID": "r" })
    );
    assert_eq!(
        delete_input("9", "r", Some(Status::Canceled))["unfulfilledRedemptionsNewStatus"],
        "CANCELED"
    );
    let s = status_input("9", &["a".into(), "b".into()], Status::Fulfilled);
    assert_eq!(
        s,
        json!({ "channelID": "9", "redemptionIDs": ["a", "b"], "newStatus": "FULFILLED", "oldStatus": "UNFULFILLED" })
    );
}

#[test]
fn switches_come_from_the_ui_tagged() {
    let s: Switch = serde_json::from_value(json!({ "kind": "paused", "on": true })).unwrap();
    assert_eq!(s, Switch::Paused(true));
    let st: Status = serde_json::from_value(json!("CANCELED")).unwrap();
    assert_eq!(st, Status::Canceled);
}

/// A local GQL that answers each request with the next body and keeps what it was sent.
fn mock(bodies: Vec<String>) -> (String, std::thread::JoinHandle<Vec<Value>>) {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let h = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for b in bodies {
            let (mut s, _) = l.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            let body = loop {
                let n = s.read(&mut buf).unwrap();
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got).to_string();
                if let Some(i) = text.find("\r\n\r\n") {
                    let len = text[..i]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if got.len() >= i + 4 + len {
                        break got[i + 4..i + 4 + len].to_vec();
                    }
                }
                if n == 0 {
                    break Vec::new();
                }
            };
            seen.push(serde_json::from_slice(&body).unwrap());
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                b.len()
            );
            s.write_all(head.as_bytes()).unwrap();
            s.write_all(b.as_bytes()).unwrap();
        }
        seen
    });
    (base, h)
}

#[tokio::test]
async fn create_pause_and_complete_against_a_mock() {
    let reward = raw_reward("new", "POINTS");
    let mut paused = raw_reward("new", "POINTS");
    paused["isPaused"] = true.into();
    let (base, h) = mock(vec![
        json!({ "data": { "createCommunityPointsCustomReward": { "error": null, "reward": reward } } }).to_string(),
        json!({ "data": { "updateCommunityPointsCustomReward": { "error": null, "reward": paused } } }).to_string(),
        json!({ "data": { "updateCommunityPointsCustomRewardRedemptionStatusesByRedemptions": { "error": { "code": "FORBIDDEN" } } } })
            .to_string(),
        json!({ "data": null, "errors": [{ "message": "service error" }] }).to_string(),
    ]);
    let http = Http::new().unwrap();
    let auth = Auth {
        client_id: "c",
        token: Some("t"),
    };
    let b = Some(base.as_str());
    assert_eq!(create(&http, auth, b, "9", &edit()).await.unwrap().id, "new");
    assert!(
        switch(&http, auth, b, "9", "new", Switch::Paused(true))
            .await
            .unwrap()
            .paused
    );
    let e = set_status(&http, auth, b, "9", &["r1".into()], Status::Canceled)
        .await
        .unwrap_err();
    assert!(matches!(e, Error::Refused(c) if c == "FORBIDDEN"));
    assert!(matches!(
        list(&http, auth, b, "9").await.unwrap_err(),
        Error::Api { .. }
    ));
    let seen = h.join().unwrap();
    assert!(
        seen[0]["query"]
            .as_str()
            .unwrap()
            .contains("createCommunityPointsCustomReward")
    );
    assert_eq!(seen[0]["variables"]["input"]["cost"], 1);
    assert_eq!(
        seen[1]["variables"]["input"],
        json!({ "channelID": "9", "rewardID": "new", "isPaused": true })
    );
    assert_eq!(seen[2]["variables"]["input"]["newStatus"], "CANCELED");
    assert_eq!(seen[3]["variables"], json!({ "id": "9" }));
}
