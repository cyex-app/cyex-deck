//! V109-C03 Your clips, per signed-in account: the clips it made (in any channel) and the
//! clips of its own channel (by anyone). Same GQL as the stream manager's Clips panel
//! (V107-R39, `gql::dashboard::content::clips`), but for any account, not only the
//! manager's. Delete goes through the paced bulk runner (`streammanager::bulk`). Editing is
//! Twitch's own clip editor in the account's Twitch window (V103-09); V109-C05 a rename is
//! Twitch's title-only `updateClip` (docs/research/clips-manager.md). Mocks: `CYEX_GQL_DASHBOARD_MOCK`
//! with `tools/mock/clips.mjs` (or `content.mjs`), as `streammanager::gql_call` reads it.

use cyex_twitch::gql::dashboard::content::clips::{self, ClipFilter, ClipPage};
use tauri::AppHandle;

use crate::streammanager::bulk::{self, Bulk};
use crate::streammanager::gql_call::{GqlCall, gql_call_as};
use crate::streammanager::words;

const NO_ACCOUNT: &str = "That account isn’t signed in here.";

/// The account's own GQL call, its channel being its own.
fn call(app: &AppHandle, account: &str) -> Result<GqlCall, String> {
    let accounts = crate::auth::accounts(app);
    let acc = accounts.get(account).ok_or(NO_ACCOUNT)?;
    gql_call_as(app, account, acc.user_id.clone(), acc.login.clone())
}

pub async fn list(app: &AppHandle, account: &str, filter: ClipFilter) -> Result<ClipPage, String> {
    let c = call(app, account)?;
    clips::list(&c.channel(), &c.login, &filter).await.map_err(|e| words::gql(&e))
}

/// Several at once, 25 a request, a second apart; Stop is the stream manager's (one bulk
/// write runs at a time in the app).
pub async fn delete(app: &AppHandle, account: &str, slugs: Vec<String>) -> Result<Bulk, String> {
    let c = &call(app, account)?;
    bulk::run(app, slugs, clips::DELETE_MAX, words::gql, |part| async move {
        clips::delete(&c.channel(), &part).await
    })
    .await
}

/// V109-C05 A new title for one clip; the title Twitch kept, or Twitch's refusal in words.
pub async fn rename(app: &AppHandle, account: &str, slug: &str, title: &str) -> Result<String, String> {
    if !valid_slug(slug) {
        return Err("That isn’t a clip.".into());
    }
    let c = call(app, account)?;
    clips::rename(&c.channel(), slug, title).await.map_err(|e| rename_words(&e))
}

/// Twitch's own lines for its title field (`ClipsTitleEdit`): `invalid_title` or a `403:`
/// is the words; anything else is Twitch's message as it came.
fn rename_words(e: &cyex_twitch::Error) -> String {
    use cyex_twitch::Error::Refused;
    match e {
        Refused(c) if c == "EMPTY" => "Add a title.".into(),
        Refused(c) if c == "TOO_LONG" => format!("Titles can be {} characters at most.", clips::TITLE_MAX),
        Refused(m) if m.eq_ignore_ascii_case("invalid_title") || m.contains("403:") => {
            "Twitch didn’t take that title. Check the words and try again.".into()
        }
        e => words::gql(e),
    }
}

/// Twitch's clip editor for `slug`, the page `POST /clips` hands back as `edit_url`.
pub fn edit_url(slug: &str) -> Result<String, String> {
    if !valid_slug(slug) {
        return Err("That isn’t a clip.".into());
    }
    Ok(format!("https://clips.twitch.tv/{slug}/edit"))
}

/// Opens Twitch's editor for the clip in the account's Twitch window. False: a mock run,
/// nothing opened (a test never loads Twitch).
pub async fn edit(app: &AppHandle, account: &str, slug: &str) -> Result<bool, String> {
    let url = edit_url(slug)?;
    if mocked(app)? {
        eprintln!("clips: mock, editor not opened for {slug}");
        return Ok(false);
    }
    crate::auth::accounts(app).get(account).ok_or(NO_ACCOUNT)?;
    crate::windows::twitch::open(app, account, &url).await?;
    Ok(true)
}

#[cfg(feature = "mocks")]
fn mocked(app: &AppHandle) -> Result<bool, String> {
    crate::mock_url::gated(app, &crate::streammanager::gql_call::MOCK_VARS, "GQL").map(|b| b.is_some())
}

#[cfg(not(feature = "mocks"))]
fn mocked(_: &AppHandle) -> Result<bool, String> {
    Ok(false)
}

/// Twitch slugs: letters, digits, `-` and `_` (e.g. `FastClip-AbC_12`).
fn valid_slug(s: &str) -> bool {
    (1..=100).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v109_c05_rename_lines() {
        use cyex_twitch::Error::Refused;
        assert_eq!(rename_words(&Refused("EMPTY".into())), "Add a title.");
        assert!(rename_words(&Refused("TOO_LONG".into())).contains("100"));
        assert!(rename_words(&Refused("invalid_title".into())).contains("didn’t take"));
        assert!(rename_words(&Refused("403: Forbidden".into())).contains("didn’t take"));
        assert_eq!(rename_words(&Refused("clip is locked".into())), "Twitch turned that down (clip is locked).");
    }

    #[test]
    fn v109_c03_edit_url_is_twitchs_editor() {
        assert_eq!(edit_url("FastClip-AbC_12").unwrap(), "https://clips.twitch.tv/FastClip-AbC_12/edit");
        for bad in ["", "a/b", "x?y=1", "a b", "../edit", "ä"] {
            assert!(edit_url(bad).is_err(), "{bad}");
        }
        assert!(edit_url(&"a".repeat(101)).is_err());
    }
}
