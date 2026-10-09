//! GQL (gql.twitch.tv), Twitch's private API. Can change without notice. All persisted
//! query hashes will live in one registry file; raw queries are used where they work.

mod users;

pub use users::{GqlUser, user_by_id};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::http::json as send_json;
use crate::{Error, Http, Result};

const URL: &str = "https://gql.twitch.tv/gql";

/// Client ID plus an optional user token. Without a token, only public data is readable.
#[derive(Clone, Copy)]
pub struct Auth<'a> {
    pub client_id: &'a str,
    pub token: Option<&'a str>,
}

#[derive(Deserialize)]
struct Response<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GqlError>,
}

#[derive(Deserialize)]
struct GqlError {
    message: String,
}

/// Runs a raw query.
pub(crate) async fn query<T: DeserializeOwned>(
    http: &Http,
    auth: Auth<'_>,
    query: &str,
    variables: Value,
) -> Result<T> {
    let mut req = http
        .0
        .post(URL)
        .header("Client-Id", auth.client_id)
        .json(&json!({ "query": query, "variables": variables }));
    if let Some(token) = auth.token {
        req = req.header("Authorization", format!("OAuth {token}"));
    }
    unwrap(send_json(req).await?)
}

fn unwrap<T>(res: Response<T>) -> Result<T> {
    match (res.data, res.errors.into_iter().next()) {
        (Some(data), None) => Ok(data),
        (_, Some(e)) => Err(Error::Api {
            status: 200,
            message: e.message,
        }),
        (None, None) => Err(Error::Api {
            status: 200,
            message: "empty answer".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_win_over_partial_data() {
        let res: Response<Value> =
            serde_json::from_str(r#"{"data":{"user":null},"errors":[{"message":"service timeout"}]}"#)
                .unwrap();
        assert_eq!(unwrap(res).unwrap_err().api_message(), Some("service timeout"));
        let res: Response<Value> = serde_json::from_str(r#"{"data":{"user":null}}"#).unwrap();
        assert!(unwrap(res).is_ok());
    }
}
