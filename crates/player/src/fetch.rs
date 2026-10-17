//! The network side of playback. [`Twitch`] is the real one; tests script their own.

use std::future::Future;
use std::sync::Arc;

use cyex_twitch::gql::{self, Auth, Player};
use cyex_twitch::{Error, Http, Result, usher};

/// Usher's `supported_codecs`. H.264 decodes in hardware everywhere and keeps every
/// variant and backup in one codec; HEVC and AV1 would only add 1440p and up.
pub const CODECS: &str = "h264";

/// How a live playlist is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// The stream itself, as the pop-out player: signed in when possible (D19).
    Main,
    /// Signed out, as another player type (1.2).
    Backup(Player<'static>),
    /// Through the user's proxy, signed out (P10).
    Proxy,
}

pub trait Fetch: Send + Sync {
    fn live_master(&self, login: &str, route: Route) -> impl Future<Output = Result<String>> + Send;
    fn vod_master(&self, id: &str) -> impl Future<Output = Result<String>> + Send;
    /// A playlist, through the proxy for [`Route::Proxy`].
    fn text(&self, url: &str, route: Route) -> impl Future<Output = Result<String>> + Send;
    fn bytes(&self, url: &str) -> impl Future<Output = Result<Vec<u8>>> + Send;
    fn has_proxy(&self) -> bool;
    /// V1061 The mock HLS server, in a test data folder only.
    fn mock_base(&self) -> Option<&str> {
        None
    }
}

/// V1061 The mock server's master for a live login (`?route=backup` for backups).
pub fn mock_master(base: &str, login: &str, route: Route) -> String {
    let q = if matches!(route, Route::Backup(_)) { "?route=backup" } else { "" };
    format!("{base}/live/{login}/master.m3u8{q}")
}

/// The account's GQL token and its client ID, read on each use so refreshed tokens count.
pub type SignIn = Arc<dyn Fn() -> Option<(String, String)> + Send + Sync>;

#[derive(Clone)]
pub struct Twitch {
    pub http: Http,
    /// The same client through the ad-block proxy (P10), if one is set.
    pub proxy: Option<Http>,
    /// Sent as `X-Device-Id` when signed out; the same for this install.
    pub device_id: String,
    pub sign_in: Option<SignIn>,
    /// V1061 `CYEX_HLS_MOCK`, test data folder only.
    pub mock: Option<String>,
}

impl Twitch {
    fn signed_in(&self) -> Option<(String, String)> {
        self.sign_in.as_ref().and_then(|f| f())
    }
}

/// Signed in, then signed out if Twitch refuses the token: an offline channel or a
/// missing VOD isn't worth a second try.
fn retry_signed_out(e: &Error) -> bool {
    !matches!(e, Error::Api { status: 404, .. })
}

impl Fetch for Twitch {
    async fn live_master(&self, login: &str, route: Route) -> Result<String> {
        if let Some(base) = &self.mock {
            return usher::text(&self.http, &mock_master(base, login, route)).await;
        }
        let (http, player) = match route {
            Route::Main => (&self.http, crate::backup::MAIN),
            Route::Backup(p) => (&self.http, p),
            Route::Proxy => (self.proxy.as_ref().unwrap_or(&self.http), crate::backup::MAIN),
        };
        if route == Route::Main
            && let Some((token, client_id)) = self.signed_in()
        {
            let auth = Auth {
                client_id: &client_id,
                token: Some(&token),
            };
            let signed = async {
                let t = gql::stream_token(http, auth, None, login, player).await?;
                usher::text(http, &usher::live_url(login, &t, CODECS)).await
            };
            match signed.await {
                Err(e) if retry_signed_out(&e) => {}
                done => return done,
            }
        }
        let token = gql::stream_token(http, Auth::PUBLIC, Some(&self.device_id), login, player).await?;
        usher::text(http, &usher::live_url(login, &token, CODECS)).await
    }

    async fn vod_master(&self, id: &str) -> Result<String> {
        if let Some(base) = &self.mock {
            return usher::text(&self.http, &format!("{base}/vod/{id}/master.m3u8")).await;
        }
        if let Some((token, client_id)) = self.signed_in() {
            let auth = Auth {
                client_id: &client_id,
                token: Some(&token),
            };
            let signed = async {
                let t = gql::video_token(&self.http, auth, None, id).await?;
                usher::text(&self.http, &usher::vod_url(id, &t, CODECS)).await
            };
            match signed.await {
                Err(e) if retry_signed_out(&e) => {}
                done => return done,
            }
        }
        let token = gql::video_token(&self.http, Auth::PUBLIC, Some(&self.device_id), id).await?;
        usher::text(&self.http, &usher::vod_url(id, &token, CODECS)).await
    }

    async fn text(&self, url: &str, route: Route) -> Result<String> {
        let http = match route {
            Route::Proxy => self.proxy.as_ref().unwrap_or(&self.http),
            _ => &self.http,
        };
        usher::text(http, url).await
    }

    async fn bytes(&self, url: &str) -> Result<Vec<u8>> {
        usher::bytes(&self.http, url).await
    }

    fn has_proxy(&self) -> bool {
        self.proxy.is_some()
    }

    fn mock_base(&self) -> Option<&str> {
        self.mock.as_deref()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;
    use crate::live::gone;

    #[test]
    fn mock_master_urls() {
        let b = "http://127.0.0.1:5200";
        assert_eq!(mock_master(b, "xqc", Route::Main), "http://127.0.0.1:5200/live/xqc/master.m3u8");
        assert_eq!(
            mock_master(b, "xqc", Route::Backup(crate::backup::MAIN)),
            "http://127.0.0.1:5200/live/xqc/master.m3u8?route=backup"
        );
    }

    /// Scripted network: live masters per route, everything else per URL.
    #[derive(Default)]
    pub struct Script {
        pub masters: Mutex<HashMap<&'static str, String>>,
        pub media: Mutex<HashMap<String, String>>,
        pub asked: Mutex<Vec<String>>,
        pub proxy: bool,
    }

    /// `popout` for the stream, the player type for backups, `proxy`.
    pub fn route_name(route: Route) -> &'static str {
        match route {
            Route::Main => "popout",
            Route::Backup(p) => p.player_type,
            Route::Proxy => "proxy",
        }
    }

    impl Fetch for &Script {
        async fn live_master(&self, _login: &str, route: Route) -> Result<String> {
            let name = route_name(route);
            self.asked.lock().unwrap().push(format!("master:{name}"));
            self.masters.lock().unwrap().get(name).cloned().ok_or_else(|| gone("Can not find channel"))
        }

        async fn vod_master(&self, id: &str) -> Result<String> {
            self.text(&format!("vod:{id}"), Route::Main).await
        }

        async fn text(&self, url: &str, _route: Route) -> Result<String> {
            self.asked.lock().unwrap().push(url.to_owned());
            self.media.lock().unwrap().get(url).cloned().ok_or_else(|| gone("gone"))
        }

        async fn bytes(&self, url: &str) -> Result<Vec<u8>> {
            self.text(url, Route::Main).await.map(String::into_bytes)
        }

        fn has_proxy(&self) -> bool {
            self.proxy
        }
    }
}
