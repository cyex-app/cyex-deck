//! V107-R39 The stream manager's Clips and Videos panels: your channel's clips and the
//! clips you made, and the Video producer (past broadcasts, highlights, uploads). GQL only,
//! as on Twitch's dashboard; Helix can't rename or delete clips. Bulk changes go through
//! `bulk` (paced, retried, stoppable, V107-R41); calls through `gql_call` (mock or Twitch).

use cyex_twitch::Error;
use cyex_twitch::gql::VideoKind;
use cyex_twitch::gql::dashboard::content::{clips, videos};
use tauri::AppHandle;

use super::bulk::{self, Bulk};
use super::gql_call::gql_call;
use super::words;

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
    let c = gql_call(app)?;
    clips::list(&c.channel(), &c.login, &filter).await.map_err(|e| say(&e))
}

pub async fn videos(app: &AppHandle, kind: Option<VideoKind>, cursor: Option<String>) -> Result<videos::VideoPage, String> {
    let c = gql_call(app)?;
    videos::list(&c.channel(), kind, cursor.as_deref()).await.map_err(|e| say(&e))
}

pub async fn rename_video(app: &AppHandle, id: String, title: String) -> Result<String, String> {
    let c = gql_call(app)?;
    videos::rename(&c.channel(), &id, &title).await.map_err(|e| say(&e))
}

pub async fn publish_video(app: &AppHandle, id: String, public: bool) -> Result<bool, String> {
    let c = gql_call(app)?;
    videos::publish(&c.channel(), &id, public).await.map_err(|e| say(&e))
}

pub async fn delete_clips(app: &AppHandle, slugs: Vec<String>) -> Result<Bulk, String> {
    let c = &gql_call(app)?;
    bulk::run(app, slugs, clips::DELETE_MAX, say, |part| async move { clips::delete(&c.channel(), &part).await }).await
}

/// Publishes or unpublishes several videos; `done` lists the ones Twitch reads back changed.
pub async fn publish_videos(app: &AppHandle, ids: Vec<String>, public: bool) -> Result<Bulk, String> {
    let c = &gql_call(app)?;
    bulk::run(app, ids, videos::DELETE_MAX, say, |part| async move {
        videos::publish_many(&c.channel(), &part, public).await
    })
    .await
}

pub async fn delete_videos(app: &AppHandle, ids: Vec<String>) -> Result<Bulk, String> {
    let c = &gql_call(app)?;
    bulk::run(app, ids, videos::DELETE_MAX, say, |part| async move { videos::delete(&c.channel(), &part).await }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_codes_read_as_lines() {
        assert_eq!(say(&Error::Refused("EMPTY".into())), "Give it a title.");
        assert!(say(&Error::Refused("TOO_LONG".into())).contains("140"));
        assert_eq!(say(&Error::Refused("FORBIDDEN".into())), "Twitch didn’t allow that.");
    }
}
