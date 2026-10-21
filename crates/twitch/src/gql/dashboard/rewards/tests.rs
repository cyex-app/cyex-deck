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
fn a_saved_reward() {
    assert_eq!(read_saved(json!({ "error": null, "reward": raw_reward("n", "POINTS") })).unwrap().id, "n");
    let e = read_saved(json!({ "error": null, "reward": null })).unwrap_err();
    assert!(matches!(e, Error::Refused(c) if c == "NO_REWARD"));
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

use crate::gql::dashboard::NOT_DONE;
use crate::gql::dashboard::mock::{ID, off, query, serve};

fn refused<T: std::fmt::Debug>(r: Result<T>) -> String {
    match r {
        Err(Error::Refused(c)) => c,
        other => panic!("not a refusal: {other:?}"),
    }
}

fn data(field: &str, payload: Value) -> String {
    json!({ "data": { field: payload } }).to_string()
}

#[tokio::test]
async fn v107_r41_create_sends_the_input_and_reads_errors() {
    let m = serve([
        data(CREATED, json!({ "error": null, "reward": raw_reward("new", "POINTS") })),
        data(CREATED, json!({ "error": { "code": "DUPLICATE_REWARD" }, "reward": null })),
        data(CREATED, Value::Null),
    ]);
    let ch = m.channel();
    assert_eq!(create(&ch, &edit()).await.unwrap().id, "new");
    assert_eq!(refused(create(&ch, &edit()).await), "DUPLICATE_REWARD");
    assert_eq!(refused(create(&ch, &edit()).await), NOT_DONE);
    let sent = m.sent();
    assert!(query(&sent[0]).contains("createCommunityPointsCustomReward(input: $input)"));
    assert_eq!(sent[0]["variables"], json!({ "input": edit_input(ID, None, &edit()) }));
}

#[tokio::test]
async fn v107_r41_update_and_switch_send_their_inputs() {
    let mut paused = raw_reward("r", "POINTS");
    paused["isPaused"] = true.into();
    let m = serve([
        data(UPDATED, json!({ "error": null, "reward": raw_reward("r", "POINTS") })),
        data(UPDATED, json!({ "error": null, "reward": paused })),
        data(UPDATED, Value::Null),
        json!({ "data": null, "errors": [{ "message": "service error" }] }).to_string(),
    ]);
    let ch = m.channel();
    assert_eq!(update(&ch, "r", &edit()).await.unwrap().id, "r");
    assert!(switch(&ch, "r", Switch::Paused(true)).await.unwrap().paused);
    assert_eq!(refused(switch(&ch, "r", Switch::Enabled(false)).await), NOT_DONE);
    assert_eq!(switch(&ch, "r", Switch::Enabled(true)).await.unwrap_err().api_message(), Some("service error"));
    let sent = m.sent();
    assert!(query(&sent[0]).contains("updateCommunityPointsCustomReward(input: $input)"));
    assert_eq!(sent[0]["variables"], json!({ "input": edit_input(ID, Some("r"), &edit()) }));
    assert_eq!(sent[1]["variables"], json!({ "input": { "channelID": ID, "rewardID": "r", "isPaused": true } }));
    assert_eq!(sent[2]["variables"], json!({ "input": { "channelID": ID, "rewardID": "r", "isEnabled": false } }));
}

#[tokio::test]
async fn v107_r41_delete_sends_what_happens_to_the_waiting() {
    let m = serve([
        data(DELETED, json!({ "error": null, "reward": { "id": "r" } })),
        data(DELETED, Value::Null),
        data(DELETED, json!({ "error": { "code": "NOT_FOUND" } })),
    ]);
    let ch = m.channel();
    delete(&ch, "r", Some(Status::Fulfilled)).await.unwrap();
    assert_eq!(refused(delete(&ch, "r", None).await), NOT_DONE);
    assert_eq!(refused(delete(&ch, "r", None).await), "NOT_FOUND");
    let sent = m.sent();
    assert!(query(&sent[0]).contains("deleteCommunityPointsCustomReward(input: $input)"));
    assert_eq!(
        sent[0]["variables"],
        json!({ "input": { "channelID": ID, "rewardID": "r", "unfulfilledRedemptionsNewStatus": "FULFILLED" } })
    );
    assert_eq!(sent[1]["variables"], json!({ "input": { "channelID": ID, "rewardID": "r" } }));
}

#[tokio::test]
async fn v107_r41_status_sends_ids_and_takes_null_as_done() {
    let m = serve([
        data(STATUSES, json!({ "error": null })),
        data(STATUSES, Value::Null),
        data(STATUSES, json!({ "error": { "code": "FORBIDDEN" } })),
    ]);
    let ch = m.channel();
    let ids = vec!["a".to_owned(), "b".to_owned()];
    set_status(&ch, &ids, Status::Canceled).await.unwrap();
    set_status(&ch, &ids, Status::Fulfilled).await.unwrap();
    assert_eq!(refused(set_status(&ch, &ids, Status::Canceled).await), "FORBIDDEN");
    // Over Twitch's 50 never leaves the app.
    let many: Vec<String> = (0..=STATUS_MAX).map(|i| i.to_string()).collect();
    assert_eq!(refused(set_status(&off(), &many, Status::Canceled).await), "TOO_MANY");
    let sent = m.sent();
    assert!(query(&sent[0]).contains("updateCommunityPointsCustomRewardRedemptionStatusesByRedemptions(input: $input)"));
    assert_eq!(
        sent[0]["variables"],
        json!({ "input": { "channelID": ID, "redemptionIDs": ["a", "b"], "newStatus": "CANCELED", "oldStatus": "UNFULFILLED" } })
    );
}

#[tokio::test]
async fn v107_r41_list_and_queue_ask_for_the_channel() {
    let m = serve([
        json!({ "data": { "user": { "channel": { "communityPointsSettings": null } } } }).to_string(),
        json!({ "data": { "user": null } }).to_string(),
    ]);
    let ch = m.channel();
    assert_eq!(list(&ch).await.unwrap(), Rewards::default());
    assert_eq!(queue(&ch, Some("a"), Some("c1")).await.unwrap(), Queue::default());
    let sent = m.sent();
    assert_eq!(sent[0]["variables"], json!({ "id": ID }));
    assert_eq!(sent[1]["variables"], json!({ "id": ID, "reward": "a", "cursor": "c1", "count": QUEUE_PAGE }));
}

#[test]
fn v107_r43_bits_redemptions_are_left_out() {
    let node = |id: &str, pricing: Option<&str>| {
        let mut r = json!({ "id": "a", "title": "Song", "cost": 100 });
        if let Some(p) = pricing {
            r["pricingType"] = p.into();
        }
        json!({ "cursor": id, "node": { "id": id, "timestamp": "2026-10-21T10:00:00Z", "reward": r, "user": null } })
    };
    let data = json!({ "user": { "channel": { "communityPointsRedemptionQueue": {
        "edges": [node("p", Some("POINTS")), node("b", Some("BITS")), node("old", None)],
        "pageInfo": { "hasNextPage": true }
    } } } });
    let q = read_queue(&data).unwrap();
    let ids: Vec<&str> = q.items.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["p", "old"], "no pricing type: points, as before");
    assert_eq!(q.next.as_deref(), Some("old"), "the cursor is the page's, bits ones included");
}

#[tokio::test]
async fn v107_r43_a_moderator_works_on_the_channel_it_moderates() {
    // The account (a mod) isn't the channel: every call names the moderated channel's id.
    let m = serve([
        json!({ "data": { "user": { "channel": { "communityPointsRedemptionQueue": { "edges": [], "pageInfo": { "hasNextPage": false } } } } } }).to_string(),
        data(STATUSES, json!({ "error": null })),
    ]);
    let ch = Channel { id: "77", ..m.channel() };
    assert_eq!(queue(&ch, None, None).await.unwrap(), Queue::default());
    set_status(&ch, &["x".to_owned()], Status::Canceled).await.unwrap();
    let sent = m.sent();
    assert!(query(&sent[0]).contains("pricingType"));
    assert_eq!(sent[0]["variables"]["id"], "77");
    assert_eq!(sent[1]["variables"]["input"]["channelID"], "77");
}
