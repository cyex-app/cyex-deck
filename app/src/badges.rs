//! V108-C37 "You earned a badge" cards. Twitch has no push for it (no PubSub user topic carries
//! a new badge; chat USERNOTICE only covers sub and gift events in one channel, see
//! docs/research/badge-signals.md), so this reads the default account's badges again every 15
//! minutes and tells what is new since the last read. The first read of an account is the
//! baseline and says nothing; badges earned while the app was closed show up in it silently.

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use cyex_twitch::gql::{self, BadgeInfo};
use tauri::{AppHandle, Manager};

use crate::auth;
use crate::cards::{self, Card, Kind};
use crate::toasts::Open;

const EVERY: Duration = Duration::from_secs(15 * 60);
/// After start, so the first read doesn't join the busy first seconds.
const FIRST: Duration = Duration::from_secs(90);

#[derive(Default)]
struct Known {
    account: Option<String>,
    keys: HashSet<String>,
    /// An account's first read is the baseline; an account with no badges still has one.
    read: bool,
}

#[derive(Default)]
struct State(Mutex<Known>);

pub fn init(app: &AppHandle) {
    app.manage(State::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST).await;
        loop {
            check(&app).await;
            tokio::time::sleep(EVERY).await;
        }
    });
}

fn key(b: &BadgeInfo) -> String {
    format!("{}/{}", b.set, b.version)
}

/// The badges in `now` not in `known`, in order. Nothing on an account's first read.
fn fresh<'a>(known: &HashSet<String>, read: bool, now: &'a [BadgeInfo]) -> Vec<&'a BadgeInfo> {
    if !read {
        return Vec::new();
    }
    now.iter().filter(|b| !known.contains(&key(b))).collect()
}

/// Reads the default account's badges and shows a card for each new one. Skipped while the
/// card is off or without a GQL sign-in.
pub async fn check(app: &AppHandle) {
    let n: cyex_settings::schema::Notifications = app.state::<crate::settings::Settings>().read(cyex_settings::Partition::Notifications);
    if !n.cards.badges {
        return;
    }
    let accounts = auth::accounts(app);
    let Some(account) = accounts.default_id.clone() else {
        return;
    };
    let Some(login) = accounts.list.iter().find(|a| a.user_id == account).map(|a| a.login.clone()) else {
        return;
    };
    let Ok(base) = crate::chat::identity::mock_base(app) else {
        return;
    };
    let Some((token, client_id)) = crate::chat::identity::gql(app, &account, base.as_deref()) else {
        return;
    };
    let http = app.state::<auth::Auth>().http.clone();
    let me = match gql::chat_identity(&http, auth::gql_auth(&token, &client_id), base.as_deref(), &login).await {
        Ok(me) => me,
        Err(e) => {
            eprintln!("badges: {e}");
            return;
        }
    };
    let mut now = me.global.available;
    for b in me.channel.available {
        if !now.iter().any(|x| key(x) == key(&b)) {
            now.push(b);
        }
    }
    let new = {
        let state = app.state::<State>();
        let mut st = state.0.lock().unwrap_or_else(|p| p.into_inner());
        if st.account.as_deref() != Some(account.as_str()) {
            st.account = Some(account);
            st.keys.clear();
            st.read = false;
        }
        let new: Vec<BadgeInfo> = fresh(&st.keys, st.read, &now).into_iter().cloned().collect();
        st.keys.extend(now.iter().map(key));
        st.read = true;
        new
    };
    for b in new {
        cards::show(
            app,
            Card {
                kind: Kind::Badge,
                title: "New badge".into(),
                text: b.title.clone(),
                image: Some(b.image2x.clone()),
                open: Open::Settings { page: "emotes".into() },
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn badge(set: &str, version: &str) -> BadgeInfo {
        BadgeInfo {
            set: set.into(),
            version: version.into(),
            title: set.into(),
            description: None,
            click_url: None,
            image1x: String::new(),
            image2x: String::new(),
            image4x: String::new(),
        }
    }

    #[test]
    fn first_read_is_quiet_and_later_ones_tell_the_new() {
        let now = [badge("bits", "100"), badge("turbo", "1")];
        assert!(fresh(&HashSet::new(), false, &now).is_empty());
        // An account that had none at the first read still tells the next one.
        assert_eq!(fresh(&HashSet::new(), true, &now).len(), 2);
        let known: HashSet<String> = ["bits/100".to_owned()].into();
        let new: Vec<&str> = fresh(&known, true, &now).iter().map(|b| b.set.as_str()).collect();
        assert_eq!(new, ["turbo"]);
    }

    #[test]
    fn a_higher_tier_counts_as_new() {
        let known: HashSet<String> = ["bits/100".to_owned()].into();
        assert_eq!(fresh(&known, true, &[badge("bits", "1000")]).len(), 1);
    }
}
