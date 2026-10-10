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
            .user_agent(concat!("cyex-viewer/", env!("CARGO_PKG_VERSION")))
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

    /// POST a JSON body to another service and decode the JSON answer.
    pub async fn post_json<T: DeserializeOwned>(&self, url: &str, body: &serde_json::Value) -> Result<T> {
        json(self.0.post(url).json(body)).await
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
    let res = req.send().await.map_err(Error::Network)?;
    let status = res.status();
    let body = res.bytes().await.map_err(Error::Network)?;
    if status.is_success() {
        Ok(body.to_vec())
    } else {
        Err(Error::api(status.as_u16(), &body))
    }
}
