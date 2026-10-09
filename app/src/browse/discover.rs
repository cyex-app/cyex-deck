//! 1.7 Discover. First pages are read signed out. Twitch serves later pages only to
//! signed-in requests, so those go with a GQL sign-in: the default account's, else any.

use cyex_settings::Partition;
use cyex_settings::schema::Accounts;
use cyex_twitch::gql::{self, Category, CategoryStreams, ChannelCard, Page, SearchResults, StreamFilter};
use tauri::{AppHandle, Manager};

use super::follows::error_text;
use crate::auth::{Auth, gql_token};
use crate::settings::Settings;

const MORE_NEEDS_SIGN_IN: &str = "Twitch shows more with a GQL sign-in. Add it in Settings → Accounts.";

/// A token and its client ID, for pages after the first.
type Token = Option<(String, String)>;

fn token(app: &AppHandle, paging: bool) -> Result<Token, String> {
    if !paging {
        return Ok(None);
    }
    let accounts: Accounts = app.state::<Settings>().read(Partition::Accounts);
    accounts
        .default_id
        .iter()
        .chain(accounts.list.iter().map(|a| &a.user_id))
        .find_map(|id| gql_token(app, id))
        .map(Some)
        .ok_or_else(|| MORE_NEEDS_SIGN_IN.to_owned())
}

fn auth(t: &Token) -> gql::Auth<'_> {
    match t {
        Some((token, client_id)) => gql::Auth {
            client_id,
            token: Some(token),
        },
        None => gql::Auth::PUBLIC,
    }
}

pub async fn streams(
    app: &AppHandle,
    filter: &StreamFilter,
    after: Option<&str>,
) -> Result<Page<ChannelCard>, String> {
    let t = token(app, after.is_some())?;
    let http = app.state::<Auth>().http.clone();
    gql::top_streams(&http, auth(&t), filter, after)
        .await
        .map_err(error_text)
}

pub async fn categories(app: &AppHandle, after: Option<&str>) -> Result<Page<Category>, String> {
    let t = token(app, after.is_some())?;
    let http = app.state::<Auth>().http.clone();
    gql::top_categories(&http, auth(&t), after)
        .await
        .map_err(error_text)
}

pub async fn category(
    app: &AppHandle,
    slug: &str,
    filter: &StreamFilter,
    after: Option<&str>,
) -> Result<Option<CategoryStreams>, String> {
    let t = token(app, after.is_some())?;
    let http = app.state::<Auth>().http.clone();
    gql::category(&http, auth(&t), slug, filter, after)
        .await
        .map_err(error_text)
}

pub async fn search(app: &AppHandle, text: &str) -> Result<SearchResults, String> {
    let http = app.state::<Auth>().http.clone();
    gql::search(&http, gql::Auth::PUBLIC, text)
        .await
        .map_err(error_text)
}

pub async fn search_more(app: &AppHandle, text: &str, after: &str) -> Result<Page<ChannelCard>, String> {
    let http = app.state::<Auth>().http.clone();
    gql::search_channels(&http, gql::Auth::PUBLIC, text, after)
        .await
        .map_err(error_text)
}

/// H4 Empty without a GQL sign-in.
pub async fn followed_categories(app: &AppHandle, account: &str) -> Result<Vec<Category>, String> {
    let Some((token, client_id)) = gql_token(app, account) else {
        return Ok(Vec::new());
    };
    let http = app.state::<Auth>().http.clone();
    let auth = gql::Auth {
        client_id: &client_id,
        token: Some(&token),
    };
    gql::followed_categories(&http, auth).await.map_err(error_text)
}
