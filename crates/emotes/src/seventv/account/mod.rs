//! V1051-02 Every 7TV account call lives here: reads of a public inventory, the signed-in
//! user, cosmetic changes and logout. The token only ever goes to the client's own base.

pub mod error;
pub mod query;
#[cfg(test)]
mod tests;

use cyex_twitch::Http;
use serde_json::{Value, json};

pub use error::Error;

pub const GQL: &str = "https://api.7tv.app/v4/gql";
pub const AUTH: &str = "https://api.7tv.app/v4/auth";
/// 7TV's subscription product (research §2).
pub const SUB_PRODUCT: &str = "01FEVKBBTGRAT7FCY276TNTJ4A";

#[derive(Clone)]
pub struct Client {
    http: Http,
    gql: String,
    auth: String,
}

/// The signed-in 7TV user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Me {
    pub id: String,
    /// The Twitch connection's `platformId`.
    pub twitch_id: Option<String>,
}

impl Client {
    pub fn new(http: Http) -> Self {
        Self {
            http,
            gql: GQL.into(),
            auth: AUTH.into(),
        }
    }

    /// Tests and mock runs: `base` like `http://127.0.0.1:5199` serves `/v4/gql` and `/v4/auth/*`.
    pub fn at(http: Http, base: &str) -> Self {
        let base = base.trim_end_matches('/');
        Self {
            http,
            gql: format!("{base}/v4/gql"),
            auth: format!("{base}/v4/auth"),
        }
    }

    pub fn login_url(&self) -> String {
        format!("{}/login?platform=twitch", self.auth)
    }

    /// The account's public inventory as 7TV answered it (`None`: no 7TV user). Group 2 types it.
    pub async fn inventory(&self, twitch_id: &str) -> Result<Option<Value>, Error> {
        let vars = json!({ "tw": twitch_id, "product": SUB_PRODUCT });
        let data = self.gql(None, query::INVENTORY, vars).await?;
        Ok(data
            .pointer("/users/userByConnection")
            .filter(|u| !u.is_null())
            .cloned())
    }

    pub async fn me(&self, token: &str) -> Result<Me, Error> {
        let data = self.gql(Some(token), query::ME, json!({})).await?;
        let me = data
            .pointer("/users/me")
            .filter(|m| !m.is_null())
            .ok_or(Error::Bad)?;
        let id = me.get("id").and_then(Value::as_str).ok_or(Error::Bad)?.to_owned();
        let twitch_id = me
            .get("connections")
            .and_then(Value::as_array)
            .and_then(|list| {
                list.iter()
                    .find(|c| c.get("platform").and_then(Value::as_str) == Some("TWITCH"))
            })
            .and_then(|c| c.get("platformId").and_then(Value::as_str))
            .map(str::to_owned);
        Ok(Me { id, twitch_id })
    }

    /// Sets (or clears, with `None`) the active paint. Returns the active id 7TV now reports.
    pub async fn set_paint(
        &self,
        token: &str,
        user: &str,
        paint: Option<&str>,
    ) -> Result<Option<String>, Error> {
        let vars = json!({ "user": user, "paint": paint });
        let data = self.gql(Some(token), query::SET_PAINT, vars).await?;
        Ok(active(&data, "/users/user/activePaint/style/activePaintId"))
    }

    pub async fn set_badge(
        &self,
        token: &str,
        user: &str,
        badge: Option<&str>,
    ) -> Result<Option<String>, Error> {
        let vars = json!({ "user": user, "badge": badge });
        let data = self.gql(Some(token), query::SET_BADGE, vars).await?;
        Ok(active(&data, "/users/user/activeBadge/style/activeBadgeId"))
    }

    /// Ends the session on 7TV's side. A 401 means it was already gone.
    pub async fn logout(&self, token: &str) -> Result<(), Error> {
        let url = format!("{}/logout", self.auth);
        let (status, reset, _) = self
            .http
            .post_json_raw(&url, Some(token), &[], &json!({}))
            .await
            .map_err(error::transport)?;
        match status {
            401 | 200..=299 => Ok(()),
            _ => error::classify(status, reset, b"{}").map(|_| ()),
        }
    }

    async fn gql(&self, token: Option<&str>, query: &str, vars: Value) -> Result<Value, Error> {
        let body = json!({ "query": query, "variables": vars });
        let headers: &[(&str, &str)] = if token.is_none() {
            &[("x-ignore-auth-failure", "true")]
        } else {
            &[]
        };
        let (status, reset, bytes) = self
            .http
            .post_json_raw(&self.gql, token, headers, &body)
            .await
            .map_err(error::transport)?;
        error::classify(status, reset, &bytes)
    }
}

fn active(data: &Value, path: &str) -> Option<String> {
    data.pointer(path).and_then(Value::as_str).map(str::to_owned)
}
