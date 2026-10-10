//! 1.26 Sending a whisper with a Helix sign-in (`user:manage:whispers`). The sender needs
//! a verified phone; Twitch answers 204 even when it drops the whisper. 500 characters to
//! someone who never whispered you, 40 new people a day, 3 a second, 100 a minute.

use serde_json::json;

use super::{Auth, post};
use crate::http::empty;
use crate::{Http, Result};

pub async fn send(http: &Http, auth: Auth<'_>, from_id: &str, to_id: &str, text: &str) -> Result<()> {
    let req = post(http, auth, "/whispers")
        .query(&[("from_user_id", from_id), ("to_user_id", to_id)])
        .json(&json!({ "message": text }));
    empty(req).await
}
