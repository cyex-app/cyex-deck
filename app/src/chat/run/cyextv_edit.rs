//! V112-17/18 `/add`, `/remove`, `/rename`, `/steal` (`/yoink`) and `/whoadded` act on cyex.tv
//! (7TV's are `/7add` …, `seventv_edit.rs`). The channel's active set is the target; it is edited
//! if the account may (`channelEditableSets`), through the cyex.tv session's Bearer
//! (`crate::cyextv::with_bearer_typed`). Results are local lines; nothing goes to the channel. A
//! change a chat holds is shown at once by reading its set again (C10 line), else a line here.
//! `/whoadded` only reads the public set: no sign-in.

use std::sync::Arc;

use cyex_chat::model::{NoticeKind, Provider};
use cyex_emotes::cyextv::api;
use cyex_emotes::cyextv::gql::{EditableSets, GqlError, Platform};
use cyex_emotes::{Emote, Scope};
use cyex_twitch::pace::Lane;
use cyex_auth::CyextvError;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::seventv_edit::{
    MAX_REMOVE, Pick, SPACES, TOO_MANY, add_fail, joined, pick_line, split_channel,
};
use super::{Place, UiAction, local};
use crate::auth::Auth;
use crate::cyextv::{CallFail, gql, mocked, with_bearer_typed};
use crate::emotes::edit::{self, SourceFail};

/// The arm in `builtin::run` and `OWN` must list these (checked in builtin's tests).
#[cfg(test)]
pub(super) const NAMES: [&str; 6] = ["add", "remove", "rename", "steal", "yoink", "whoadded"];

const USAGE_ADD: &str = "Say which emote: /add <cyex.tv link or id> [name] [#channel].";
const USAGE_REMOVE: &str = "Say which emote: /remove <name> [more names] [#channel].";
const USAGE_RENAME: &str = "Say which emote: /rename <name> [new name] [#channel].";
const USAGE_STEAL: &str = "Say which emote and where from: /steal <name> [new name] #channel.";
const USAGE_WHO: &str = "Say which emote: /whoadded <name> [#channel].";
const STEAL_HERE: &str = "That’s this chat’s channel. Say where from: /steal <name> #channel.";
const NOT_LINK: &str = "That isn’t a cyex.tv emote link or id.";
const NO_EMOTE: &str = "No cyex.tv emote with that id.";
const SIGN_IN: &str = "Sign in to cyex.tv in Accounts.";
const SIGN_IN_AGAIN: &str = "Your cyex.tv sign-in ran out. Sign in again in Accounts.";
const ALREADY: &str = "Already in {channel}’s cyex.tv emotes.";
const NO_SUCH: &str = "{channel} has no cyex.tv emote called {name}.";
const SAME: &str = "It’s called {from} already.";
const DOWN: &str = "Couldn’t reach cyex.tv. Nothing changed.";
const MOCK_ONLY: &str = "cyex.tv edits only go to the test mock here.";

#[derive(Debug, PartialEq, Eq)]
enum Parsed {
    Add {
        id: String,
        alias: Option<String>,
    },
    /// One or more, once each, in the order typed: names, or ids and links.
    Remove {
        words: Vec<String>,
    },
    /// `to: None` goes back to the emote's own name.
    Rename {
        from: String,
        to: Option<String>,
    },
    /// A name in the `#channel`'s set, or (alone, no channel) an id or link.
    Steal {
        what: String,
        alias: Option<String>,
    },
    WhoAdded {
        what: String,
    },
}

/// An emote id typed bare, or in a `cyex.tv/emotes/<id>` link. Ids are one run of 20 to 32
/// letters and digits (cyex.tv's are 26 characters).
fn emote_id(word: &str) -> Option<String> {
    const MARK: &str = "cyex.tv/emotes/";
    let lower = word.to_ascii_lowercase();
    let id = match lower.find(MARK) {
        Some(at) => {
            let head = &lower[..at];
            let head = head
                .strip_prefix("https://")
                .or_else(|| head.strip_prefix("http://"))
                .unwrap_or(head);
            let head = head.strip_prefix("www.").unwrap_or(head);
            if !head.is_empty() {
                return None;
            }
            let tail = &word[at + MARK.len()..];
            &tail[..tail.find(['/', '?', '#']).unwrap_or(tail.len())]
        }
        None => word,
    };
    ((20..=32).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric())).then(|| id.to_owned())
}

/// `command` is the verb (`add`, `yoink` …). Also gives the `#channel`.
fn parse(command: &str, args: &str) -> Result<(Parsed, Option<String>), String> {
    let (words, channel) = split_channel(args)?;
    let (usage, max) = match command {
        "add" => (USAGE_ADD, 2),
        "remove" => (USAGE_REMOVE, MAX_REMOVE),
        "steal" | "yoink" => (USAGE_STEAL, 2),
        "whoadded" => (USAGE_WHO, 1),
        _ => (USAGE_RENAME, 2),
    };
    if words.is_empty() {
        return Err(usage.into());
    }
    if words.len() > max {
        return Err(if command == "remove" { TOO_MANY } else { SPACES }.into());
    }
    let second = words.get(1).map(|w| (*w).to_owned());
    let first = words[0].to_owned();
    let cmd = match command {
        "add" => Parsed::Add {
            id: emote_id(words[0]).ok_or(NOT_LINK)?,
            alias: second,
        },
        "remove" => {
            let mut seen: Vec<String> = Vec::new();
            for w in words {
                if !seen.iter().any(|n| n == w) {
                    seen.push(w.to_owned());
                }
            }
            Parsed::Remove { words: seen }
        }
        "steal" | "yoink" => {
            // A link or id needs no channel: it is an add.
            if channel.is_none() && emote_id(&first).is_none() {
                return Err(usage.into());
            }
            Parsed::Steal {
                what: first,
                alias: second,
            }
        }
        "whoadded" => Parsed::WhoAdded { what: first },
        _ => Parsed::Rename { from: first, to: second },
    };
    Ok((cmd, channel))
}

/// The set emote a word names: its name (exact, else the only one up to case), else its id (a
/// link too).
fn find<'a>(emotes: &'a [Arc<Emote>], word: &str) -> Result<&'a Arc<Emote>, Pick> {
    if let Some(e) = emotes.iter().find(|e| e.name == word) {
        return Ok(e);
    }
    let like: Vec<&Arc<Emote>> = emotes.iter().filter(|e| e.name.eq_ignore_ascii_case(word)).collect();
    match like.as_slice() {
        [] => {}
        [one] => return Ok(one),
        many => return Err(Pick::Ambiguous(many.iter().map(|e| e.name.clone()).collect())),
    }
    emote_id(word)
        .and_then(|id| emotes.iter().find(|e| api::emote_id(&e.id) == id))
        .ok_or(Pick::Missing)
}

/// The channel's cyex.tv set as the command edits it.
struct Target {
    set_id: String,
    channel: String,
    capacity: u32,
    used: u32,
    emotes: Vec<Arc<Emote>>,
}

impl Target {
    fn taken(&self, name: &str) -> bool {
        self.emotes.iter().any(|e| e.name == name)
    }

    fn has(&self, emote_id: &str) -> bool {
        self.emotes.iter().any(|e| api::emote_id(&e.id) == emote_id)
    }
}

/// The checks before an add, in the menu's order.
fn add_checks(t: &Target, id: &str, alias: &str) -> Result<(), String> {
    let channel = t.channel.as_str();
    if t.has(id) {
        return Err(ALREADY.replace("{channel}", channel));
    }
    if t.capacity > 0 && t.used >= t.capacity {
        return Err(format!("{channel}’s cyex.tv emotes are full."));
    }
    if !name_ok(alias) {
        return Err("cyex.tv doesn’t allow that name.".into());
    }
    if t.taken(alias) {
        return Err(format!("{channel} already has an emote named {alias}."));
    }
    Ok(())
}

/// 1-100 characters, no spaces or control characters.
fn name_ok(name: &str) -> bool {
    (1..=100).contains(&name.chars().count()) && !name.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// The line for a failed call.
fn fail_line(e: &CallFail, channel: &str) -> String {
    match e {
        CallFail::Auth(CyextvError::SignedOut) | CallFail::Gql(GqlError::SignedOut) => SIGN_IN_AGAIN.into(),
        CallFail::Auth(CyextvError::Down) | CallFail::Gql(GqlError::Down) => DOWN.into(),
        CallFail::Auth(CyextvError::RateLimited) | CallFail::Gql(GqlError::RateLimited) => {
            "cyex.tv needs a short break. Try again in a minute.".into()
        }
        CallFail::Gql(GqlError::Forbidden) => cant(channel),
        CallFail::Gql(GqlError::NotFound) => "That emote isn’t on cyex.tv anymore.".into(),
        CallFail::Gql(GqlError::Scope) => "cyex deck can’t change that. Use cyex.tv for it.".into(),
        other => other.to_string(),
    }
}

fn cant(channel: &str) -> String {
    format!("You can’t edit {channel}’s cyex.tv emotes.")
}

/// A failure after which the other names of a `/remove` aren't tried.
fn stops(e: &CallFail) -> bool {
    matches!(
        e,
        CallFail::Auth(_)
            | CallFail::Gql(GqlError::SignedOut | GqlError::Forbidden | GqlError::RateLimited | GqlError::Scope)
    )
}

fn source_line(f: SourceFail, channel: &str) -> String {
    match f {
        SourceFail::NoChannel => format!("No Twitch channel called {channel}."),
        SourceFail::NoSet => format!("{channel} has no cyex.tv emotes."),
        SourceFail::Down => "Couldn’t reach cyex.tv. Try again in a minute.".into(),
    }
}

/// An open chat's cyex.tv channel set, cloned. `None` also while cyex.tv emotes are off there.
fn loaded_set(app: &AppHandle, login: &str) -> Option<Vec<Arc<Emote>>> {
    edit::loaded_channel_set(app, login, Provider::Cyextv)
}

/// Tab for the commands: an open chat's cyex.tv emote names, sorted. No network.
pub fn loaded_names(app: &AppHandle, login: &str) -> Vec<String> {
    let mut names: Vec<String> = loaded_set(app, login)
        .map(|s| s.iter().map(|e| e.name.clone()).collect())
        .unwrap_or_default();
    names.sort_unstable();
    names.dedup();
    names
}

/// A channel's active cyex.tv set: an open chat's, else read from cyex.tv (public, no sign-in).
async fn channel_set(app: &AppHandle, login: &str) -> Result<Vec<Arc<Emote>>, SourceFail> {
    if let Some(found) = loaded_set(app, login) {
        return Ok(found);
    }
    let id = edit::twitch_id(app, login)
        .await
        .map_err(|()| SourceFail::Down)?
        .ok_or(SourceFail::NoChannel)?;
    let http = app.state::<Auth>().http.clone();
    let set = api::channel(&http, &id, Lane::Front).await.map_err(|e| {
        eprintln!("cyex.tv edit: emotes of {login}: {e}");
        SourceFail::Down
    })?;
    Ok(set.ok_or(SourceFail::NoSet)?.emotes)
}

/// The set as it is now (a fresh read, so a check never trusts a stale copy).
async fn read_set(app: &AppHandle, set_id: &str) -> Result<Vec<Arc<Emote>>, String> {
    let http = app.state::<Auth>().http.clone();
    let v = api::set_raw(&http, set_id, Lane::Front).await.map_err(|e| {
        eprintln!("cyex.tv edit: set {set_id}: {e}");
        DOWN.to_owned()
    })?;
    api::parse_set(&v, Scope::Channel).map(|s| s.emotes).map_err(|e| {
        eprintln!("cyex.tv edit: set {set_id}: {e}");
        DOWN.to_owned()
    })
}

/// What the account may edit in `login`'s cyex.tv set, and the set as it is.
async fn target(app: &AppHandle, account: &str, login: &str) -> Result<Target, String> {
    let id = edit::twitch_id(app, login)
        .await
        .map_err(|()| DOWN.to_owned())?
        .ok_or_else(|| format!("No Twitch channel called {login}."))?;
    let api = gql(app);
    let got = with_bearer_typed(app, account, |t| {
        let api = api.clone();
        let id = id.clone();
        async move { api.channel_editable_sets(&t, Platform::Twitch, &id).await }
    })
    .await
    .map_err(|e| fail_line(&e, login))?;
    let (set_id, capacity, used) = editable_active(got.as_ref()).ok_or_else(|| cant(login))?;
    let emotes = read_set(app, &set_id).await?;
    Ok(Target {
        set_id,
        channel: login.to_owned(),
        capacity,
        used,
        emotes,
    })
}

/// The active set's id, capacity and slots used, when the account may edit it.
fn editable_active(sets: Option<&EditableSets>) -> Option<(String, u32, u32)> {
    let sets = sets?;
    let id = sets.active_set_id.as_deref().filter(|_| sets.can_edit_active_set)?;
    let (capacity, used) = sets
        .sets
        .iter()
        .find(|s| s.set.id == id)
        .map_or((0, 0), |s| (s.set.capacity, s.set.slots_used));
    Some((id.to_owned(), capacity, used))
}

/// A short line about the change, when no chat shows it by itself.
fn told(app: &AppHandle, place: &Place, t: &Target, line: &str) {
    if !crate::emotes::cyextv_set_changed(app, &t.set_id) {
        local(app, &place.key, NoticeKind::Info, line);
    }
}

pub(super) async fn run(
    app: &AppHandle,
    place: &Place,
    name: &str,
    args: &str,
) -> Result<Option<UiAction>, String> {
    let (cmd, other) = parse(name, args)?;
    let here = place.channel();
    let other = other.filter(|c| !c.eq_ignore_ascii_case(here));
    // Read-only: no account, no cyex.tv sign-in, fine in automations.
    if let Parsed::WhoAdded { what } = &cmd {
        return who_added(app, place, what, other.as_deref()).await;
    }
    let account = place
        .account()
        .ok_or_else(|| format!("Sign in to use /{name}."))?;
    if place.automation {
        return Err(format!("/{name} doesn’t run in automations."));
    }
    let signed = app.state::<Auth>().cyextv.who(account).is_some();
    if !signed {
        return Err(if app.state::<Auth>().cyextv.ended(account) { SIGN_IN_AGAIN } else { SIGN_IN }.into());
    }
    // Agents and test runs never edit the real cyex.tv.
    if crate::paths::webview(app).is_some() && !mocked(app) {
        return Err(MOCK_ONLY.into());
    }
    // `/steal` names a source; the change goes into this chat's channel.
    let steal_named = matches!(&cmd, Parsed::Steal { what, .. } if emote_id(what).is_none());
    if steal_named && other.is_none() {
        return Err(STEAL_HERE.into());
    }
    let into = match (&cmd, &other) {
        (Parsed::Steal { .. }, _) | (_, None) => here,
        (_, Some(o)) => o.as_str(),
    };
    let t = target(app, account, into).await?;
    let channel = t.channel.as_str();
    match cmd {
        Parsed::Add { id, alias } => add(app, place, account, &t, &id, alias).await,
        Parsed::Steal { what, alias } => {
            if let Some(id) = emote_id(&what).filter(|_| !steal_named) {
                return add(app, place, account, &t, &id, alias).await;
            }
            let src = other.as_deref().unwrap_or_default();
            let emotes = channel_set(app, src).await.map_err(|f| source_line(f, src))?;
            let e = find(&emotes, &what).map_err(|p| pick_line(p, src, &what))?;
            let alias = alias.unwrap_or_else(|| e.name.clone());
            let id = api::emote_id(&e.id).to_owned();
            add_checks(&t, &id, &alias)?;
            put(app, place, account, &t, &id, &alias).await
        }
        Parsed::Remove { words } => remove(app, place, account, &t, words).await,
        Parsed::Rename { from, to } => {
            let e = find(&t.emotes, &from).map_err(|p| pick_line(p, channel, &from))?;
            let from = e.name.clone();
            let to = match to {
                Some(to) => to,
                None => e.original.clone().ok_or_else(|| SAME.replace("{from}", &from))?,
            };
            if to == from {
                return Err(SAME.replace("{from}", &from));
            }
            if !name_ok(&to) {
                return Err("cyex.tv doesn’t allow that name.".into());
            }
            if t.taken(&to) {
                return Err(format!("{channel} already has an emote named {to}."));
            }
            let id = api::emote_id(&e.id).to_owned();
            let api = gql(app);
            with_bearer_typed(app, account, |tok| {
                let (api, set, id, to) = (api.clone(), t.set_id.clone(), id.clone(), to.clone());
                async move { api.update_set_emote(&tok, &set, &id, Some(&to), None).await }
            })
            .await
            .map_err(|e| fail_line(&e, channel))?;
            told(app, place, &t, &format!("Renamed {from} to {to} in {channel}’s cyex.tv emotes."));
            Ok(None)
        }
        Parsed::WhoAdded { .. } => Err(USAGE_WHO.into()),
    }
}

/// `/add id [name]`: its own name unless one is given.
async fn add(
    app: &AppHandle,
    place: &Place,
    account: &str,
    t: &Target,
    id: &str,
    alias: Option<String>,
) -> Result<Option<UiAction>, String> {
    let http = app.state::<Auth>().http.clone();
    let alias = match alias {
        Some(a) => a,
        None => {
            api::emote(&http, id, Lane::Front)
                .await
                .map_err(|e| {
                    eprintln!("cyex.tv edit: emote {id}: {e}");
                    DOWN.to_owned()
                })?
                .ok_or(NO_EMOTE)?
                .name
        }
    };
    add_checks(t, id, &alias)?;
    put(app, place, account, t, id, &alias).await
}

/// The add itself, once checked.
async fn put(
    app: &AppHandle,
    place: &Place,
    account: &str,
    t: &Target,
    id: &str,
    alias: &str,
) -> Result<Option<UiAction>, String> {
    send_add(app, account, t, id, alias).await?;
    told(app, place, t, &format!("Added {alias} to {}’s cyex.tv emotes.", t.channel));
    Ok(None)
}

async fn send_add(app: &AppHandle, account: &str, t: &Target, id: &str, alias: &str) -> Result<(), String> {
    let api = gql(app);
    with_bearer_typed(app, account, |tok| {
        let (api, set, id, alias) = (api.clone(), t.set_id.clone(), id.to_owned(), alias.to_owned());
        async move { api.add_set_emote(&tok, &set, &id, Some(&alias), None).await }
    })
    .await
    .map(drop)
    .map_err(|e| fail_line(&e, &t.channel))
}

/// V112-17 What the emote menu offers for a cyex.tv emote in this chat's channel.
#[derive(Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CyextvMenuOptions {
    /// The account is signed in to cyex.tv and may edit this channel's active set.
    pub can_edit: bool,
    /// Why the entry is greyed out (already there, full).
    pub note: Option<String>,
}

/// Whether "Add to cyex.tv" is offered: signed in, the channel's active set editable. A session
/// or network that fails just offers nothing.
pub async fn menu_options(app: &AppHandle, account: &str, login: &str, deck_id: &str) -> CyextvMenuOptions {
    let none = CyextvMenuOptions { can_edit: false, note: None };
    if app.state::<Auth>().cyextv.who(account).is_none() {
        return none;
    }
    let Ok(t) = target(app, account, login).await else {
        return none;
    };
    let note = add_checks(&t, api::emote_id(deck_id), "x").err().filter(|n| !n.contains("allow that name"));
    CyextvMenuOptions { can_edit: true, note }
}

/// The menu's add: the emote under its own name. Gives the line to show.
pub async fn menu_add(app: &AppHandle, account: &str, login: &str, deck_id: &str) -> Result<String, String> {
    if crate::paths::webview(app).is_some() && !mocked(app) {
        return Err(MOCK_ONLY.into());
    }
    if app.state::<Auth>().cyextv.who(account).is_none() {
        return Err(SIGN_IN.into());
    }
    let id = api::emote_id(deck_id).to_owned();
    let http = app.state::<Auth>().http.clone();
    let alias = api::emote(&http, &id, Lane::Front)
        .await
        .map_err(|e| {
            eprintln!("cyex.tv edit: emote {id}: {e}");
            DOWN.to_owned()
        })?
        .ok_or(NO_EMOTE)?
        .name;
    let t = target(app, account, login).await?;
    add_checks(&t, &id, &alias)?;
    send_add(app, account, &t, &id, &alias).await?;
    crate::emotes::cyextv_set_changed(app, &t.set_id);
    Ok(format!("Added {alias} to {}’s cyex.tv emotes.", t.channel))
}

/// `/remove a b c`: one call each, one line for all.
async fn remove(
    app: &AppHandle,
    place: &Place,
    account: &str,
    t: &Target,
    words: Vec<String>,
) -> Result<Option<UiAction>, String> {
    let channel = t.channel.as_str();
    let mut known: Vec<(String, String)> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for w in words {
        match find(&t.emotes, &w) {
            Ok(e) => {
                let pair = (e.name.clone(), api::emote_id(&e.id).to_owned());
                if !known.contains(&pair) {
                    known.push(pair);
                }
            }
            Err(p) => {
                match p {
                    Pick::Missing => missing.push(w),
                    Pick::Ambiguous(_) => return Err(pick_line(p, channel, &w)),
                }
            }
        }
    }
    let missing_line = match missing.as_slice() {
        [] => None,
        [one] => Some(NO_SUCH.replace("{channel}", channel).replace("{name}", one)),
        many => Some(format!("{channel} has no cyex.tv emotes called {}.", joined(many, "or"))),
    };
    if known.is_empty() {
        return Err(missing_line.unwrap_or_else(|| USAGE_REMOVE.into()));
    }
    let api = gql(app);
    let mut removed: Vec<String> = Vec::new();
    let mut failed: Vec<(String, Vec<String>)> = Vec::new();
    let mut stopped: Option<CallFail> = None;
    for (name, id) in known {
        if let Some(e) = &stopped {
            add_fail(&mut failed, fail_line(e, channel), name);
            continue;
        }
        let sent = with_bearer_typed(app, account, |tok| {
            let (api, set, id) = (api.clone(), t.set_id.clone(), id.clone());
            async move { api.remove_set_emote(&tok, &set, &id).await }
        })
        .await;
        match sent {
            Ok(_) => removed.push(name),
            Err(e) => {
                add_fail(&mut failed, fail_line(&e, channel), name);
                if stops(&e) {
                    stopped = Some(e);
                }
            }
        }
    }
    let done_line = (!removed.is_empty()).then(|| format!("Removed {}.", joined(&removed, "and")));
    let fail_lines = failed
        .into_iter()
        .map(|(reason, names)| format!("Not removed: {}. {reason}", joined(&names, "and")));
    let rest: Vec<String> = missing_line.into_iter().chain(fail_lines).collect();
    let rest = rest.join(" ");
    match done_line {
        None => Err(rest),
        Some(line) => {
            if rest.is_empty() {
                told(app, place, t, &line);
            } else {
                crate::emotes::cyextv_set_changed(app, &t.set_id);
                local(app, &place.key, NoticeKind::Error, &format!("{line} {rest}"));
            }
            Ok(None)
        }
    }
}

/// `/whoadded name [#channel]`: who added it and when, from the set's data.
async fn who_added(
    app: &AppHandle,
    place: &Place,
    what: &str,
    other: Option<&str>,
) -> Result<Option<UiAction>, String> {
    let login = other.unwrap_or(place.channel());
    let emotes = channel_set(app, login).await.map_err(|f| source_line(f, login))?;
    let e = find(&emotes, what).map_err(|p| pick_line(p, login, what))?;
    let mine = place
        .account()
        .and_then(|a| app.state::<Auth>().cyextv.who(a))
        .map(|w| (w.id, w.name));
    let who = e.added_by.as_deref().and_then(|id| {
        let owner = e.owner_id.as_deref().zip(e.owner.as_deref());
        known_name(id, owner, mine.as_ref().map(|(i, n)| (i.as_str(), n.as_str())))
    });
    let when = e
        .added_at
        .as_deref()
        .map(crate::bell_dates::local_dates)
        .and_then(|d| d.split_whitespace().next().map(str::to_owned));
    let text = who_line(&e.name, other, who.as_deref(), when.as_deref());
    local(app, &place.key, NoticeKind::Info, &text);
    Ok(None)
}

/// A name for an account id without asking: the emote's owner, or you.
fn known_name(id: &str, owner: Option<(&str, &str)>, me: Option<(&str, &str)>) -> Option<String> {
    [owner, me]
        .into_iter()
        .flatten()
        .find(|(i, _)| *i == id)
        .map(|(_, n)| n.to_owned())
}

/// The `/whoadded` line. `channel`: named only when it isn't this chat's.
fn who_line(name: &str, channel: Option<&str>, who: Option<&str>, when: Option<&str>) -> String {
    let to = channel.map(|c| format!(" to {c}")).unwrap_or_default();
    let by = who.unwrap_or("someone");
    match when {
        Some(d) => format!("{name} was added{to} by {by} on {d}."),
        None => format!("{name} was added{to} by {by}."),
    }
}

#[cfg(test)]
mod tests {
    use cyex_emotes::cyextv::gql::{Access, EditableSet, SetSummary};

    use super::super::seventv_edit::ONE_CHANNEL;
    use super::*;

    const ID: &str = "01HELLO0000000000000000000";

    fn emote(id: &str, name: &str, original: Option<&str>) -> Arc<Emote> {
        let mut e = Emote::new(Provider::Cyextv, format!("{id}.key"), name);
        e.original = original.map(str::to_owned);
        Arc::new(e)
    }

    fn here(p: Parsed) -> Result<(Parsed, Option<String>), String> {
        Ok((p, None))
    }

    #[test]
    fn ids_and_links() {
        assert_eq!(emote_id(ID).as_deref(), Some(ID));
        assert_eq!(emote_id(&format!("https://cyex.tv/emotes/{ID}")).as_deref(), Some(ID));
        assert_eq!(emote_id(&format!("http://www.cyex.tv/emotes/{ID}/?x=1")).as_deref(), Some(ID));
        assert_eq!(emote_id(&format!("cyex.tv/emotes/{ID}#a")).as_deref(), Some(ID));
        assert_eq!(emote_id(&format!("https://evil.cyex.tv/emotes/{ID}")), None);
        assert_eq!(emote_id(&format!("https://7tv.app/emotes/{ID}")), None);
        assert_eq!(emote_id("peepoHappy"), None);
        assert_eq!(emote_id("short"), None);
        assert_eq!(emote_id("0123456789012345678-"), None);
    }

    #[test]
    fn parse_table() {
        assert_eq!(parse("add", ""), Err(USAGE_ADD.into()));
        assert_eq!(parse("add", "nope"), Err(NOT_LINK.into()));
        assert_eq!(parse("add", ID), here(Parsed::Add { id: ID.into(), alias: None }));
        assert_eq!(
            parse("add", &format!(" {ID} happy #Forsen")),
            Ok((Parsed::Add { id: ID.into(), alias: Some("happy".into()) }, Some("forsen".into())))
        );
        assert_eq!(parse("add", &format!("{ID} a b")), Err(SPACES.into()));
        assert_eq!(parse("remove", ""), Err(USAGE_REMOVE.into()));
        assert_eq!(
            parse("remove", "a b a"),
            here(Parsed::Remove { words: vec!["a".into(), "b".into()] })
        );
        let many: Vec<String> = (0..21).map(|i| format!("n{i}")).collect();
        assert_eq!(parse("remove", &many.join(" ")), Err(TOO_MANY.into()));
        assert_eq!(parse("rename", "a"), here(Parsed::Rename { from: "a".into(), to: None }));
        assert_eq!(
            parse("rename", "a b #x"),
            Ok((Parsed::Rename { from: "a".into(), to: Some("b".into()) }, Some("x".into())))
        );
        assert_eq!(parse("rename", "a b c"), Err(SPACES.into()));
        assert_eq!(parse("whoadded", "a b"), Err(SPACES.into()));
        assert_eq!(parse("whoadded", ""), Err(USAGE_WHO.into()));
        assert_eq!(parse("whoadded", "a #x"), Ok((Parsed::WhoAdded { what: "a".into() }, Some("x".into()))));
        assert_eq!(parse("add", "a #b #c"), Err(ONE_CHANNEL.into()));
        assert_eq!(parse("add", "a #b!"), Err("#b! isn’t a channel name.".into()));
    }

    #[test]
    fn steal_needs_a_channel_unless_it_is_an_id() {
        assert_eq!(parse("steal", "KEKW"), Err(USAGE_STEAL.into()));
        assert_eq!(parse("yoink", "KEKW"), Err(USAGE_STEAL.into()));
        assert_eq!(
            parse("steal", "KEKW happy #forsen"),
            Ok((Parsed::Steal { what: "KEKW".into(), alias: Some("happy".into()) }, Some("forsen".into())))
        );
        assert_eq!(parse("yoink", ID), here(Parsed::Steal { what: ID.into(), alias: None }));
    }

    #[test]
    fn find_takes_name_then_case_then_id() {
        let set = vec![
            emote("01AAAAAAAAAAAAAAAAAAAAAAAA", "KEKW", None),
            emote("01BBBBBBBBBBBBBBBBBBBBBBBB", "Happy", Some("peepoHappy")),
            emote("01CCCCCCCCCCCCCCCCCCCCCCCC", "ab", None),
            emote("01DDDDDDDDDDDDDDDDDDDDDDDD", "AB", None),
        ];
        assert_eq!(find(&set, "KEKW").unwrap().name, "KEKW");
        assert_eq!(find(&set, "kekw").unwrap().name, "KEKW");
        assert_eq!(find(&set, "ab").unwrap().name, "ab");
        assert!(matches!(find(&set, "aB"), Err(Pick::Ambiguous(n)) if n == ["ab", "AB"]));
        assert_eq!(find(&set, "01BBBBBBBBBBBBBBBBBBBBBBBB").unwrap().name, "Happy");
        assert_eq!(
            find(&set, "https://cyex.tv/emotes/01BBBBBBBBBBBBBBBBBBBBBBBB").unwrap().name,
            "Happy"
        );
        assert!(matches!(find(&set, "nope"), Err(Pick::Missing)));
    }

    fn target(used: u32, capacity: u32) -> Target {
        Target {
            set_id: "S".into(),
            channel: "forsen".into(),
            capacity,
            used,
            emotes: vec![emote("01AAAAAAAAAAAAAAAAAAAAAAAA", "KEKW", None)],
        }
    }

    #[test]
    fn add_checks_in_order() {
        let t = target(1, 10);
        assert_eq!(add_checks(&t, "01AAAAAAAAAAAAAAAAAAAAAAAA", "x"), Err("Already in forsen’s cyex.tv emotes.".into()));
        assert_eq!(add_checks(&t, "01ZZ", "KEKW"), Err("forsen already has an emote named KEKW.".into()));
        assert_eq!(add_checks(&t, "01ZZ", "a b"), Err("cyex.tv doesn’t allow that name.".into()));
        assert_eq!(add_checks(&target(10, 10), "01ZZ", "new"), Err("forsen’s cyex.tv emotes are full.".into()));
        assert_eq!(add_checks(&t, "01ZZ", "new"), Ok(()));
    }

    fn editable(active: Option<&str>, can: bool) -> EditableSets {
        EditableSets {
            active_set_id: active.map(str::to_owned),
            login: "forsen".into(),
            role: Some("OWNER".into()),
            can_edit_active_set: can,
            sets: vec![EditableSet {
                set: SetSummary { id: "S".into(), name: "Main".into(), capacity: 100, slots_used: 7 },
                active_here: true,
                access: Access::Owner,
            }],
        }
    }

    #[test]
    fn the_active_set_is_the_target_only_when_editable() {
        assert_eq!(editable_active(Some(&editable(Some("S"), true))), Some(("S".into(), 100, 7)));
        assert_eq!(editable_active(Some(&editable(Some("S"), false))), None);
        assert_eq!(editable_active(Some(&editable(None, true))), None);
        assert_eq!(editable_active(None), None);
        // A set the list doesn't show still takes the edit; the server decides.
        assert_eq!(editable_active(Some(&editable(Some("T"), true))), Some(("T".into(), 0, 0)));
    }

    #[test]
    fn lines_name_cyex_tv() {
        assert_eq!(fail_line(&CallFail::Gql(GqlError::Forbidden), "forsen"), "You can’t edit forsen’s cyex.tv emotes.");
        assert_eq!(fail_line(&CallFail::Auth(CyextvError::SignedOut), "x"), SIGN_IN_AGAIN);
        assert_eq!(fail_line(&CallFail::Gql(GqlError::Down), "x"), DOWN);
        assert_eq!(fail_line(&CallFail::Gql(GqlError::Said("Name taken.".into())), "x"), "Name taken.");
        assert!(stops(&CallFail::Gql(GqlError::Forbidden)) && !stops(&CallFail::Gql(GqlError::NotFound)));
        assert_eq!(who_line("KEKW", None, None, Some("2026/11/14")), "KEKW was added by someone on 2026/11/14.");
        assert_eq!(who_line("KEKW", Some("forsen"), Some("Ann"), None), "KEKW was added to forsen by Ann.");
        assert_eq!(known_name("o", Some(("o", "Owner")), Some(("m", "Me"))).as_deref(), Some("Owner"));
        assert_eq!(known_name("m", Some(("o", "Owner")), Some(("m", "Me"))).as_deref(), Some("Me"));
        assert_eq!(known_name("x", None, None), None);
    }
}
