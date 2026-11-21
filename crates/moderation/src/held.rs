//! V108-N15 / V112-28 Why AutoMod held a message, as one wire string for
//! `ui/src/features/feeds/heldReason.ts`, from either feed:
//! `automod[:<category>[:<level>]]`, `blocked_term[:<term>\x1f<term>…]` or `blocked_link`.
//! EventSub's `automod.message.hold` v2 carries `reason` and `blocked_term.terms_found[]`;
//! Hermes' `automod-queue` carries `caught_message_reason` (Twitch's own client reads it).
//! Neither sends a term's text: it is cut from the message by its boundary. Private terms
//! (owner-only on Twitch) stay unnamed.

use cyex_eventsub::events::{Boundary, Held};
use serde_json::Value;

/// Between terms on the wire. Never in a term people type.
pub const SEP: char = '\u{1f}';
/// At most this many terms on the wire; the UI shows fewer.
const MAX_TERMS: usize = 8;

/// `text`'s code points `start..=end`, trimmed; none if out of range or blank.
pub fn cut(text: &str, start: u32, end: u32) -> Option<String> {
    let n = end.checked_sub(start)? as usize + 1;
    let t: String = text.chars().skip(start as usize).take(n).collect();
    let t = t.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

/// The wire form of a blocked-term hold: each term once (case aside), in message order.
pub fn blocked_term(terms: impl IntoIterator<Item = String>) -> String {
    let mut seen: Vec<String> = Vec::new();
    for t in terms {
        let t = t.replace(SEP, " ");
        let t = t.trim();
        if t.is_empty() || seen.iter().any(|s| s.eq_ignore_ascii_case(t)) {
            continue;
        }
        seen.push(t.to_owned());
        if seen.len() == MAX_TERMS {
            break;
        }
    }
    if seen.is_empty() {
        return "blocked_term".into();
    }
    let joined: Vec<&str> = seen.iter().map(String::as_str).collect();
    format!("blocked_term:{}", joined.join(&SEP.to_string()))
}

/// An AutoMod category hold. `unknown` or none: plain AutoMod.
pub fn automod(category: Option<&str>, level: Option<i64>) -> String {
    match (category.map(str::trim).filter(|c| !c.is_empty() && !c.eq_ignore_ascii_case("unknown")), level.filter(|l| *l > 0)) {
        (Some(c), Some(l)) => format!("automod:{c}:{l}"),
        (Some(c), None) => format!("automod:{c}"),
        _ => "AutoMod".into(),
    }
}

fn cut_all(text: &str, bounds: &[Boundary]) -> Vec<String> {
    let mut b = bounds.to_vec();
    b.sort_by_key(|b| b.start_pos);
    b.iter().filter_map(|b| cut(text, b.start_pos, b.end_pos)).collect()
}

/// EventSub `automod.message.hold` v2.
pub fn eventsub(h: &Held) -> String {
    match h.reason.as_str() {
        "blocked_term" => blocked_term(cut_all(&h.message.text, &h.boundaries)),
        "blocked_link" => "blocked_link".into(),
        _ => automod(h.category.as_deref(), h.level.map(i64::from)),
    }
}

/// Hermes `automod_caught_message` `data`: `caught_message_reason.reason` is
/// `AutoModCaughtMessageReason`, `BlockedTermCaughtMessageReason`, `LinkCaughtMessageReason`
/// (and Baseline / HighNoise, read as AutoMod). Without it (older shapes): the classification.
pub fn hermes(data: &Value, text: &str) -> String {
    let why = data.get("caught_message_reason").filter(|v| v.is_object());
    let kind = why.and_then(|w| w.get("reason")).and_then(Value::as_str).unwrap_or_default();
    if kind.contains("BlockedTerm") {
        let found = why
            .and_then(|w| w.pointer("/blocked_term_failure/terms_found"))
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let mut terms: Vec<(u32, String)> = found.iter().filter_map(|t| hermes_term(t, text)).collect();
        terms.sort_by_key(|(at, _)| *at);
        return blocked_term(terms.into_iter().map(|(_, t)| t));
    }
    if kind.contains("Link") {
        return "blocked_link".into();
    }
    let num = |v: Option<&Value>| v.and_then(Value::as_i64);
    let failure = why.and_then(|w| w.get("automod_failure")).filter(|v| v.is_object());
    let class = data.get("content_classification");
    let category = failure
        .and_then(|f| f.get("category")?.as_str())
        .filter(|c| !c.is_empty())
        .or_else(|| class.and_then(|c| c.get("category")?.as_str()));
    let level = num(failure.and_then(|f| f.get("level"))).or_else(|| num(class.and_then(|c| c.get("level"))));
    automod(category, level)
}

/// One `terms_found` entry: its text if it has one, else cut from the message by
/// `positions_in_message` (or `boundary`). A private term: none.
fn hermes_term(t: &Value, text: &str) -> Option<(u32, String)> {
    if t.get("is_private").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let pos = t.get("positions_in_message").or_else(|| t.get("boundary"));
    let at = |k: &str| pos.and_then(|p| p.get(k)?.as_u64()).map(|n| n as u32);
    let start = at("start_pos");
    if let Some(s) = t.get("text").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        return Some((start.unwrap_or(u32::MAX), s.to_owned()));
    }
    let (s, e) = (start?, at("end_pos")?);
    Some((s, cut(text, s, e)?))
}

#[cfg(test)]
mod tests {
    use cyex_eventsub::events::{Message, Who};
    use serde_json::json;

    use super::*;

    fn held(text: &str, reason: &str, bounds: &[(u32, u32)]) -> Held {
        Held {
            message_id: "m".into(),
            user: Who::default(),
            message: Message { text: text.into(), ..Default::default() },
            reason: reason.into(),
            category: None,
            level: None,
            boundaries: bounds.iter().map(|&(start_pos, end_pos)| Boundary { start_pos, end_pos }).collect(),
            terms: vec![],
            held_at: String::new(),
        }
    }

    #[test]
    fn v112_28_eventsub_blocked_terms() {
        assert_eq!(eventsub(&held("you bad word", "blocked_term", &[(4, 6)])), "blocked_term:bad");
        // Several, in message order, each once.
        let h = held("worse bad BAD worse", "blocked_term", &[(6, 8), (0, 4), (10, 12), (14, 18)]);
        assert_eq!(eventsub(&h), "blocked_term:worse\u{1f}bad");
        // No boundary, or one past the text: the term goes unnamed.
        assert_eq!(eventsub(&held("bad", "blocked_term", &[])), "blocked_term");
        assert_eq!(eventsub(&held("bad", "blocked_term", &[(9, 12)])), "blocked_term");
        assert_eq!(eventsub(&held("bad", "blocked_term", &[(2, 1)])), "blocked_term");
        // Code points, not bytes.
        assert_eq!(eventsub(&held("ä bäd", "blocked_term", &[(2, 4)])), "blocked_term:bäd");
        assert_eq!(eventsub(&held("x", "blocked_link", &[])), "blocked_link");
    }

    #[test]
    fn v112_28_eventsub_categories() {
        let mut h = held("you bad word", "automod", &[]);
        h.category = Some("swearing".into());
        h.level = Some(3);
        assert_eq!(eventsub(&h), "automod:swearing:3");
        h.level = None;
        assert_eq!(eventsub(&h), "automod:swearing");
        h.category = Some("unknown".into());
        assert_eq!(eventsub(&h), "AutoMod");
    }

    #[test]
    fn v112_28_terms_capped_and_cleaned() {
        let many = (0..20).map(|i| format!("t{i}"));
        assert_eq!(blocked_term(many).split(SEP).count(), MAX_TERMS);
        assert_eq!(blocked_term(["a\u{1f}b".to_owned(), " ".into()]), "blocked_term:a b");
    }

    fn caught(reason: Value, text: &str) -> String {
        let data = json!({"status": "PENDING", "content_classification": {"category": "unknown", "level": 0},
            "message": {"id": "m1", "content": {"text": text}}, "caught_message_reason": reason});
        hermes(&data, text)
    }

    #[test]
    fn v112_28_hermes_blocked_terms() {
        let term = |id: &str, s: u32, e: u32, private: bool| {
            json!({"term_id": id, "is_private": private, "positions_in_message": {"start_pos": s, "end_pos": e}})
        };
        let one = json!({"reason": "BlockedTermCaughtMessageReason",
            "blocked_term_failure": {"contains_private_term": false, "terms_found": [term("t1", 4, 6, false)]}});
        assert_eq!(caught(one, "you bad word"), "blocked_term:bad");
        let two = json!({"reason": "BlockedTermCaughtMessageReason",
            "blocked_term_failure": {"terms_found": [term("t2", 8, 11, false), term("t1", 4, 6, false)]}});
        assert_eq!(caught(two, "you bad word"), "blocked_term:bad\u{1f}word");
        // A private term: named only by its kind.
        let private = json!({"reason": "BlockedTermCaughtMessageReason",
            "blocked_term_failure": {"contains_private_term": true, "terms_found": [term("t1", 4, 6, true)]}});
        assert_eq!(caught(private, "you bad word"), "blocked_term");
        // No terms or positions at all.
        let bare = json!({"reason": "BlockedTermCaughtMessageReason", "blocked_term_failure": {}});
        assert_eq!(caught(bare, "you bad word"), "blocked_term");
        let no_pos = json!({"reason": "BlockedTermCaughtMessageReason", "blocked_term_failure": {"terms_found": [{"term_id": "t1"}]}});
        assert_eq!(caught(no_pos, "you bad word"), "blocked_term");
        // A text field wins when one is sent.
        let texted = json!({"reason": "BlockedTermCaughtMessageReason",
            "blocked_term_failure": {"terms_found": [{"term_id": "t1", "text": "bad word"}]}});
        assert_eq!(caught(texted, "you bad word"), "blocked_term:bad word");
        assert_eq!(caught(json!({"reason": "LinkCaughtMessageReason"}), "x.com"), "blocked_link");
    }

    #[test]
    fn v112_28_hermes_categories() {
        let am = json!({"reason": "AutoModCaughtMessageReason", "automod_failure": {"category": "aggressive", "level": 2}});
        assert_eq!(caught(am, "hi"), "automod:aggressive:2");
        // Older shape, no caught_message_reason: the classification.
        let old = json!({"content_classification": {"category": "swearing", "level": 1}});
        assert_eq!(hermes(&old, "hi"), "automod:swearing:1");
        assert_eq!(hermes(&json!({}), "hi"), "AutoMod");
        // Before V112-28 a blocked-term hold read as "Unknown": the classification says `unknown`.
        assert_eq!(hermes(&json!({"content_classification": {"category": "unknown", "level": 0}}), "hi"), "AutoMod");
    }
}
