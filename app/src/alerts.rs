//! 2.2 Live alerts: once a minute, which channels the default account follows went live.
//! Each alerts when its Twitch bell is on, unless the user picked on or off for it in
//! Settings → Notifications. With only a Helix sign-in there's no bell to read, so every
//! followed channel counts. Channels already live when the app starts stay quiet.
//! V106-25 A channel that is only in muted tabs alerts without its sound.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use cyex_settings::Partition;
use cyex_settings::schema::Notifications;
use cyex_twitch::gql::{self, ChannelCard};
use tauri::{AppHandle, Manager};

use crate::auth;
use crate::settings::Settings;
use crate::toasts::{self, Open, Pop};

const EVERY: Duration = Duration::from_secs(60);

#[derive(Default)]
struct Seen {
    account: Option<String>,
    /// Channel id → when its stream started (and who, for the offline line), for every live followed channel.
    live: HashMap<String, Known>,
    /// Nothing polled yet for this account.
    fresh: bool,
}

/// V106-07 A live followed channel as last seen.
#[derive(Clone)]
struct Known {
    started: String,
    login: String,
    name: String,
    avatar: Option<String>,
}

#[derive(Default)]
struct State(Mutex<Seen>);

pub fn init(app: &AppHandle) {
    app.manage(State::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            poll(&app).await;
            tokio::time::sleep(EVERY).await;
        }
    });
}

async fn poll(app: &AppHandle) {
    let n: Notifications = app.state::<Settings>().read(Partition::Notifications);
    let account = auth::accounts(app).default_id;
    {
        let mut seen = lock(app);
        if !n.live.enabled || seen.account != account {
            *seen = Seen {
                account: account.clone(),
                live: HashMap::new(),
                fresh: true,
            };
        }
    }
    let Some(account) = account.filter(|_| n.live.enabled) else {
        return;
    };
    let Some(live) = live_now(app, &account).await else {
        return;
    };

    let (alerts, flips) = {
        let mut seen = lock(app);
        if seen.account.as_deref() != Some(account.as_str()) {
            return; // the default account changed while we asked
        }
        let quiet = seen.fresh && n.live.skip_at_start;
        let alerts = news(&seen.live, &live, quiet)
            .into_iter()
            .filter(|(card, bell)| n.live.wants(&card.id, *bell))
            .map(|(card, _)| card.clone())
            .collect::<Vec<_>>();
        // V106-07 Live notifications feed: quiet on the first check (Q5), no bell filter.
        let flips = if seen.fresh { Vec::new() } else { live_flips(&seen.live, &live) };
        seen.live = live
            .iter()
            .filter_map(|(c, _)| {
                let known = Known {
                    started: c.stream.as_ref()?.started_at.clone(),
                    login: c.login.clone(),
                    name: c.name.clone(),
                    avatar: c.avatar.clone(),
                };
                Some((c.id.clone(), known))
            })
            .collect();
        seen.fresh = false;
        (alerts, flips)
    };
    for card in alerts {
        alert(app, &n, card);
    }
    for f in flips {
        crate::feeds::live::flip(app, f);
    }
}

/// Followed channels live now, each with its Twitch bell. `None` when nothing can say.
async fn live_now(app: &AppHandle, account: &str) -> Option<Vec<(ChannelCard, bool)>> {
    if let Some((token, client_id)) = auth::gql_token(app, account) {
        let http = app.state::<auth::Auth>().http.clone();
        let auth = gql::Auth {
            client_id: &client_id,
            token: Some(&token),
        };
        match gql::followed_live_alerts(&http, auth).await {
            Ok(Some(list)) => return Some(list.into_iter().map(|a| (a.card, a.bell)).collect()),
            Ok(None) => return None,
            Err(e) => eprintln!("alerts: {e}"),
        }
    }
    let cards = crate::browse::follows::follows(app, account, false).await.ok()?;
    Some(
        cards
            .into_iter()
            .filter(|c| c.stream.is_some())
            .map(|c| (c, true))
            .collect(),
    )
}

/// Channels that went live since the last poll: new ones, and ones whose stream started
/// again. `quiet` (the first poll after start) lets nothing through.
fn news<'a>(
    before: &HashMap<String, Known>,
    now: &'a [(ChannelCard, bool)],
    quiet: bool,
) -> Vec<&'a (ChannelCard, bool)> {
    if quiet {
        return Vec::new();
    }
    now.iter()
        .filter(|(c, _)| {
            let Some(s) = &c.stream else { return false };
            before.get(&c.id).map(|k| &k.started) != Some(&s.started_at)
        })
        .collect()
}

/// V106-07 Streams that started (new, or started again) or ended since the last check.
fn live_flips(before: &HashMap<String, Known>, now: &[(ChannelCard, bool)]) -> Vec<crate::feeds::live::Flip> {
    let mut flips: Vec<crate::feeds::live::Flip> = now
        .iter()
        .filter_map(|(c, _)| {
            let s = c.stream.as_ref()?;
            (before.get(&c.id).map(|k| &k.started) != Some(&s.started_at)).then(|| crate::feeds::live::Flip {
                id: c.id.clone(),
                login: c.login.clone(),
                name: c.name.clone(),
                avatar: c.avatar.clone(),
                live: true,
                title: Some(s.title.clone()),
                game: s.game.as_ref().map(|g| g.name.clone()),
            })
        })
        .collect();
    for (id, k) in before {
        if !now.iter().any(|(c, _)| &c.id == id && c.stream.is_some()) {
            flips.push(crate::feeds::live::Flip {
                id: id.clone(),
                login: k.login.clone(),
                name: k.name.clone(),
                avatar: k.avatar.clone(),
                live: false,
                title: None,
                game: None,
            });
        }
    }
    flips
}

fn alert(app: &AppHandle, n: &Notifications, card: ChannelCard) {
    if n.live.sound && !crate::tabs::mute::hushes_id(app, &card.id) {
        crate::sound::alert(app, None);
    }
    if !n.live.toast {
        return;
    }
    let stream = card.stream.as_ref();
    let mut text = stream.map(|s| s.title.trim().to_owned()).unwrap_or_default();
    if let Some(game) = stream.and_then(|s| s.game.as_ref()) {
        text = if text.is_empty() {
            game.name.clone()
        } else {
            format!("{text}\n{}", game.name)
        };
    }
    toasts::show(
        app,
        Pop {
            title: format!("{} is live", card.name),
            text,
            avatar: card.avatar.clone(),
            open: Open::Channel { login: card.login },
        },
    );
}

fn lock(app: &AppHandle) -> std::sync::MutexGuard<'_, Seen> {
    app.state::<State>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

#[cfg(test)]
mod tests {
    use cyex_twitch::gql::LiveStream;

    use super::*;

    fn live(id: &str, started: &str) -> (ChannelCard, bool) {
        let card = ChannelCard {
            id: id.into(),
            login: id.into(),
            name: id.into(),
            avatar: None,
            stream: Some(LiveStream {
                title: "t".into(),
                viewers: 1,
                started_at: started.into(),
                rerun: false,
                game: None,
                tags: Vec::new(),
            }),
            last_live: None,
            last_game: None,
            last_title: None,
        };
        (card, true)
    }

    fn known(id: &str, started: &str) -> (String, Known) {
        let k = Known { started: started.into(), login: id.into(), name: id.into(), avatar: None };
        (id.to_owned(), k)
    }

    #[test]
    fn new_and_restarted_streams_alert() {
        let before = HashMap::from([known("a", "1"), known("b", "1")]);
        let now = [live("a", "1"), live("b", "2"), live("c", "1")];
        let ids: Vec<&str> = news(&before, &now, false)
            .iter()
            .map(|(c, _)| c.id.as_str())
            .collect();
        assert_eq!(ids, ["b", "c"]);
        assert!(news(&before, &now, true).is_empty());
    }

    #[test]
    fn feed_flips_on_start_restart_and_end() {
        let before = HashMap::from([known("a", "1"), known("b", "1"), known("gone", "1")]);
        let now = [live("a", "1"), live("b", "2"), live("c", "1")];
        let mut f: Vec<(String, bool)> = live_flips(&before, &now).into_iter().map(|f| (f.id, f.live)).collect();
        f.sort();
        assert_eq!(f, [("b".to_owned(), true), ("c".to_owned(), true), ("gone".to_owned(), false)]);
        assert_eq!(live_flips(&before, &now).iter().find(|f| f.id == "b").and_then(|f| f.title.clone()).as_deref(), Some("t"));
    }
}
