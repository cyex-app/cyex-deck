//! V108-N12 Shared Chat: the session a channel is in right now (no scope needed).

use serde::Deserialize;

use super::{Auth, Data, get};
use crate::http::json;
use crate::{Http, Result};

/// A running session. Helix names the participants by id only.
#[derive(Clone, Debug, Deserialize)]
pub struct SharedSession {
    pub session_id: String,
    pub participants: Vec<Participant>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Participant {
    pub broadcaster_id: String,
}

/// The session `broadcaster_id` is in, `None` when it isn't in one (Twitch answers 404).
pub async fn session(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<Option<SharedSession>> {
    let req = get(http, auth, "/shared_chat/session").query(&[("broadcaster_id", broadcaster_id)]);
    match json::<Data<SharedSession>>(req).await {
        Ok(d) => Ok(d.data.into_iter().next()),
        Err(e) if e.status() == Some(404) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_parses() {
        let d: Data<SharedSession> = serde_json::from_str(
            r#"{"data":[{"session_id":"74e8c1ba","host_broadcaster_id":"1","participants":[{"broadcaster_id":"1"},{"broadcaster_id":"2"}],
            "created_at":"2024-09-29T19:45:37Z","updated_at":"2024-09-29T19:45:37Z"}]}"#,
        )
        .unwrap();
        let s = &d.data[0];
        assert_eq!((s.session_id.as_str(), s.participants.len()), ("74e8c1ba", 2));
        assert_eq!(s.participants[1].broadcaster_id, "2");
    }
}
