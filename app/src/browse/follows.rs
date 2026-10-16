//! 1.6 Who each account follows, for home and the switcher. Fetched when asked and kept
//! in memory: the whole list again after 15 minutes, live status after a minute.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cyex_settings::Partition;
use cyex_settings::schema::Accounts;
use cyex_twitch::gql::{self, ChannelCard};
use cyex_twitch::{Http, helix};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::auth::{Auth, gql_token, helix_token};
use crate::settings::Settings;

const LIST_AGE: Duration = Duration::from_secs(15 * 60);
const LIVE_AGE: Duration = Duration::from_secs(50);
const HELIX_SCOPE: &str = "user:read:follows";
const NO_SIGN_IN: &str = "Sign in to see who you follow.";

#[derive(Default)]
pub struct Follows {
    /// By account user id.
    lists: Mutex<HashMap<String, Cached>>,
}

struct Cached {
    channels: Vec<ChannelCard>,
    listed: Instant,
    checked: Instant,
}

/// Where an account's follows come from. Helix is Twitch's documented API, so it goes
/// first when the account has it.
enum Source {
    Helix {
        token: String,
        client_id: String,
        user_id: String,
    },
    Gql {
        token: String,
        client_id: String,
    },
}

impl Source {
    fn of(app: &AppHandle, account: &str) -> Option<Self> {
        let accounts: Accounts = app.state::<Settings>().read(Partition::Accounts);
        let scoped = accounts
            .get(account)
            .and_then(|a| a.helix.as_ref())
            .is_some_and(|m| m.scopes.iter().any(|s| s == HELIX_SCOPE));
        if scoped && let Some((token, client_id)) = helix_token(app, account) {
            return Some(Source::Helix {
                token,
                client_id,
                user_id: account.to_owned(),
            });
        }
        gql_token(app, account).map(|(token, client_id)| Source::Gql { token, client_id })
    }

    async fn all(&self, http: &Http) -> cyex_twitch::Result<Vec<ChannelCard>> {
        match self {
            Source::Helix {
                token,
                client_id,
                user_id,
            } => {
                let auth = helix::Auth { client_id, token };
                let ids = helix::followed_ids(http, auth, user_id).await?;
                gql::cards(http, &ids).await
            }
            Source::Gql { token, client_id } => {
                let auth = gql::Auth {
                    client_id,
                    token: Some(token),
                };
                let ids = gql::followed_ids(http, auth).await?.unwrap_or_default();
                gql::cards(http, &ids).await
            }
        }
    }

    async fn live(&self, http: &Http) -> cyex_twitch::Result<Vec<ChannelCard>> {
        match self {
            Source::Helix {
                token,
                client_id,
                user_id,
            } => {
                let auth = helix::Auth { client_id, token };
                let ids = helix::followed_live_ids(http, auth, user_id).await?;
                gql::cards(http, &ids).await
            }
            Source::Gql { token, client_id } => {
                let auth = gql::Auth {
                    client_id,
                    token: Some(token),
                };
                Ok(gql::followed_live(http, auth).await?.unwrap_or_default())
            }
        }
    }
}

/// The account's follows, live ones up to a minute old. `force` refetches the list.
pub async fn follows(app: &AppHandle, account: &str, force: bool) -> Result<Vec<ChannelCard>, String> {
    let source = Source::of(app, account).ok_or(NO_SIGN_IN)?;
    let http = app.state::<Auth>().http.clone();
    let state = app.state::<Follows>();
    let mut lists = state.lists.lock().await;
    let now = Instant::now();
    if !force && let Some(c) = lists.get_mut(account).filter(|c| now - c.listed < LIST_AGE) {
        if now - c.checked >= LIVE_AGE {
            match source.live(&http).await {
                Ok(live) => merge(&mut c.channels, live),
                Err(e) if e.is_network() => return Err(error_text(e)),
                // Twitch's live query is touchy; asking for every channel works too.
                Err(e) => {
                    eprintln!("browse: live follows: {e}");
                    let ids: Vec<String> = c.channels.iter().map(|x| x.id.clone()).collect();
                    c.channels = gql::cards(&http, &ids).await.map_err(error_text)?;
                }
            }
            c.checked = now;
        }
        return Ok(c.channels.clone());
    }
    let channels = source.all(&http).await.map_err(error_text)?;
    lists.insert(
        account.to_owned(),
        Cached {
            channels: channels.clone(),
            listed: now,
            checked: now,
        },
    );
    Ok(channels)
}

/// Fresh live status into the list. Channels missing from `live` went offline; live
/// ones missing from the list were followed since it was fetched.
fn merge(list: &mut Vec<ChannelCard>, live: Vec<ChannelCard>) {
    let mut live: HashMap<String, ChannelCard> = live.into_iter().map(|c| (c.id.clone(), c)).collect();
    for c in list.iter_mut() {
        match live.remove(&c.id) {
            Some(fresh) => *c = fresh,
            None => c.stream = None,
        }
    }
    let mut added: Vec<ChannelCard> = live.into_values().collect();
    added.sort_by(|a, b| a.login.cmp(&b.login));
    list.splice(0..0, added);
}

pub fn error_text(e: cyex_twitch::Error) -> String {
    if e.is_network() {
        "Couldn’t reach Twitch.".into()
    } else {
        eprintln!("browse: {e}");
        "Twitch didn’t send the list. Try again in a bit.".into()
    }
}

#[cfg(test)]
mod tests {
    use cyex_twitch::gql::LiveStream;

    use super::*;

    fn card(id: &str, live: bool) -> ChannelCard {
        ChannelCard {
            id: id.into(),
            login: id.into(),
            name: id.into(),
            avatar: None,
            stream: live.then(|| LiveStream {
                title: String::new(),
                viewers: 1,
                started_at: "s".into(),
                rerun: false,
                game: None,
                tags: Vec::new(),
            }),
            last_live: Some("s".into()),
            last_game: None,
            last_title: None,
        }
    }

    #[test]
    fn live_status_merges_in() {
        let mut list = vec![card("a", true), card("b", false), card("c", false)];
        merge(&mut list, vec![card("c", true), card("new", true)]);
        let state: Vec<(&str, bool)> = list.iter().map(|c| (c.id.as_str(), c.stream.is_some())).collect();
        assert_eq!(state, [("new", true), ("a", false), ("b", false), ("c", true)]);
        // Offline keeps when it last went live.
        assert_eq!(list[1].last_live.as_deref(), Some("s"));
    }
}
