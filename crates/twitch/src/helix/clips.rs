//! P9 Clips from the player. Needs a Helix token with `clips:edit`. Twitch makes the clip
//! from the last seconds of the stream; it can be trimmed and titled at `edit_url`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Auth, Data, post};
use crate::http::json;
use crate::{Error, Http, Result};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreatedClip {
    /// The clip's slug. It takes a few seconds to be watchable.
    pub id: String,
    pub edit_url: String,
}

pub async fn create_clip(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<CreatedClip> {
    let req = post(http, auth, "/clips").query(&[("broadcaster_id", broadcaster_id)]);
    let data: Data<CreatedClip> = json(req).await?;
    data.data.into_iter().next().ok_or(Error::Api {
        status: 200,
        message: "no clip".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_clip_parses() {
        let d: Data<CreatedClip> = serde_json::from_str(
            r#"{"data":[{"id":"FiveWordsForClipSlug","edit_url":"https://clips.twitch.tv/FiveWordsForClipSlug/edit"}]}"#,
        )
        .unwrap();
        assert_eq!(d.data[0].id, "FiveWordsForClipSlug");
    }
}
