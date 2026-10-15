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

use crate::cosmetics::VarScope;
use crate::settings::Settings;

/// `$request` gives up after this long.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// And reads at most this much.
const MAX_BODY: usize = 256 * 1024;

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
        let http = self.app.state::<crate::auth::Auth>().http.clone();
        let headers = headers.to_vec();
        self.runtime.block_on(async move {
            tokio::time::timeout(
                REQUEST_TIMEOUT,
                http.get_text(parsed.as_str(), &headers, MAX_BODY),
            )
            .await
            .map_err(|_| "The request took too long.".to_owned())?
            .map_err(|e| e.to_string())
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
