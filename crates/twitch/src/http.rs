use std::time::Duration;

use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;

use crate::{Error, Result};

/// Shared HTTP client. Cheap to clone; keep one per app. The second field is a local mock
/// that gets every Helix and GQL call instead of Twitch (P4 #16, mocks builds only).
#[derive(Clone)]
pub struct Http(pub(crate) reqwest::Client, pub(crate) Option<std::sync::Arc<str>>);

/// V108-C10 A desktop browser, for picture hosts that refuse the app's own user agent.
pub const BROWSER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

/// P4 #16 What a mock gets instead of the account's sign-in (SEC-01: never the real one).
pub(crate) const MOCK_TOKEN: &str = "mock-token";
pub(crate) const MOCK_CLIENT: &str = "mock-client";

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
        builder.build().map(|c| Self(c, None)).map_err(Error::Network)
    }

    /// P4 #16 This client with every Helix and GQL call sent to `base`
    /// (`http://127.0.0.1:<port>`, checked by the caller) with a fake sign-in.
    #[cfg(any(test, feature = "mocks"))]
    pub fn mocked(&self, base: &str) -> Self {
        Self(self.0.clone(), Some(base.trim_end_matches('/').into()))
    }

    /// P4 #16 The mock this client talks to, if any.
    pub fn mock_base(&self) -> Option<&str> {
        self.1.as_deref()
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
        let mut res = req.send().await.map_err(|e| Error::Network(e.without_url()))?;
        let status = res.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(Error::Api {
                status,
                message: "request".into(),
            });
        }
        let mut out = Vec::new();
        while out.len() < max {
            match res.chunk().await.map_err(|e| Error::Network(e.without_url()))? {
                Some(chunk) => out.extend_from_slice(&chunk),
                None => break,
            }
        }
        out.truncate(max);
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    /// V106-24 A small file, or None when it's bigger than `max` (stops reading there).
    pub async fn get_small(&self, url: &str, max: usize) -> Result<Option<Vec<u8>>> {
        small(self.0.get(url), max).await
    }

    /// V108-C10 A small picture as a browser asks for it (some badge hosts refuse other user
    /// agents), given up after `timeout`. None when it's bigger than `max`.
    pub async fn get_small_as(
        &self,
        url: &str,
        max: usize,
        user_agent: &str,
        timeout: Duration,
    ) -> Result<Option<Vec<u8>>> {
        let req = self
            .0
            .get(url)
            .header("User-Agent", user_agent)
            .header("Accept", "image/webp,image/*")
            .timeout(timeout);
        small(req, max).await
    }

    /// X6 A picture to keep, refused when bigger than `max`.
    pub async fn get_bytes(&self, url: &str, max: usize) -> Result<Vec<u8>> {
        let req = self.0.get(url).header("Accept", "image/webp,image/*");
        // V110-F85 Read with a running limit: a chunked or mislabelled body never fills memory.
        limited_to(req, max).await.0
    }

    /// The status of a HEAD request (watch time touches a segment like a player would).
    pub async fn head(&self, url: &str, user_agent: &str) -> Result<u16> {
        let res = self.0.head(url).header("User-Agent", user_agent).send().await.map_err(|e| Error::Network(e.without_url()))?;
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
            .map_err(|e| Error::Network(e.without_url()))?;
        Ok(res.status().as_u16())
    }

    /// POST a JSON body to another service and decode the JSON answer.
    pub async fn post_json<T: DeserializeOwned>(&self, url: &str, body: &serde_json::Value) -> Result<T> {
        json(self.0.post(url).json(body)).await
    }

    /// V1051-02 POST JSON with an optional bearer token and extra headers; returns the status,
    /// the rate-limit reset (seconds, if sent) and the body. Errors never carry the URL.
    pub async fn post_json_raw(
        &self,
        url: &str,
        bearer: Option<&str>,
        headers: &[(&str, &str)],
        body: &serde_json::Value,
    ) -> Result<(u16, Option<u64>, Vec<u8>)> {
        const MAX: usize = 4 << 20;
        let mut req = self.0.post(url).json(body);
        if let Some(b) = bearer {
            req = req.bearer_auth(b);
        }
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let mut res = req.send().await.map_err(|e| Error::Network(e.without_url()))?;
        let status = res.status().as_u16();
        let reset = retry_secs(res.headers());
        let mut out = Vec::new();
        while let Some(chunk) = res.chunk().await.map_err(|e| Error::Network(e.without_url()))? {
            out.extend_from_slice(&chunk);
            if out.len() > MAX {
                return Err(Error::Api {
                    status: 413,
                    message: "too big".into(),
                });
            }
        }
        Ok((status, reset, out))
    }

    /// V109-T01 Any method with an optional bearer token and body (`(content type, bytes)`).
    /// Returns the status and up to `max` bytes of the answer. Errors never carry the URL.
    pub async fn send(
        &self,
        method: &str,
        url: &str,
        bearer: Option<&str>,
        body: Option<(String, Vec<u8>)>,
        max: usize,
    ) -> Result<(u16, Vec<u8>)> {
        let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| Error::Api {
            status: 0,
            message: "bad method".into(),
        })?;
        let mut req = self.0.request(method, url).timeout(Duration::from_secs(60));
        if let Some(b) = bearer {
            req = req.bearer_auth(b);
        }
        if let Some((content_type, bytes)) = body {
            req = req.header("Content-Type", content_type).body(bytes);
        }
        let mut res = req.send().await.map_err(|e| Error::Network(e.without_url()))?;
        let status = res.status().as_u16();
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

/// `Retry-After` or `X-RateLimit-Reset` in whole seconds (the latter may be a unix time).
pub(crate) fn retry_secs(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    ["retry-after", "x-ratelimit-reset"].iter().find_map(|h| {
        headers
            .get(*h)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok())
    })
}

/// V106-24 The body, or None when it's bigger than `max` (stops reading there). Errors never
/// carry the URL.
async fn small(req: RequestBuilder, max: usize) -> Result<Option<Vec<u8>>> {
    let mut res = req.send().await.map_err(|e| Error::Network(e.without_url()))?;
    let status = res.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(Error::Api {
            status,
            message: "request".into(),
        });
    }
    if res.content_length().is_some_and(|n| n > max as u64) {
        return Ok(None);
    }
    let mut out = Vec::new();
    while let Some(chunk) = res.chunk().await.map_err(|e| Error::Network(e.without_url()))? {
        out.extend_from_slice(&chunk);
        if out.len() > max {
            return Ok(None);
        }
    }
    Ok(Some(out))
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
    limited_to(req, MAX_BODY).await
}

/// V110-F85 The most an API answer may be (the biggest are inventories and follow lists).
const MAX_BODY: usize = 32 << 20;

/// V110-F85 [`limited`] with a running size limit: `Error::Api` 413 once the body passes
/// `max`, without having buffered more than `max` plus one chunk. Errors never carry the URL.
async fn limited_to(req: RequestBuilder, max: usize) -> (Result<Vec<u8>>, Option<RateLimit>) {
    let mut res = match req.send().await {
        Ok(r) => r,
        Err(e) => return (Err(Error::Network(e.without_url())), None),
    };
    let status = res.status();
    let limit = RateLimit::read(res.headers());
    if res.content_length().is_some_and(|n| n > max as u64) {
        return (Err(too_big()), limit);
    }
    let mut body = Vec::new();
    loop {
        match res.chunk().await {
            Ok(Some(chunk)) => {
                if !push_within(&mut body, &chunk, max) {
                    return (Err(too_big()), limit);
                }
            }
            Ok(None) => break,
            Err(e) => return (Err(Error::Network(e.without_url())), limit),
        }
    }
    let out = if status.is_success() {
        Ok(body)
    } else {
        Err(Error::api(status.as_u16(), &body))
    };
    (out, limit)
}

fn too_big() -> Error {
    Error::Api {
        status: 413,
        message: "too big".into(),
    }
}

/// V110-F85 Appends `chunk` unless the body would pass `max`; false then (nothing appended).
pub(crate) fn push_within(body: &mut Vec<u8>, chunk: &[u8], max: usize) -> bool {
    if body.len().saturating_add(chunk.len()) > max {
        return false;
    }
    body.extend_from_slice(chunk);
    true
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, HeaderValue};

    use super::{Http, RateLimit, push_within};

    #[test]
    fn v110_f85_a_body_is_refused_past_the_limit_without_being_kept() {
        let mut body = Vec::new();
        assert!(push_within(&mut body, &[0; 6], 10));
        assert!(!push_within(&mut body, &[0; 5], 10));
        assert_eq!(body.len(), 6);
        assert!(push_within(&mut body, &[0; 4], 10));
        assert!(!push_within(&mut body, &[0; 1], 10));
        assert!(!push_within(&mut Vec::new(), &[0; 1], 0));
    }

    /// A server that answers with a chunked body of `chunks` x 1 KB and then hangs up.
    async fn chunky(chunks: usize) -> String {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let _ = tokio::io::AsyncReadExt::read(&mut s, &mut buf).await;
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n").await;
            for _ in 0..chunks {
                let _ = s.write_all(b"400\r\n").await;
                let _ = s.write_all(&[b'x'; 1024]).await;
                if s.write_all(b"\r\n").await.is_err() {
                    return;
                }
            }
            let _ = s.write_all(b"0\r\n\r\n").await;
        });
        format!("http://{addr}/")
    }

    #[tokio::test]
    async fn v110_f85_a_chunked_picture_over_the_limit_is_refused() {
        let http = Http::new().unwrap();
        let err = http.get_bytes(&chunky(64).await, 10 * 1024).await.unwrap_err();
        assert!(matches!(err, crate::Error::Api { status: 413, .. }), "{err:?}");
        let ok = http.get_bytes(&chunky(4).await, 10 * 1024).await.unwrap();
        assert_eq!(ok.len(), 4096);
    }

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
