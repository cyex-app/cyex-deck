//! P10 The ad-block proxy. Its address can carry a password, so it lives in the Windows
//! Credential Manager like tokens do, not in settings files or exports.

use cyex_auth::secrets::{KeyringStore, SecretStore};
use cyex_twitch::Http;
use url::Url;

const KEY: &str = "player.proxy";

pub fn load() -> Option<String> {
    KeyringStore.load(KEY).ok().flatten().filter(|u| valid(u).is_ok())
}

/// Saves `url`, or forgets the proxy with `None`. Returns the client to use.
pub fn save(url: Option<&str>) -> Result<Option<Http>, String> {
    match url.map(str::trim).filter(|u| !u.is_empty()) {
        None => {
            KeyringStore.delete(KEY)?;
            Ok(None)
        }
        Some(u) => {
            valid(u)?;
            let http = Http::with_proxy(u).map_err(|_| "That proxy address doesn’t work.".to_owned())?;
            KeyringStore.save(KEY, u)?;
            Ok(Some(http))
        }
    }
}

pub fn client(url: &str) -> Option<Http> {
    Http::with_proxy(url).ok()
}

/// What settings show: `host:port`, never the password.
pub fn shown(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|u| Some(format!("{}:{}", u.host_str()?, u.port_or_known_default()?)))
        .unwrap_or_default()
}

fn valid(url: &str) -> Result<(), String> {
    let u = Url::parse(url).map_err(|_| "Use an address like http://host:8080.".to_owned())?;
    if !matches!(u.scheme(), "http" | "https") || u.host_str().is_none() {
        return Err("Only http:// and https:// proxies work.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert!(valid("http://user:pass@proxy.example:8080").is_ok());
        assert!(valid("socks5://proxy.example:1080").is_err());
        assert!(valid("proxy.example:8080").is_err());
        assert_eq!(shown("http://user:secret@proxy.example:8080"), "proxy.example:8080");
        assert_eq!(shown("https://proxy.example"), "proxy.example:443");
    }
}
