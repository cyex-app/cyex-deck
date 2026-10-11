//! 1.35 The same activity as EventSub's, from Twitch's dashboard topics on Hermes (D30):
//! `activity-feed-alerts-v2.<channel>` carries structured alerts in GQL shape, and
//! `activity-feed-broadcaster-v2.<channel>.<channel>` carries the feed's own lines, used
//! here only for watch streaks, which never come as alerts.

use serde_json::Value;

use crate::events::{Activity, Who};

/// A Hermes message on the alerts topic. Only new alerts count; updates are replays and skips.
pub fn parse_alert(msg: &Value) -> Option<Activity> {
    if msg.get("type")?.as_str()? != "activity_feed_alerts_create" {
        return None;
    }
    let d = msg.get("data")?;
    let who = |k: &str| user(d.get(k)?);
    let num = |k: &str| d.get(k).and_then(Value::as_u64);
    let tier = || tier(d.get("tier").and_then(Value::as_str));
    let anonymous = d.get("isAnonymous").and_then(Value::as_bool).unwrap_or(false);
    Some(match d.get("__typename")?.as_str()? {
        "ActivityFeedFollowAlert" => Activity::Follow {
            user: who("follower")?,
            at: str_of(d, "createdAt"),
        },
        "ActivityFeedSubscriptionAlert" => Activity::Subscribe {
            user: who("subscriber")?,
            tier: tier(),
            gift: false,
        },
        "ActivityFeedPrimeSubscriptionAlert" => Activity::Subscribe {
            user: who("subscriber")?,
            tier: "1000".into(),
            gift: false,
        },
        "ActivityFeedResubscriptionAlert" | "ActivityFeedPrimeResubscriptionAlert" => Activity::Resub {
            user: who("subscriber")?,
            tier: tier(),
            months: num("totalDuration").unwrap_or(0),
            streak: num("streakDuration").filter(|s| *s > 0),
            text: message(d.get("messageContent")),
        },
        "ActivityFeedIndividualGiftSubscriptionAlert" => Activity::Subscribe {
            user: who("recipient")?,
            tier: tier(),
            gift: true,
        },
        "ActivityFeedCommunityGiftSubscriptionAlert" => Activity::SubGift {
            user: who("gifter").filter(|_| !anonymous),
            tier: tier(),
            total: num("quantity").unwrap_or(1),
        },
        "ActivityFeedCheerAlert" => Activity::Cheer {
            user: who("cheerer").filter(|_| !anonymous),
            bits: num("amount").unwrap_or(0),
            text: message(d.get("messageContent")),
        },
        // Cheers come this way now; power-ups spend Bits too but aren't cheers.
        "ActivityFeedBitsSpendAlert" if str_of(d, "featureType") == "CHEER" => Activity::Cheer {
            user: who("fromUser"),
            bits: num("bitsAmount").unwrap_or(0),
            text: message(d.get("featureData").and_then(|f| f.get("messageContent"))),
        },
        "ActivityFeedRaidAlert" => Activity::Raid {
            from: who("raider")?,
            viewers: num("partySize").unwrap_or(0),
        },
        "ActivityFeedChannelPointsRedemptionAlert" => Activity::Redemption {
            user: who("redeemer")?,
            reward: d
                .get("reward")
                .and_then(|r| r.get("title"))
                .and_then(Value::as_str)?
                .to_owned(),
            cost: num("pointsSpent").unwrap_or(0),
            input: message(d.get("messageContent")),
        },
        _ => return None,
    })
}

/// A watch streak from the broadcaster's feed topic: who, how many streams, their message.
pub fn parse_streak(msg: &Value) -> Option<(Who, u64, String)> {
    if msg.get("type")?.as_str()? != "activity_feed_event_create" {
        return None;
    }
    let d = msg.get("data")?;
    if d.get("type")?.as_str()? != "WATCH_STREAK" {
        return None;
    }
    // The viewer is only in the title; the count is the content's first number.
    let who = tokens(d.get("title")?)?.iter().find_map(|t| {
        (t.get("type")?.as_str()? == "USER")
            .then(|| user(t.get("token")?))
            .flatten()
    })?;
    let streams = tokens(d.get("content")?)?.iter().find_map(|t| {
        (t.get("type")?.as_str()? == "INTEGER_NUMBER")
            .then(|| t.get("token")?.as_u64())
            .flatten()
    })?;
    let text = d
        .get("user_content")
        .and_then(Value::as_array)
        .map(|ts| ts.iter().filter_map(token_text).collect::<String>())
        .unwrap_or_default();
    Some((who, streams, text.trim().to_owned()))
}

/// The English tokens of a localized line (`{"en-US": [...]}`), else any locale's.
fn tokens(v: &Value) -> Option<&Vec<Value>> {
    let map = v.as_object()?;
    map.get("en-US").or_else(|| map.values().next())?.as_array()
}

fn token_text(t: &Value) -> Option<String> {
    let token = t.get("token")?;
    Some(match t.get("type")?.as_str()? {
        "USER" => user(token)?.name,
        "CHEERMOTE" => token.get("token").and_then(Value::as_str)?.to_owned(),
        _ => match token {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => return None,
        },
    })
}

/// GQL users carry `userID` (or `id`), `login` and `displayName`; feed tokens use snake case.
fn user(v: &Value) -> Option<Who> {
    let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
    let id = s("userID").or_else(|| s("id"))?;
    let login = s("login").unwrap_or_default();
    let name = s("displayName").or_else(|| s("display_name")).unwrap_or_default();
    Some(Who { id, login, name })
}

fn str_of(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// `T_2000` → `2000`; Prime and missing read as tier 1.
fn tier(t: Option<&str>) -> String {
    match t.and_then(|t| t.strip_prefix("T_")) {
        Some(n @ ("2000" | "3000")) => n.into(),
        _ => "1000".into(),
    }
}

/// The text of an alert's `messageContent`: text pieces, cheermotes as `cheer100`.
/// Emotes come only as ids, so they're left out.
fn message(content: Option<&Value>) -> String {
    let Some(frags) = content.and_then(|c| c.get("fragments")).and_then(Value::as_array) else {
        return String::new();
    };
    let mut out = String::new();
    for f in frags {
        if let Some(t) = f.get("text").and_then(Value::as_str) {
            out.push_str(t);
        } else if let Some(c) = f.get("cheermote") {
            let prefix = c.get("prefix").and_then(Value::as_str).unwrap_or("cheer");
            let bits = c.get("bitsAmount").and_then(Value::as_u64).unwrap_or(0);
            out.push_str(&format!("{prefix}{bits}"));
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn alert(data: Value) -> Option<Activity> {
        parse_alert(&json!({ "type": "activity_feed_alerts_create", "data": data }))
    }

    fn u(name: &str) -> Value {
        json!({ "__typename": "User", "userID": "1", "displayName": name, "login": name.to_lowercase() })
    }

    #[test]
    fn alerts_become_activity() {
        let a = alert(
            json!({ "__typename": "ActivityFeedFollowAlert", "createdAt": "t", "follower": u("Kuni") }),
        );
        assert!(
            matches!(a, Some(Activity::Follow { user, .. }) if user.name == "Kuni" && user.login == "kuni")
        );

        let a = alert(
            json!({ "__typename": "ActivityFeedResubscriptionAlert", "subscriber": u("Kuni"),
            "totalDuration": 19, "streakDuration": 0, "tier": "T_2000",
            "messageContent": { "fragments": [{ "text": "hi " }, { "emote": { "id": "1" } }, { "text": " there" }] } }),
        );
        assert_eq!(
            a,
            Some(Activity::Resub {
                user: Who {
                    id: "1".into(),
                    login: "kuni".into(),
                    name: "Kuni".into()
                },
                tier: "2000".into(),
                months: 19,
                streak: None,
                text: "hi there".into(),
            })
        );

        let a = alert(
            json!({ "__typename": "ActivityFeedCommunityGiftSubscriptionAlert", "gifter": u("G"),
            "isAnonymous": true, "quantity": 5, "tier": "T_1000" }),
        );
        assert!(matches!(
            a,
            Some(Activity::SubGift {
                user: None,
                total: 5,
                ..
            })
        ));

        let a = alert(
            json!({ "__typename": "ActivityFeedBitsSpendAlert", "fromUser": u("C"), "bitsAmount": 100,
            "featureType": "CHEER", "featureData": { "messageContent": { "fragments": [
                { "cheermote": { "prefix": "cheer", "bitsAmount": 100, "tier": 100 } }, { "text": " omg" }] } } }),
        );
        assert!(matches!(a, Some(Activity::Cheer { bits: 100, ref text, .. }) if text == "cheer100 omg"));

        let power_up = alert(
            json!({ "__typename": "ActivityFeedBitsSpendAlert", "fromUser": u("C"), "bitsAmount": 20,
            "featureType": "POWER_UP" }),
        );
        assert_eq!(power_up, None);

        let a = alert(
            json!({ "__typename": "ActivityFeedChannelPointsRedemptionAlert", "redeemer": u("R"),
            "pointsSpent": 5000, "reward": { "id": "x", "title": "Hydrate" }, "messageContent": { "fragments": [] } }),
        );
        assert!(
            matches!(a, Some(Activity::Redemption { cost: 5000, ref reward, .. }) if reward == "Hydrate")
        );

        let a = alert(json!({ "__typename": "ActivityFeedRaidAlert", "raider": u("R"), "partySize": 12 }));
        assert!(matches!(a, Some(Activity::Raid { viewers: 12, .. })));

        // Replays and skips are updates, not new activity.
        let update = parse_alert(&json!({ "type": "activity_feed_alerts_update",
            "data": { "__typename": "ActivityFeedRaidAlert", "raider": u("R"), "partySize": 1 } }));
        assert_eq!(update, None);
    }

    #[test]
    fn watch_streaks_read_from_the_feed() {
        let msg = json!({ "type": "activity_feed_event_create", "data": {
            "type": "WATCH_STREAK",
            "title": { "de-DE": [], "en-US": [{ "type": "USER", "token": { "id": "9", "display_name": "Kuni", "login": "kuni" } }] },
            "content": { "en-US": [{ "type": "TEXT", "token": "Reached " }, { "type": "INTEGER_NUMBER", "token": 50 },
                { "type": "TEXT", "token": "-Stream Streak" }] },
            "user_content": [{ "type": "TEXT", "token": "love it" }] } });
        let (who, n, text) = parse_streak(&msg).unwrap();
        assert_eq!((who.name.as_str(), n, text.as_str()), ("Kuni", 50, "love it"));
        let follow = json!({ "type": "activity_feed_event_create", "data": { "type": "FOLLOW" } });
        assert_eq!(parse_streak(&follow), None);
    }
}
