//! V109-B02 One plain call to a JSON service of cyex's own (api.cyex.app): any verb, an
//! optional bearer, JSON body and `If-None-Match`. The answer comes back whole (status, ETag,
//! body), so the caller reads its own error shape. Errors never carry the URL.

use crate::{Error, Http, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Get,
    Post,
    Put,
    Delete,
}

/// What came back: any status, 304 included.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub etag: Option<String>,
    pub body: Vec<u8>,
}

/// One call's parts.
#[derive(Clone, Copy, Debug, Default)]
pub struct Call<'a> {
    pub bearer: Option<&'a str>,
    pub json: Option<&'a serde_json::Value>,
    pub if_none_match: Option<&'a str>,
    /// Bodies longer than this are refused (413), read no further.
    pub max: usize,
}

impl Http {
    pub async fn call(&self, verb: Verb, url: &str, c: Call<'_>) -> Result<Reply> {
        let mut req = match verb {
            Verb::Get => self.0.get(url),
            Verb::Post => self.0.post(url),
            Verb::Put => self.0.put(url),
            Verb::Delete => self.0.delete(url),
        };
        if let Some(b) = c.bearer {
            req = req.bearer_auth(b);
        }
        if let Some(j) = c.json {
            req = req.json(j);
        }
        if let Some(t) = c.if_none_match {
            req = req.header("If-None-Match", t);
        }
        let mut res = req.send().await.map_err(|e| Error::Network(e.without_url()))?;
        let status = res.status().as_u16();
        let etag = res
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let mut body = Vec::new();
        while let Some(chunk) = res.chunk().await.map_err(|e| Error::Network(e.without_url()))? {
            body.extend_from_slice(&chunk);
            if body.len() > c.max {
                return Err(Error::Api {
                    status: 413,
                    message: "too big".into(),
                });
            }
        }
        Ok(Reply { status, etag, body })
    }
}
