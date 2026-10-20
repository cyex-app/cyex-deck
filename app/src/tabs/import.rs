//! V1041-02 Tabs from a Chatterino layout. Picking the file reads it and looks its
//! channels up with the default account (Helix, 100 names a call); the tabs go in after
//! the user picks chat or chat + viewer. Logic lives in `cyex_settings::chatterino`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use cyex_settings::Partition;
use cyex_settings::chatterino::{self, ImportReport, Layout};
use cyex_settings::schema::{TabChannel, Tabs, TileShow, random_id};
use cyex_twitch::helix;
use tauri::{AppHandle, Manager};

use crate::auth::{self, Auth};
use crate::settings::Settings;

/// Layouts are tens of KB; a file this big isn't one.
const MAX_BYTES: u64 = 8 * 1024 * 1024;
const NOT_A_LAYOUT: &str = "That file isn’t a Chatterino layout.";
const NEEDS_ACCOUNT: &str = "Looking up the channels needs a Twitch account. Add one in Settings → Accounts.";
const SIGN_IN_AGAIN: &str =
    "Looking up the channels needs a fresh sign-in. Sign in again in Settings → Accounts.";

/// The layout picked last and what the lookup found, until the import or the next pick.
#[derive(Default)]
pub struct Picked(Mutex<Option<Lookup>>);

struct Lookup {
    /// `None` joins anonymously (test builds only).
    account: Option<String>,
    layout: Layout,
    found: HashMap<String, TabChannel>,
}

pub fn init(app: &AppHandle) {
    app.manage(Picked::default());
}

/// Chatterino's settings folder, where the file picker starts, if it's there.
pub fn folder(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().data_dir().ok()?.join("Chatterino2").join("Settings");
    dir.is_dir().then_some(dir)
}

/// The default account and its Helix token and client ID, or why the lookup can't run.
pub fn account(app: &AppHandle) -> Result<(String, (String, String)), String> {
    let id = auth::accounts(app)
        .default_account()
        .map(|a| a.user_id.clone())
        .ok_or(NEEDS_ACCOUNT)?;
    let token = auth::helix_token(app, &id).ok_or(SIGN_IN_AGAIN)?;
    Ok((id, token))
}

/// Reads the file and looks its channels up. The report says what an import would do.
pub async fn preview(app: &AppHandle, path: &Path) -> Result<ImportReport, String> {
    let (account, (token, client_id)) = account(app)?;
    let layout = read(path)?;
    let http = app.state::<Auth>().http.clone();
    let helix_auth = crate::auth::helix_auth(&token, &client_id);
    let found = chatterino::resolve(&chatterino::names(&layout), &known(app), |batch| {
        let http = &http;
        async move {
            let logins: Vec<&str> = batch.iter().map(String::as_str).collect();
            let users = helix::users_by_login(http, helix_auth, &logins).await?;
            let mut channels: Vec<TabChannel> = users.into_iter().map(channel).collect();
            // V107-R20 Helix can miss a login (late after a rename): ask public GQL once more.
            let missed: Vec<String> = batch
                .into_iter()
                .filter(|n| !channels.iter().any(|c| c.login.eq_ignore_ascii_case(n)))
                .collect();
            if !missed.is_empty() {
                channels.extend(by_gql(http, &missed).await?);
            }
            Ok::<_, cyex_twitch::Error>(channels)
        }
    })
    .await
    .map_err(lookup_error)?;

    Ok(keep(app, Some(account), layout, found))
}

/// C1 Test builds: looks the channels up on Twitch's public GQL instead of Helix, so no
/// account is needed, and the tabs join anonymously.
#[cfg(feature = "mocks")]
pub async fn preview_public(app: &AppHandle, path: &Path) -> Result<ImportReport, String> {
    let layout = read(path)?;
    let http = app.state::<Auth>().http.clone();
    let found = chatterino::resolve(&chatterino::names(&layout), &known(app), |batch| {
        let http = &http;
        async move { by_gql(http, &batch).await }
    })
    .await
    .map_err(lookup_error)?;
    Ok(keep(app, None, layout, found))
}

/// V107-R20 Channels the tabs show now: taken as they are, not looked up again.
fn known(app: &AppHandle) -> HashMap<String, TabChannel> {
    chatterino::known(&app.state::<Settings>().read::<Tabs>(Partition::Tabs))
}

/// Logins on Twitch's public GQL, no account needed.
async fn by_gql(http: &cyex_twitch::Http, logins: &[String]) -> Result<Vec<TabChannel>, cyex_twitch::Error> {
    let users = cyex_twitch::gql::miner::streams_by_logins(http, logins).await?;
    Ok(users
        .into_iter()
        .map(|u| TabChannel {
            id: u.id,
            login: u.login,
            name: u.name,
        })
        .collect())
}

fn keep(app: &AppHandle, account: Option<String>, layout: Layout, found: HashMap<String, TabChannel>) -> ImportReport {
    let report = chatterino::plan(&layout, &found).report;
    *lock(app) = Some(Lookup { account, layout, found });
    report
}

/// Adds what the last preview found, chat only or with the stream, joined as the default
/// account. C1 Channels open in other tabs go in too, sharing their chat room.
pub fn confirm(app: &AppHandle, player: bool) -> Result<ImportReport, String> {
    let lookup = lock(app).take().ok_or("Pick the layout file again.")?;
    let account = auth::accounts(app).default_account().map(|a| a.user_id.clone());
    if lookup.account.is_some() && account != lookup.account {
        return Err("The default account changed. Pick the layout file again.".into());
    }
    let show = TileShow { player, chat: true };
    let mut report = ImportReport::default();
    app.state::<Settings>()
        .update::<Tabs>(app, Partition::Tabs, |tabs| {
            let plan = chatterino::plan(&lookup.layout, &lookup.found);
            report = plan.report;
            if plan.tabs.is_empty() {
                return false;
            }
            tabs.import(plan.tabs, show, lookup.account.clone(), &mut random_id);
            true
        })?;
    Ok(report)
}

fn read(path: &Path) -> Result<Layout, String> {
    let size = std::fs::metadata(path)
        .map_err(|_| "Couldn’t open that file.")?
        .len();
    if size > MAX_BYTES {
        return Err(NOT_A_LAYOUT.into());
    }
    let bytes = std::fs::read(path).map_err(|_| "Couldn’t open that file.")?;
    let text = String::from_utf8(bytes).map_err(|_| NOT_A_LAYOUT)?;
    chatterino::parse(&text).map_err(|_| NOT_A_LAYOUT.into())
}

fn channel(user: helix::User) -> TabChannel {
    let name = if user.display_name.trim().is_empty() {
        user.login.clone()
    } else {
        user.display_name
    };
    TabChannel {
        id: user.id,
        login: user.login,
        name,
    }
}

fn lookup_error(e: cyex_twitch::Error) -> String {
    if e.is_network() {
        "Couldn’t reach Twitch.".into()
    } else if e.is_unauthorized() {
        SIGN_IN_AGAIN.into()
    } else {
        "Twitch didn’t answer the lookup. Try again in a moment.".into()
    }
}

fn lock(app: &AppHandle) -> MutexGuard<'_, Option<Lookup>> {
    app.state::<Picked>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}
