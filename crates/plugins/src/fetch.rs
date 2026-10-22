//! S5 Runs a plugin's HTTP request. Redirects are followed by hand so every hop is checked
//! against the plugin's hosts; credentials don't follow a redirect to another host.

use std::error::Error as _;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::header::{AUTHORIZATION, COOKIE, LOCATION};
use reqwest::{Method, StatusCode};

pub use reqwest::Client;

use crate::host::{HttpRequest, HttpResponse};
use crate::perm::{check_url, public};

const MAX_REDIRECTS: usize = 5;
/// Biggest answer a plugin gets.
pub const MAX_RESPONSE: usize = 5 << 20;

/// One client for every plugin: no redirects of its own, no cookies, no proxy.
pub fn client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(format!("{} (plugin)", cyex_core::app::user_agent()))
        .dns_resolver(Arc::new(PublicOnly))
        // V107-Mb A system proxy (HTTPS_PROXY and the like) would look the name up itself,
        // past the public-address check.
        .no_proxy()
        .build()
        .unwrap_or_default()
}

/// V107-A5 Resolves names and keeps only public addresses, so a name that points to this
/// computer or the local network is refused (docs/plugins.md, `Network`).
struct PublicOnly;

impl Resolve for PublicOnly {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let found = tokio::net::lookup_host((host.as_str(), 0)).await?;
            let addrs = keep_public(found);
            if addrs.is_empty() {
                return Err(Box::new(LocalAddress(host)) as Box<dyn std::error::Error + Send + Sync>);
            }
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

/// The addresses a plugin may dial.
fn keep_public(addrs: impl Iterator<Item = SocketAddr>) -> Vec<SocketAddr> {
    addrs.filter(|a| public(a.ip())).collect()
}

#[derive(Debug)]
struct LocalAddress(String);

impl std::fmt::Display for LocalAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} points to a local address.", self.0)
    }
}

impl std::error::Error for LocalAddress {}

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
    let mut src = e.source();
    while let Some(s) = src {
        if let Some(local) = s.downcast_ref::<LocalAddress>() {
            return local.to_string();
        }
        src = s.source();
    }
    if e.is_timeout() {
        "Timed out.".into()
    } else if e.is_connect() {
        "Couldn’t connect.".into()
    } else {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_for_this_pc_is_refused() {
        let addrs: Vec<SocketAddr> = ["127.0.0.1:0", "192.168.1.2:0", "[::1]:0", "93.184.216.34:0"]
            .iter()
            .map(|a| a.parse().unwrap())
            .collect();
        let kept = keep_public(addrs.into_iter());
        assert_eq!(kept, vec!["93.184.216.34:0".parse::<SocketAddr>().unwrap()]);
    }

    #[test]
    fn a_local_address_error_reads_plainly() {
        assert_eq!(LocalAddress("example.com".into()).to_string(), "example.com points to a local address.");
    }
}
