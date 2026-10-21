//! V107-R39 The stream manager's Clips and Videos panels: your channel's clips and the
//! clips you made, and the Video producer (past broadcasts, highlights, uploads). GQL only,
//! as on Twitch's dashboard; Helix can't rename or delete clips. Deletes go in paced
//! batches. Mocks builds on a test data folder can aim every call at
//! `tools/mock/content.mjs` (`CYEX_GQL_CONTENT_MOCK`); no real token goes there.

use std::time::Duration;

use cyex_twitch::gql::VideoKind;
use cyex_twitch::gql::dashboard::content::{clips, videos};
use cyex_twitch::{Error, Http};
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use super::{Call, words};

const GQL_ONLY: &str = "This needs the sign-in with a code (Settings → Accounts).";
/// Between delete batches.
const PACE: Duration = Duration::from_secs(1);
#[cfg(feature = "mocks")]
const MOCK_VAR: &str = "CYEX_GQL_CONTENT_MOCK";

/// What a delete did: what went, and why the rest didn't.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ContentDeleted")]
pub struct Deleted {
    pub gone: Vec<String>,
    pub error: Option<String>,
}

#[cfg(feature = "mocks")]
fn mock_base(app: &AppHandle) -> Option<String> {
    crate::paths::webview(app)?;
    crate::mock_url::loopback_base(&std::env::var(MOCK_VAR).ok()?)
}

#[cfg(not(feature = "mocks"))]
fn mock_base(_: &AppHandle) -> Option<String> {
    None
}

/// The manager's GQL sign-in, or a fake one aimed at the mock.
struct Target {
    http: Http,
    token: String,
    client_id: String,
    base: Option<String>,
    channel_id: String,
    login: String,
}

impl Target {
    fn new(app: &AppHandle) -> Result<Self, String> {
        let base = mock_base(app);
        let call = Call::new(app)?;
        let login = super::manager(app).map(|m| m.login).unwrap_or_default();
        let (token, client_id) = match &base {
            Some(_) => ("mock-token".to_owned(), cyex_twitch::clients::WEB.to_owned()),
            None => crate::auth::gql_token(app, &call.channel_id).ok_or(GQL_ONLY)?,
        };
        Ok(Self { http: call.http, token, client_id, base, channel_id: call.channel_id, login })
    }

    fn auth(&self) -> cyex_twitch::gql::Auth<'_> {
        crate::auth::gql_auth(&self.token, &self.client_id)
    }

    fn base(&self) -> Option<&str> {
        self.base.as_deref()
    }
}

/// Twitch's answer as a line; this panel's own codes first.
fn say(e: &Error) -> String {
    match e {
        Error::Refused(c) if c == "EMPTY" => "Give it a title.".into(),
        Error::Refused(c) if c == "TOO_LONG" => format!("Titles can be {} characters at most.", videos::TITLE_MAX),
        Error::Refused(c) if c == "UNCHANGED" => "Twitch didn’t change it. Try again.".into(),
        e => words::gql(e),
    }
}

pub async fn clips(app: &AppHandle, filter: clips::ClipFilter) -> Result<clips::ClipPage, String> {
    let t = Target::new(app)?;
    clips::list(&t.http, t.auth(), t.base(), &t.login, &filter).await.map_err(|e| say(&e))
}

pub async fn videos(app: &AppHandle, kind: Option<VideoKind>, cursor: Option<String>) -> Result<videos::VideoPage, String> {
    let t = Target::new(app)?;
    videos::list(&t.http, t.auth(), t.base(), &t.channel_id, kind, cursor.as_deref()).await.map_err(|e| say(&e))
}

pub async fn rename_video(app: &AppHandle, id: String, title: String) -> Result<String, String> {
    let t = Target::new(app)?;
    videos::rename(&t.http, t.auth(), t.base(), &id, &title).await.map_err(|e| say(&e))
}

pub async fn publish_video(app: &AppHandle, id: String, public: bool) -> Result<bool, String> {
    let t = Target::new(app)?;
    videos::publish(&t.http, t.auth(), t.base(), &id, public).await.map_err(|e| say(&e))
}

pub async fn delete_clips(app: &AppHandle, slugs: Vec<String>) -> Result<Deleted, String> {
    let t = &Target::new(app)?;
    let run = |part: Vec<String>| async move { clips::delete(&t.http, t.auth(), t.base(), &part).await };
    Ok(in_batches(unique(slugs), clips::DELETE_MAX, run).await)
}

/// Publishes or unpublishes several videos; `gone` lists the ones changed.
pub async fn publish_videos(app: &AppHandle, ids: Vec<String>, public: bool) -> Result<Deleted, String> {
    let t = &Target::new(app)?;
    let run = |part: Vec<String>| async move { videos::publish_many(&t.http, t.auth(), t.base(), &part, public).await };
    Ok(in_batches(unique(ids), videos::DELETE_MAX, run).await)
}

pub async fn delete_videos(app: &AppHandle, ids: Vec<String>) -> Result<Deleted, String> {
    let t = &Target::new(app)?;
    let run = |part: Vec<String>| async move { videos::delete(&t.http, t.auth(), t.base(), &part).await };
    Ok(in_batches(unique(ids), videos::DELETE_MAX, run).await)
}

fn unique(mut v: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    v.retain(|s| !s.is_empty() && seen.insert(s.clone()));
    v
}

/// Runs `run` on batches of `size`, one a second, stopping at the first error.
async fn in_batches<F: Future<Output = cyex_twitch::Result<Vec<String>>>>(
    items: Vec<String>,
    size: usize,
    run: impl Fn(Vec<String>) -> F,
) -> Deleted {
    let mut out = Deleted::default();
    for (i, part) in items.chunks(size.max(1)).enumerate() {
        if i > 0 {
            tokio::time::sleep(PACE).await;
        }
        match run(part.to_vec()).await {
            Ok(gone) => out.gone.extend(gone),
            Err(e) => {
                out.error = Some(say(&e));
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn own_codes_read_as_lines() {
        assert_eq!(say(&Error::Refused("EMPTY".into())), "Give it a title.");
        assert!(say(&Error::Refused("TOO_LONG".into())).contains("140"));
        assert_eq!(say(&Error::Refused("FORBIDDEN".into())), "Twitch didn’t allow that.");
    }

    #[test]
    fn unique_drops_blanks_and_repeats() {
        let v = unique(vec!["a".into(), "".into(), "b".into(), "a".into()]);
        assert_eq!(v, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[tokio::test]
    async fn batches_stop_at_the_first_error() {
        let sent = RefCell::new(Vec::new());
        let items: Vec<String> = (0..7).map(|i| i.to_string()).collect();
        let out = in_batches(items, 3, |part: Vec<String>| {
            sent.borrow_mut().push(part.len());
            let fail = sent.borrow().len() == 2;
            async move { if fail { Err(Error::Refused("FORBIDDEN".into())) } else { Ok(part) } }
        })
        .await;
        assert_eq!(*sent.borrow(), vec![3, 3]);
        assert_eq!(out.gone, vec!["0", "1", "2"]);
        assert_eq!(out.error.as_deref(), Some("Twitch didn’t allow that."));
    }

    #[tokio::test]
    async fn all_batches_run_when_fine() {
        let items: Vec<String> = (0..5).map(|i| i.to_string()).collect();
        let out = in_batches(items.clone(), 2, |part: Vec<String>| async move { Ok(part) }).await;
        assert_eq!((out.gone, out.error), (items, None));
    }
}
