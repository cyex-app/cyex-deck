//! V109-B02 What the cosmetics area says when cyex says no, and what a code gave.

use serde_json::Value;

use super::api::ApiError;

pub const OFFLINE: &str = "Can’t reach cyex right now.";

pub fn error(e: &ApiError) -> String {
    let fixed = match e.code() {
        _ if *e == ApiError::Offline => OFFLINE,
        "not_owned" => "You don’t own that badge.",
        "bad_code" => "That code doesn’t work.",
        "expired" => "That code has run out.",
        "used_up" => "That code was used up.",
        "already_redeemed" => "You already used this code.",
        "owned" => "You already have that.",
        "sold_out" => "Sold out.",
        "too_many" => "Too many tries. Wait a minute.",
        "signed_out" | "bad_token" | "wrong_client" => "cyex didn’t take this account’s sign-in. Sign in again.",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.to_owned();
    }
    match e {
        ApiError::Said { message, .. } if !message.trim().is_empty() => message.trim().to_owned(),
        _ => "Something went wrong. Try again.".to_owned(),
    }
}

fn badge_name(id: &str) -> &str {
    match id {
        "founder" => "Founder",
        "supporter" => "Supporter",
        "custom" | "plus" | "supporter_plus" => "Supporter+",
        other => other,
    }
}

/// `{ reward }` as a line: a badge, Supporter+ days, or plain text.
pub fn reward(v: &Value) -> String {
    if let Some(s) = v.as_str().filter(|s| !s.trim().is_empty()) {
        return s.chars().take(120).collect();
    }
    let days = v.get("days").and_then(Value::as_u64);
    let badge = v.get("badge").and_then(Value::as_str);
    match (badge, days) {
        (_, Some(1)) => "1 day of Supporter+ added.".to_owned(),
        (_, Some(n)) => format!("{} days of Supporter+ added.", grouped(n)),
        (Some(b), None) => format!("{} badge added.", badge_name(b)),
        _ => "Code redeemed.".to_owned(),
    }
}

/// Numbers with dots (`1.000`).
fn grouped(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn said(code: &str, message: &str) -> ApiError {
        ApiError::Said {
            status: 409,
            code: code.into(),
            message: message.into(),
        }
    }

    #[test]
    fn errors_read_short() {
        assert_eq!(error(&ApiError::Offline), OFFLINE);
        assert_eq!(error(&said("sold_out", "x")), "Sold out.");
        assert_eq!(error(&said("new_code", "Try later.")), "Try later.");
        assert_eq!(error(&said("new_code", " ")), "Something went wrong. Try again.");
    }

    #[test]
    fn rewards_read_short() {
        assert_eq!(reward(&json!({"badge":"supporter"})), "Supporter badge added.");
        assert_eq!(reward(&json!({"kind":"plus","days":30})), "30 days of Supporter+ added.");
        assert_eq!(reward(&json!({"days":1000})), "1.000 days of Supporter+ added.");
        assert_eq!(reward(&json!({"days":1})), "1 day of Supporter+ added.");
        assert_eq!(reward(&json!("Founder #12 is yours.")), "Founder #12 is yours.");
        assert_eq!(reward(&Value::Null), "Code redeemed.");
    }
}
