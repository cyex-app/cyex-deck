//! S5 Runs a plugin's HTTP request. Redirects are followed by hand so every hop is checked
//! against the plugin's hosts; credentials don't follow a redirect to another host.

use std::time::Duration;

use reqwest::header::{AUTHORIZATION, COOKIE, LOCATION};
use reqwest::{Method, StatusCode};

pub use reqwest::Client;

use crate::host::{HttpRequest, HttpResponse};
use crate::perm::check_url;

const MAX_REDIRECTS: usize = 5;
/// Biggest answer a plugin gets.
pub const MAX_RESPONSE: usize = 5 << 20;

/// One client for every plugin: no redirects of its own, no cookies.
pub fn client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("cyex-viewer/", env!("CARGO_PKG_VERSION"), " (plugin)"))
        .build()
        .unwrap_or_default()
}

pub async fn fetch(client: &Client, req: HttpRequest) -> HttpResponse {
    match run(client, req).await {
        Ok(r) => r,
        Err(e) => HttpResponse {
            status: 0,
            data: Vec::new(),
            error: Some(e),
        },
    }
}

async fn run(client: &Client, req: HttpRequest) -> Result<HttpResponse, String> {
    let mut url = check_url(&req.url, &req.hosts)?;
    let mut method = Method::from_bytes(req.method.as_bytes()).map_err(|e| e.to_string())?;
    let mut body = req.body;
    let first_host = url.host_str().map(str::to_owned);
    for _ in 0..=MAX_REDIRECTS {
        let same_host = url.host_str().map(str::to_owned) == first_host;
        let mut rb = client
            .request(method.clone(), url.clone())
            .timeout(Duration::from_millis(req.timeout_ms));
        for (k, v) in &req.headers {
            if !same_host
                && (k.eq_ignore_ascii_case(AUTHORIZATION.as_str()) || k.eq_ignore_ascii_case(COOKIE.as_str()))
            {
                continue;
            }
            rb = rb.header(k, v);
        }
        if let Some(b) = &body {
            rb = rb.body(b.clone());
        }
        let mut resp = rb.send().await.map_err(|e| short(&e))?;
        let status = resp.status();
        if status.is_redirection()
            && let Some(loc) = resp.headers().get(LOCATION).and_then(|l| l.to_str().ok())
        {
            let next = url.join(loc).map_err(|_| "Bad redirect.".to_owned())?;
            url = check_url(next.as_str(), &req.hosts)?;
            if status == StatusCode::SEE_OTHER
                || (matches!(status, StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND)
                    && method == Method::POST)
            {
                method = Method::GET;
                body = None;
            }
            continue;
        }
        let mut data = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(|e| short(&e))? {
            if data.len() + chunk.len() > MAX_RESPONSE {
                return Err("The answer is over 5 MB.".into());
            }
            data.extend_from_slice(&chunk);
        }
        return Ok(HttpResponse {
            status: status.as_u16(),
            data,
            error: None,
        });
    }
    Err("Too many redirects.".into())
}

fn short(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "Timed out.".into()
    } else if e.is_connect() {
        "Couldn't connect.".into()
    } else {
        e.to_string()
    }
}
