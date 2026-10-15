//! V1051-02 The one decision behind the menu's state.

use cyex_auth::SevenTvToken;
use cyex_emotes::seventv::account::Error;
use cyex_emotes::seventv::account::model::Inventory;
use cyex_emotes::seventv::account::view::Status;

pub fn status(
    consent: bool,
    inv: Result<Option<&Inventory>, &Error>,
    token: Option<&SevenTvToken>,
    now: i64,
) -> Status {
    if !consent {
        return Status::NoConsent;
    }
    match inv {
        Err(_) => Status::Down,
        Ok(None) => Status::NoAccount,
        Ok(Some(_)) => match token {
            Some(t) if t.expired(now) => Status::Expired,
            Some(_) => Status::Ready,
            None => Status::SignedOut,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inv() -> Inventory {
        cyex_emotes::seventv::account::model::parse_user(serde_json::json!({ "id": "01X" }))
            .unwrap()
            .unwrap()
    }

    /// Made-up tokens: `{"sub":"01X","exp":2000}` and `exp` 1000.
    fn token(exp: i64) -> SevenTvToken {
        let payload = if exp == 2_000 {
            "eyJzdWIiOiIwMVgiLCJleHAiOjIwMDB9"
        } else {
            "eyJzdWIiOiIwMVgiLCJleHAiOjEwMDB9"
        };
        SevenTvToken::parse(&format!("e30.{payload}.c2ln")).unwrap()
    }

    #[test]
    fn every_row() {
        let i = inv();
        let down = Error::Down;
        let live = token(2_000);
        let dead = token(1_000);
        assert_eq!(status(false, Ok(Some(&i)), Some(&live), 0), Status::NoConsent);
        assert_eq!(status(false, Err(&down), None, 0), Status::NoConsent);
        assert_eq!(status(true, Err(&down), Some(&live), 0), Status::Down);
        assert_eq!(status(true, Err(&Error::Bad), None, 0), Status::Down);
        assert_eq!(status(true, Ok(None), Some(&live), 0), Status::NoAccount);
        assert_eq!(status(true, Ok(Some(&i)), Some(&live), 100), Status::Ready);
        assert_eq!(status(true, Ok(Some(&i)), Some(&dead), 1_000), Status::Expired);
        assert_eq!(status(true, Ok(Some(&i)), None, 0), Status::SignedOut);
    }
}
