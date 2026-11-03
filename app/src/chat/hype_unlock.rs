//! V108-C35 Hype train emotes the signed-in user unlocked: a card over the bottom of the
//! chat and a fresh emote list. Twitch's signals (`docs/research/twitch-api.md`): the user's
//! topic `hype-train-events-v1.rewards.<user id>` says whether their part counts
//! (`hype-train-reward-eligibility`) and may list the emotes (`hype-train-rewards`). Neither
//! is seen live, so a train's end also reads the account's emotes once (5 s on, again at
//! 60 s when Twitch said it counts) and shows the hype train emotes that are new. A user Twitch
//! said doesn't count reads nothing.
//!
//! V108-C35b A Golden Kappa train gives its participants the Golden Kappa (emote 80393,
//! typed "Kappa", set 15940, Twitch type `UNKNOWN`) for 24 hours; a read that finds it new
//! shows it whatever Twitch said about the part (`docs/research/hype-train.md`).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use cyex_emotes::Emote;
use cyex_twitch::gql::hypetrain::{self as api, UserEvent};
use cyex_twitch::gql::{EmoteKind, TwitchEmote};
use cyex_twitch::hermes::Event;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::emotes::Baseline;

use super::{Chat, Identity};

const EVENT: &str = "chat:hype-unlock";
/// Twitch hands the rewards out as the train ends; the list is read after this.
const FIRST_LOOK: Duration = Duration::from_secs(5);
/// One more read when Twitch said the user's part counts and the first found nothing.
const SECOND_LOOK: Duration = Duration::from_secs(60);
/// One card per account and channel in this long (the topic and the read may both find them).
const SHOWN_GAP: Duration = Duration::from_secs(600);
/// V108-C35b The Golden Kappa's emote id.
const GOLDEN_KAPPA: &str = "80393";
/// What Twitch said about a train is kept this long.
const KEEP: Duration = Duration::from_secs(3 * 3600);

/// The card's content.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HypeUnlock {
    /// The user id it is for.
    pub account: String,
    /// The train's channel; empty when Twitch didn't say.
    pub channel_id: String,
    pub emotes: Vec<Emote>,
}

#[derive(Default)]
struct Book {
    /// (account, train id): whether the account's part counts.
    eligible: HashMap<(String, String), (bool, Instant)>,
    /// (account, channel id): when its last card showed.
    shown: HashMap<(String, String), Instant>,
    /// (account, train id or channel id): a baseline read was started for the train.
    baselined: HashMap<(String, String), Instant>,
}

impl Book {
    fn note(&mut self, account: &str, train: &str, yes: bool, now: Instant) {
        self.eligible.retain(|_, (_, at)| now.duration_since(*at) < KEEP);
        self.eligible.insert((account.to_owned(), train.to_owned()), (yes, now));
    }

    fn eligible(&self, account: &str, train: &str) -> Option<bool> {
        if train.is_empty() {
            return None;
        }
        self.eligible.get(&(account.to_owned(), train.to_owned())).map(|(yes, _)| *yes)
    }

    /// V108-C35j True (and noted) when the account's baseline read for the train is due.
    fn claim_baseline(&mut self, account: &str, key: &str, cached: bool, eligible: Option<bool>, now: Instant) -> bool {
        self.baselined.retain(|_, at| now.duration_since(*at) < KEEP);
        let k = (account.to_owned(), key.to_owned());
        if !needs_baseline(cached, self.baselined.contains_key(&k), eligible) {
            return false;
        }
        self.baselined.insert(k, now);
        true
    }

    /// True (and noted) when no card showed for the pair lately.
    fn may_show(&mut self, account: &str, channel_id: &str, now: Instant) -> bool {
        self.shown.retain(|_, at| now.duration_since(*at) < SHOWN_GAP);
        // V108-C35g A card without a channel (the Rewards message may omit it) stands for
        // every channel of the account, and the other way round.
        let same = |(who, id): &(String, String)| who == account && (id == channel_id || id.is_empty() || channel_id.is_empty());
        if self.shown.keys().any(same) {
            return false;
        }
        self.shown.insert((account.to_owned(), channel_id.to_owned()), now);
        true
    }
}

/// V108-C35j Whether a train's start should read the account's emotes for a baseline: not
/// when a list is cached, one was already asked for in this train, or Twitch said the part
/// doesn't count.
fn needs_baseline(cached: bool, done: bool, eligible: Option<bool>) -> bool {
    !cached && !done && eligible != Some(false)
}

static BOOK: LazyLock<Mutex<Book>> = LazyLock::new(Mutex::default);

fn book() -> std::sync::MutexGuard<'static, Book> {
    BOOK.lock().unwrap_or_else(|p| p.into_inner())
}

/// A message of an account's rewards topic (the topic's id is the account).
pub fn on_user_event(app: &AppHandle, e: &Event) {
    let account = e.topic.channel_id.as_str();
    match api::user_event_of(&e.payload) {
        Some(UserEvent::Eligible { train, eligible }) if !train.is_empty() => {
            book().note(account, &train, eligible, Instant::now());
        }
        Some(UserEvent::Rewards { channel_id, emotes }) => {
            let list = emotes
                .into_iter()
                .map(|r| {
                    crate::emotes::twitch_emote(&TwitchEmote {
                        id: r.id,
                        name: r.token,
                        set_id: String::new(),
                        kind: EmoteKind::HypeTrain,
                        animated: false,
                        modifiers: Vec::new(),
                    })
                })
                .collect();
            show(app, account, &channel_id, list);
            // The menu and completion should have them now: the list is read again.
            let (app, account) = (app.clone(), account.to_owned());
            let channel = if channel_id.is_empty() { account.clone() } else { channel_id };
            tauri::async_runtime::spawn(async move {
                let _ = crate::emotes::refresh_diff(&app, &account, &channel).await;
            });
        }
        _ => {}
    }
}

/// V108-C35j A train started (or was first seen running) in `channel_id`: an account reading
/// it that has no emote list cached reads it once now, so the end-of-train diff has a
/// baseline. Once per account and train; cheap when called on every progress message.
pub fn on_start(app: &AppHandle, channel_id: &str, train: &str) {
    let key = if train.is_empty() { channel_id } else { train };
    for account in readers(app, channel_id) {
        let eligible = book().eligible(&account, train);
        let cached = crate::emotes::known(app, &account, channel_id).is_some();
        if !book().claim_baseline(&account, key, cached, eligible, Instant::now()) {
            continue;
        }
        let (app, channel_id) = (app.clone(), channel_id.to_owned());
        tauri::async_runtime::spawn(async move {
            if let Err(e) = crate::emotes::twitch_sets(&app, &account, &channel_id, false).await {
                eprintln!("hype train: no baseline for {account}: {e}");
            }
        });
    }
}

/// A train ended in `channel_id`: every account with a chat open there looks for new emotes,
/// unless Twitch said its part doesn't count.
pub fn on_end(app: &AppHandle, channel_id: &str, train: &str) {
    for account in readers(app, channel_id) {
        let eligible = book().eligible(&account, train);
        if eligible == Some(false) {
            continue;
        }
        // V108-C35g The baseline is taken now: a menu or points read in the 5 s before the
        // look would otherwise become it and hide the new emotes.
        let before = crate::emotes::known(app, &account, channel_id);
        let (app, channel_id) = (app.clone(), channel_id.to_owned());
        tauri::async_runtime::spawn(async move { look(app, account, channel_id, eligible == Some(true), before).await });
    }
}

/// The accounts reading a chat of the channel.
fn readers(app: &AppHandle, channel_id: &str) -> Vec<String> {
    let chat = app.state::<Chat>();
    let Some(login) = chat.channels.login_of(channel_id) else { return Vec::new() };
    let inner = chat.lock();
    let mut out: Vec<String> = inner
        .rooms
        .keys()
        .filter(|(_, l)| *l == login)
        .filter_map(|(who, _)| match who {
            Identity::Account(id) => Some(id.clone()),
            Identity::Anon => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

async fn look(app: AppHandle, account: String, channel_id: String, eligible: bool, before: Option<Baseline>) {
    tokio::time::sleep(FIRST_LOOK).await;
    let mut found = fresh(&app, &account, &channel_id, eligible, Some(before)).await;
    if found.is_empty() && eligible {
        tokio::time::sleep(SECOND_LOOK).await;
        // The first read is the baseline now.
        found = fresh(&app, &account, &channel_id, eligible, None).await;
    }
    show(&app, &account, &channel_id, found);
}

/// `before`: a baseline taken earlier (`Some`), or `None` for the cache as it is now.
async fn fresh(app: &AppHandle, account: &str, channel_id: &str, eligible: bool, before: Option<Option<Baseline>>) -> Vec<Emote> {
    let read = match before {
        Some(before) => crate::emotes::refresh_diff_from(app, account, channel_id, before).await,
        None => crate::emotes::refresh_diff(app, account, channel_id).await,
    };
    match read {
        Ok(added) => picked(&added, eligible).into_iter().map(crate::emotes::twitch_emote).collect(),
        Err(e) => {
            eprintln!("hype train: no emote list for {account}: {e}");
            Vec::new()
        }
    }
}

/// The new emotes the train gave: its own kind and the Golden Kappa; with Twitch's word that
/// the part counts, anything new (the kind is read from Twitch's type, unseen for these).
fn picked(added: &[TwitchEmote], eligible: bool) -> Vec<&TwitchEmote> {
    let hype: Vec<&TwitchEmote> = added.iter().filter(|e| e.kind == EmoteKind::HypeTrain || e.id == GOLDEN_KAPPA).collect();
    if hype.is_empty() && eligible { added.iter().collect() } else { hype }
}

fn show(app: &AppHandle, account: &str, channel_id: &str, emotes: Vec<Emote>) {
    if emotes.is_empty() || !book().may_show(account, channel_id, Instant::now()) {
        return;
    }
    let card = HypeUnlock { account: account.to_owned(), channel_id: channel_id.to_owned(), emotes };
    let _ = app.emit(EVENT, card);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emote(id: &str, kind: EmoteKind) -> TwitchEmote {
        TwitchEmote { id: id.into(), name: id.into(), set_id: String::new(), kind, animated: false, modifiers: Vec::new() }
    }

    #[test]
    fn what_twitch_said_is_kept_per_account_and_train() {
        let mut b = Book::default();
        let t = Instant::now();
        b.note("me", "t1", false, t);
        assert_eq!(b.eligible("me", "t1"), Some(false));
        assert_eq!(b.eligible("you", "t1"), None);
        assert_eq!(b.eligible("me", ""), None);
        b.note("me", "t2", true, t + KEEP);
        assert_eq!(b.eligible("me", "t1"), None);
        assert_eq!(b.eligible("me", "t2"), Some(true));
    }

    #[test]
    fn v108_c35j_a_baseline_is_read_once_when_nothing_is_cached() {
        assert!(needs_baseline(false, false, None));
        assert!(needs_baseline(false, false, Some(true)));
        assert!(!needs_baseline(true, false, None));
        assert!(!needs_baseline(false, true, None));
        assert!(!needs_baseline(false, false, Some(false)));
        let mut b = Book::default();
        let t = Instant::now();
        assert!(b.claim_baseline("me", "t1", false, None, t));
        assert!(!b.claim_baseline("me", "t1", false, None, t));
        assert!(b.claim_baseline("you", "t1", false, None, t));
        assert!(!b.claim_baseline("me", "t2", true, None, t));
        assert!(b.claim_baseline("me", "t2", false, None, t));
    }

    #[test]
    fn one_card_per_channel_a_while() {
        let mut b = Book::default();
        let t = Instant::now();
        assert!(b.may_show("me", "1", t));
        assert!(!b.may_show("me", "1", t + Duration::from_secs(5)));
        assert!(b.may_show("me", "2", t));
        assert!(b.may_show("me", "1", t + SHOWN_GAP));
        // The empty channel key and a real one are the same train.
        assert!(!b.may_show("me", "", t + SHOWN_GAP + Duration::from_secs(5)));
        assert!(b.may_show("you", "", t));
        assert!(!b.may_show("you", "3", t + Duration::from_secs(5)));
        assert!(!b.may_show("me", "1", t + SHOWN_GAP + Duration::from_secs(5)));
    }

    #[test]
    fn hype_emotes_first_anything_new_only_when_it_counts() {
        let added = [emote("a", EmoteKind::Global), emote("b", EmoteKind::HypeTrain)];
        assert_eq!(picked(&added, false).len(), 1);
        assert_eq!(picked(&added, true)[0].id, "b");
        let plain = [emote("a", EmoteKind::Subscription)];
        assert!(picked(&plain, false).is_empty());
        assert_eq!(picked(&plain, true).len(), 1);
    }

    #[test]
    fn the_golden_kappa_counts_without_twitchs_word() {
        // Twitch lists it as type UNKNOWN, which reads as Other.
        let added = [emote("80393", EmoteKind::Other), emote("a", EmoteKind::Global)];
        let got = picked(&added, false);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, GOLDEN_KAPPA);
    }
}
