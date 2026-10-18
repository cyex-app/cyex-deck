//! 1.41 What GQL doesn't say about a Twitch emote: whether it's still active, and what
//! a bits emote costs. From potat, ivr when potat fails (not on a 404). Asked only when
//! a card opens, once per emote and day; never retried in a loop (25 calls a window).

use std::sync::LazyLock;
use std::time::Duration;

use cyex_twitch::Http;
use serde_json::Value;

use super::ttl::Ttl;

const POTAT: &str = "https://api.potat.app/twitch/emotes?id=";
const IVR: &str = "https://api.ivr.fi/v2/twitch/emotes/";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// "Inactive", "Archived", "Pending", "Deleted"; `None` when active or unknown.
    pub state: Option<String>,
    /// Bits cheered to unlock it.
    pub bit_cost: Option<u32>,
}

/// Successes and "no such emote", per emote id, for a day.
static CACHE: LazyLock<Ttl<String, Option<Facts>>> =
    LazyLock::new(|| Ttl::new(Duration::from_secs(24 * 60 * 60)));

#[derive(Debug, PartialEq, Eq)]
enum Answer {
    Found(Facts),
    Missing,
    Failed,
}

/// `None` when neither knows the emote or both failed.
pub async fn facts(http: &Http, id: &str) -> Option<Facts> {
    if let Some(hit) = CACHE.get(&id.to_owned()) {
        return hit;
    }
    let enc: String = url::form_urlencoded::byte_serialize(id.as_bytes()).collect();
    let mut got = answer(http.get_json::<Value>(&format!("{POTAT}{enc}")).await);
    if got == Answer::Failed {
        got = answer(http.get_json::<Value>(&format!("{IVR}{enc}?id=true")).await);
    }
    let out = match got {
        Answer::Found(f) => Some(f),
        Answer::Missing => None,
        Answer::Failed => return None,
    };
    CACHE.put(id.to_owned(), out.clone());
    out
}

fn answer(r: cyex_twitch::Result<Value>) -> Answer {
    match r {
        Ok(v) => parse(&v),
        Err(e) if e.status() == Some(404) => Answer::Missing,
        Err(_) => Answer::Failed,
    }
}

/// potat wraps it (`{"data":[{...}],"statusCode":200}`), ivr doesn't (`{...}`).
fn parse(v: &Value) -> Answer {
    if v.get("statusCode").and_then(Value::as_u64) == Some(404) {
        return Answer::Missing;
    }
    let emote = match v.get("data") {
        Some(Value::Array(list)) => match list.first() {
            Some(e) if e.is_object() => e,
            _ => return Answer::Missing,
        },
        Some(Value::Object(_)) => &v["data"],
        _ => v,
    };
    const KEYS: [&str; 4] = ["emoteID", "emoteCode", "emoteState", "emoteBitCost"];
    if !KEYS.iter().any(|k| emote.get(k).is_some_and(|x| !x.is_null())) {
        return Answer::Failed;
    }
    Answer::Found(Facts {
        state: emote
            .get("emoteState")
            .and_then(Value::as_str)
            .and_then(state_label),
        bit_cost: emote.get("emoteBitCost").and_then(number).filter(|n| *n > 0),
    })
}

fn state_label(s: &str) -> Option<String> {
    let label = match s.to_ascii_uppercase().as_str() {
        "INACTIVE" => "Inactive",
        "ARCHIVED" => "Archived",
        "PENDING" => "Pending",
        "DELETED" => "Deleted",
        _ => return None,
    };
    Some(label.to_owned())
}

fn number(v: &Value) -> Option<u32> {
    match v {
        Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(s: &str) -> Value {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn potat_shape() {
        let v = json(
            r#"{"data":[{"emoteID":"25","emoteCode":"Kappa","emotePrefix":"","emoteSuffix":"Kappa",
            "emoteURL":"https://static-cdn.jtvnw.net/emoticons/v2/25/default/dark/3.0",
            "emoteSetID":"0","emoteAssetType":"STATIC","emoteState":"ACTIVE","emoteType":"GLOBALS"}],
            "duration":282.0475,"statusCode":200}"#,
        );
        assert_eq!(parse(&v), Answer::Found(Facts::default()));
    }

    #[test]
    fn ivr_shape_with_state_and_cost() {
        let v = json(
            r#"{"emoteID":"emotesv2_abc","emoteCode":"xqcCheer","emoteState":"ARCHIVED",
            "emoteType":"BITS_BADGE_TIERS","emoteBitCost":1000,"channelLogin":"xqc"}"#,
        );
        assert_eq!(
            parse(&v),
            Answer::Found(Facts {
                state: Some("Archived".into()),
                bit_cost: Some(1000),
            })
        );
        let v = json(r#"{"emoteCode":"x","emoteState":"PENDING","emoteBitCost":"500"}"#);
        assert_eq!(
            parse(&v),
            Answer::Found(Facts {
                state: Some("Pending".into()),
                bit_cost: Some(500),
            })
        );
    }

    #[test]
    fn missing_fields_are_fine() {
        assert_eq!(
            parse(&json(r#"{"emoteID":"1"}"#)),
            Answer::Found(Facts::default())
        );
        let v = json(r#"{"emoteID":"1","emoteState":null,"emoteBitCost":0}"#);
        assert_eq!(parse(&v), Answer::Found(Facts::default()));
    }

    #[test]
    fn unknown_and_odd_answers() {
        assert_eq!(parse(&json(r#"{"data":[],"statusCode":200}"#)), Answer::Missing);
        assert_eq!(parse(&json(r#"{"data":[null]}"#)), Answer::Missing);
        assert_eq!(
            parse(&json(r#"{"statusCode":404,"error":"nope"}"#)),
            Answer::Missing
        );
        assert_eq!(parse(&json(r#"{"something":"else"}"#)), Answer::Failed);
        assert_eq!(parse(&json("[]")), Answer::Failed);
    }

    #[test]
    fn errors_fall_back_but_404_is_final() {
        let api = |status| cyex_twitch::Error::Api {
            status,
            message: String::new(),
        };
        assert_eq!(answer(Err(api(404))), Answer::Missing);
        assert_eq!(answer(Err(api(429))), Answer::Failed);
        assert_eq!(answer(Err(api(500))), Answer::Failed);
    }

    #[test]
    fn states() {
        assert_eq!(state_label("ACTIVE"), None);
        assert_eq!(state_label("INACTIVE").as_deref(), Some("Inactive"));
        assert_eq!(state_label("deleted").as_deref(), Some("Deleted"));
        assert_eq!(state_label("NEW"), None);
    }
}
