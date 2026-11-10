//! What the command interpreter can reach in the app: the user's custom replacements,
//! `/set var` values, web requests for `$request`, the clock.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::Arc;
use std::time::Duration;

use cyex_chat::commands::{CommandList, Host, Items, Var};
use cyex_settings::Partition;
use cyex_settings::schema::Commands;
use tauri::{AppHandle, Manager};

use super::reach;
use crate::cosmetics::VarScope;
use crate::settings::Settings;

/// `$request` gives up after this long.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// And reads at most this much.
const MAX_BODY: usize = 256 * 1024;

/// V110-F87 `$request`'s GET: only to the public web (`reach`), on a client of its own that
/// connects to the address that was checked and follows no redirect by itself. Each hop is
/// checked again, and your headers stay with the host they were written for.
async fn fetch(mut url: url::Url, mut headers: Vec<(String, String)>) -> Result<String, String> {
    for _ in 0..=reach::MAX_HOPS {
        let at = url.clone();
        let addr = tokio::task::spawn_blocking(move || reach::resolve(&at))
            .await
            .map_err(|e| e.to_string())??;
        let client = reqwest::Client::builder()
            .user_agent(cyex_core::app::user_agent())
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .resolve(url.host_str().unwrap_or_default(), addr)
            .build()
            .map_err(|e| e.to_string())?;
        let mut req = client.get(url.as_str());
        for (k, v) in &headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let mut res = req.send().await.map_err(|e| e.without_url().to_string())?;
        let status = res.status();
        if status.is_redirection() {
            let to = res
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| format!("The request failed ({}).", status.as_u16()))?;
            let (next, same) = reach::next_hop(&url, to)?;
            if !same {
                headers.clear();
            }
            url = next;
            continue;
        }
        if !status.is_success() {
            return Err(format!("The request failed ({}).", status.as_u16()));
        }
        let mut out = Vec::new();
        while out.len() < MAX_BODY {
            match res.chunk().await.map_err(|e| e.without_url().to_string())? {
                Some(chunk) => out.extend_from_slice(&chunk),
                None => break,
            }
        }
        out.truncate(MAX_BODY);
        return Ok(String::from_utf8_lossy(&out).into_owned());
    }
    Err("Too many redirects.".into())
}

pub struct AppHost {
    pub app: AppHandle,
    pub list: Arc<CommandList>,
    pub channel: String,
    pub runtime: tokio::runtime::Handle,
    /// V1051-04 7TV variables for this run.
    pub vars: VarScope,
}

impl Host for AppHost {
    fn custom(&self, name: &str) -> Option<Arc<Items>> {
        self.list.replacement(name, &self.channel)
    }

    fn get(&self, name: &str, key: Option<&str>) -> Option<String> {
        match (name, key) {
            ("var", Some(k)) => self
                .app
                .state::<Settings>()
                .read::<Commands>(Partition::Commands)
                .vars
                .get(k)
                .cloned(),
            ("username", None) => {
                let accounts = crate::auth::accounts(&self.app);
                let id = accounts.default_id.as_deref()?;
                accounts.get(id).map(|a| a.login.clone())
            }
            _ => None,
        }
    }

    fn request(&mut self, url: &str, headers: &[(String, String)]) -> Result<String, String> {
        let parsed = url::Url::parse(url).map_err(|_| format!("Not a web address: {url}"))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(format!("Not a web address: {url}"));
        }
        let headers = headers.to_vec();
        self.runtime.block_on(async move {
            tokio::time::timeout(REQUEST_TIMEOUT, fetch(parsed, headers))
                .await
                .map_err(|_| "The request took too long.".to_owned())?
        })
    }

    fn cosmetic(&mut self, v: &Var) -> String {
        self.vars.resolve(&self.app, &self.runtime, v)
    }

    fn random(&mut self, below: u64) -> u64 {
        if below == 0 {
            return 0;
        }
        RandomState::new().build_hasher().finish() % below
    }

    fn now_ms(&self) -> i64 {
        super::super::now_ms()
    }

    fn utc_offset_secs(&self) -> i32 {
        time::UtcOffset::current_local_offset().map_or(0, |o| o.whole_seconds())
    }
}
