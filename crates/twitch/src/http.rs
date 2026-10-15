use std::time::Duration;

use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;

use crate::{Error, Result};

/// Shared HTTP client. Cheap to clone; keep one per app.
#[derive(Clone)]
pub struct Http(pub(crate) reqwest::Client);

impl Http {
    pub fn new() -> Result<Self> {
        Self::build(None)
    }

    /// P10 Everything through an HTTP proxy, e.g. `http://user:pass@host:8080`.
    pub fn with_proxy(url: &str) -> Result<Self> {
        Self::build(Some(url))
    }

    fn build(proxy: Option<&str>) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .user_agent(cyex_core::app::user_agent())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20));
        if let Some(url) = proxy {
            builder = builder.proxy(reqwest::Proxy::all(url).map_err(Error::Network)?);
        }
        builder.build().map(Self).map_err(Error::Network)
    }

    /// GET JSON from another service (emote providers).
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        json(self.0.get(url)).await
    }

    /// 1.8 `$request` in custom commands: any web page as text, at most `max` bytes.
    pub async fn get_text(&self, url: &str, headers: &[(String, String)], max: usize) -> Result<String> {
        let mut req = self.0.get(url);
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let res = req.send().await.map_err(Error::Network)?;
        let bytes = res.bytes().await.map_err(Error::Network)?;
        Ok(String::from_utf8_lossy(&bytes[..bytes.len().min(max)]).into_owned())
    }

    /// X6 A picture to keep, refused when bigger than `max`.
    pub async fn get_bytes(&self, url: &str, max: usize) -> Result<Vec<u8>> {
        let req = self.0.get(url).header("Accept", "image/webp,image/*");
        let bytes = body(req).await?;
        if bytes.len() > max {
            return Err(Error::Api {
                status: 413,
                message: "too big".into(),
            });
        }
        Ok(bytes)
    }

    /// The status of a HEAD request (watch time touches a segment like a player would).
    pub async fn head(&self, url: &str, user_agent: &str) -> Result<u16> {
        let res = self.0.head(url).header("User-Agent", user_agent).send().await.map_err(Error::Network)?;
        Ok(res.status().as_u16())
    }

    /// POST a form and return the status, e.g. spade's 204.
    pub async fn post_form(&self, url: &str, form: &[(&str, &str)], user_agent: &str) -> Result<u16> {
        let res = self
            .0
            .post(url)
            .header("User-Agent", user_agent)
            .form(form)
            .send()
            .await
            .map_err(Error::Network)?;
        Ok(res.status().as_u16())
    }

    /// POST a JSON body to another service and decode the JSON answer.
    pub async fn post_json<T: DeserializeOwned>(&self, url: &str, body: &serde_json::Value) -> Result<T> {
        json(self.0.post(url).json(body)).await
    }

    /// V105-11 POST a ready body (the image uploader's multipart form) with extra headers.
    /// Returns the status and up to `max` bytes of the answer. Errors never carry the URL.
    pub async fn post_body(
        &self,
        url: &str,
        headers: &[(String, String)],
        content_type: &str,
        body: Vec<u8>,
        max: usize,
    ) -> Result<(u16, Vec<u8>)> {
        let mut req = self.0.post(url).header("Content-Type", content_type).body(body);
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let mut res = req
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| Error::Network(e.without_url()))?;
        let status = res.status().as_u16();
        // Only what's needed: a host that answers with a huge body isn't read to the end.
        let mut out = Vec::new();
        while out.len() < max {
            match res.chunk().await.map_err(|e| Error::Network(e.without_url()))? {
                Some(chunk) => out.extend_from_slice(&chunk),
                None => break,
            }
        }
        out.truncate(max);
        Ok((status, out))
    }
}

/// Sends the request and decodes a JSON body, or turns an error status into `Error::Api`.
pub(crate) async fn json<T: DeserializeOwned>(req: RequestBuilder) -> Result<T> {
    let body = body(req).await?;
    serde_json::from_slice(&body).map_err(Error::Decode)
}

/// Sends the request and ignores the body.
pub(crate) async fn empty(req: RequestBuilder) -> Result<()> {
    body(req).await.map(|_| ())
}

async fn body(req: RequestBuilder) -> Result<Vec<u8>> {
    limited(req).await.0
}

/// Helix's points bucket after a request (`Ratelimit-*` headers), per token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateLimit {
    pub limit: u32,
    pub remaining: u32,
    /// Unix seconds when the bucket is full again.
    pub reset: i64,
}

impl RateLimit {
    fn read(headers: &reqwest::header::HeaderMap) -> Option<Self> {
        let num = |k: &str| headers.get(k)?.to_str().ok()?.trim().parse::<i64>().ok();
        Some(Self {
            limit: u32::try_from(num("ratelimit-limit")?).ok()?,
            remaining: u32::try_from(num("ratelimit-remaining")?).ok()?,
            reset: num("ratelimit-reset")?,
        })
    }
}

/// Like `body`, plus the rate limit Twitch reported, if any (mod tools pace by it).
pub(crate) async fn limited(req: RequestBuilder) -> (Result<Vec<u8>>, Option<RateLimit>) {
    let res = match req.send().await {
        Ok(r) => r,
        Err(e) => return (Err(Error::Network(e)), None),
    };
    let status = res.status();
    let limit = RateLimit::read(res.headers());
    let body = match res.bytes().await {
        Ok(b) => b,
        Err(e) => return (Err(Error::Network(e)), limit),
    };
    let out = if status.is_success() {
        Ok(body.to_vec())
    } else {
        Err(Error::api(status.as_u16(), &body))
    };
    (out, limit)
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, HeaderValue};

    use super::RateLimit;

    #[test]
    fn rate_limit_headers() {
        let mut h = HeaderMap::new();
        h.insert("Ratelimit-Limit", HeaderValue::from_static("800"));
        h.insert("Ratelimit-Remaining", HeaderValue::from_static("799"));
        h.insert("Ratelimit-Reset", HeaderValue::from_static("1760000000"));
        assert_eq!(
            RateLimit::read(&h),
            Some(RateLimit {
                limit: 800,
                remaining: 799,
                reset: 1_760_000_000
            })
        );
        h.remove("Ratelimit-Reset");
        assert_eq!(RateLimit::read(&h), None);
    }
}
