//! What the miner reads from Hermes: the old PubSub messages of the user topics
//! (points, bets) and the channel topics (stream up and down, raids, moments, goals).

use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Points earned; `reason` is Twitch's code, e.g. `WATCH`, `CLAIM`, `WATCH_STREAK`.
    Earned {
        channel_id: String,
        balance: u64,
        gain: u64,
        reason: String,
    },
    Spent {
        channel_id: String,
        balance: u64,
    },
    /// C22 A bonus chest is ready.
    ClaimReady {
        channel_id: String,
        claim_id: String,
    },
    StreamUp,
    StreamDown,
    Viewers(u64),
    Raid {
        raid_id: String,
        target: String,
    },
    Moment {
        moment_id: String,
    },
    BetMade {
        channel_id: String,
        event_id: String,
    },
    /// `result` is `WIN`, `LOSE` or `REFUND`.
    BetResult {
        channel_id: String,
        event_id: String,
        result: String,
        won: u64,
    },
    /// W2 A goal started, changed or ended.
    Goals,
}

/// Reads one notification's payload. Unknown types are `None`.
pub fn parse(payload: &Value) -> Option<Event> {
    let kind = payload.get("type")?.as_str()?;
    let data = payload.get("data");
    let s = |v: Option<&Value>, path: &str| {
        v.and_then(|v| v.pointer(path))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    let n = |v: Option<&Value>, path: &str| v.and_then(|v| v.pointer(path)).and_then(Value::as_u64);
    Some(match kind {
        "points-earned" => Event::Earned {
            channel_id: s(data, "/balance/channel_id").or_else(|| s(data, "/channel_id"))?,
            balance: n(data, "/balance/balance")?,
            gain: n(data, "/point_gain/total_points").unwrap_or(0),
            reason: s(data, "/point_gain/reason_code").unwrap_or_default(),
        },
        "points-spent" => Event::Spent {
            channel_id: s(data, "/balance/channel_id")?,
            balance: n(data, "/balance/balance")?,
        },
        "claim-available" => Event::ClaimReady {
            channel_id: s(data, "/claim/channel_id")?,
            claim_id: s(data, "/claim/id")?,
        },
        "stream-up" => Event::StreamUp,
        "stream-down" => Event::StreamDown,
        "viewcount" => Event::Viewers(n(Some(payload), "/viewers").unwrap_or(0)),
        "raid_update_v2" => Event::Raid {
            raid_id: s(Some(payload), "/raid/id")?,
            target: s(Some(payload), "/raid/target_login")?,
        },
        "active" => Event::Moment {
            moment_id: s(data, "/moment_id")?,
        },
        "prediction-made" => Event::BetMade {
            channel_id: s(data, "/prediction/channel_id")?,
            event_id: s(data, "/prediction/event_id")?,
        },
        "prediction-result" => Event::BetResult {
            channel_id: s(data, "/prediction/channel_id")?,
            event_id: s(data, "/prediction/event_id")?,
            result: s(data, "/prediction/result/type")?,
            won: n(data, "/prediction/result/points_won").unwrap_or(0),
        },
        k if k.starts_with("community-goal-") => Event::Goals,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn points() {
        let earned = json!({"type":"points-earned","data":{"timestamp":"t","channel_id":"5",
            "point_gain":{"user_id":"1","channel_id":"5","total_points":12,"baseline_points":10,"reason_code":"WATCH"},
            "balance":{"user_id":"1","channel_id":"5","balance":1012}}});
        assert_eq!(
            parse(&earned),
            Some(Event::Earned {
                channel_id: "5".into(),
                balance: 1012,
                gain: 12,
                reason: "WATCH".into()
            })
        );
        let claim =
            json!({"type":"claim-available","data":{"claim":{"id":"c","channel_id":"5","point_gain":{}}}});
        assert_eq!(
            parse(&claim),
            Some(Event::ClaimReady {
                channel_id: "5".into(),
                claim_id: "c".into()
            })
        );
        let spent = json!({"type":"points-spent","data":{"balance":{"channel_id":"5","balance":3}}});
        assert!(matches!(parse(&spent), Some(Event::Spent { balance: 3, .. })));
    }

    #[test]
    fn channel_events() {
        assert_eq!(
            parse(&json!({"type":"stream-up","server_time":1.0})),
            Some(Event::StreamUp)
        );
        assert_eq!(
            parse(&json!({"type":"viewcount","viewers":31350})),
            Some(Event::Viewers(31350))
        );
        let raid = json!({"type":"raid_update_v2","raid":{"id":"r","target_login":"b","viewer_count":5}});
        assert_eq!(
            parse(&raid),
            Some(Event::Raid {
                raid_id: "r".into(),
                target: "b".into()
            })
        );
        let moment = json!({"type":"active","data":{"moment_id":"m","channel_id":"5"}});
        assert_eq!(
            parse(&moment),
            Some(Event::Moment {
                moment_id: "m".into()
            })
        );
        assert_eq!(
            parse(&json!({"type":"community-goal-updated","data":{}})),
            Some(Event::Goals)
        );
        assert_eq!(parse(&json!({"type":"commercial","length":180})), None);
    }

    #[test]
    fn bets() {
        let result = json!({"type":"prediction-result","data":{"prediction":{"event_id":"e","channel_id":"5",
            "result":{"type":"WIN","points_won":900}}}});
        assert_eq!(
            parse(&result),
            Some(Event::BetResult {
                channel_id: "5".into(),
                event_id: "e".into(),
                result: "WIN".into(),
                won: 900
            })
        );
        let lost = json!({"type":"prediction-result","data":{"prediction":{"event_id":"e","channel_id":"5",
            "result":{"type":"LOSE","points_won":null}}}});
        assert!(matches!(parse(&lost), Some(Event::BetResult { won: 0, .. })));
    }
}
