use std::time::Duration;

use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;

use crate::{Error, Result};

/// Shared HTTP client. Cheap to clone; keep one per app.
#[derive(Clone)]
pub struct Http(pub(crate) reqwest::Client);

impl Http {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("cyex-viewer/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(Error::Network)?;
        Ok(Self(client))
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
