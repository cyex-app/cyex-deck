//! V106-31/32 7TV set editing: per account, the channels it can edit (kept 5 min, updated by
//! our edits and 7TV's events), one edit at a time per set, our edits shown in loaded chats at
//! once with their C10 line (C1). Nothing calls 7TV before the account's 7TV consent and a
//! stored session; a test run refuses edits unless the client points at the loopback mock.

mod echo;
mod here;
mod ops;
mod options;
mod reach;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use echo::Echoes;
pub use here::{emote_facts, here_names, here_target};
pub use ops::{EditDone, EditFail, EditOp, EditOptions, NameCheck, Reachability};
pub use reach::Target;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cyex_chat::model::Provider;
use cyex_emotes::seventv::account::{Client, EditError, Error};
use cyex_emotes::seventv::events::SetChange;
use cyex_emotes::{Emote, Scope};
use tauri::{AppHandle, Manager};

use crate::auth::{self, Auth};
use crate::cosmetics;
use options::{Facts, Viewed};
use reach::Reach;

/// How long a read reach is kept.
const KEEP: Duration = Duration::from_secs(5 * 60);

type Gate = Arc<tokio::sync::Mutex<()>>;

#[derive(Default)]
pub struct Edits {
    /// By account (Twitch id): when read, and what it can edit. `None` = no 7TV account.
    /// Edited in place under this lock; never held across an await.
    reach: Mutex<HashMap<String, (Instant, Option<Reach>)>>,
    /// One read per account at a time.
    warming: Mutex<HashMap<String, Gate>>,
    /// One edit per set at a time.
    gates: Mutex<HashMap<String, Gate>>,
    /// Last read failed: `Down` until the next warm.
    down: Mutex<HashSet<String>>,
    /// C1 Our edits that 7TV's event must not print again.
    echoes: Mutex<echo::Echoes>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn gate(map: &Mutex<HashMap<String, Gate>>, key: &str) -> Gate {
    lock(map).entry(key.to_owned()).or_default().clone()
}

/// After `cosmetics::init`.
pub fn init(app: &AppHandle) {
    app.manage(Edits::default());
}

fn state(app: &AppHandle) -> &Edits {
    app.state::<Edits>().inner()
}

/// No 7TV call without consent and a live session.
fn gate_state(app: &AppHandle, account: &str) -> Option<Reachability> {
    let tokens = &app.state::<Auth>().seventv;
    standing(
        cosmetics::has_consent(app, account),
        tokens.get(account).as_ref(),
        tokens.rejected(account),
        auth::unix_now(),
    )
}

/// Why no edit can go out, or the live session. A session 7TV rejected is gone but still
/// reads "Sign in again" (not "signed out") until the next sign-in.
fn standing(
    consent: bool,
    token: Option<&cyex_auth::SevenTvToken>,
    rejected: bool,
    now: i64,
) -> Option<Reachability> {
    if !consent {
        return Some(Reachability::NoConsent);
    }
    match token {
        None if rejected => Some(Reachability::SignInAgain),
        None => Some(Reachability::SignedOut),
        Some(t) if t.expired(now) => Some(Reachability::SignInAgain),
        Some(_) => None,
    }
}

/// The cached answer, if still fresh.
fn cached(app: &AppHandle, account: &str) -> Option<Reachability> {
    let reach = lock(&state(app).reach);
    let (at, r) = reach.get(account)?;
    (at.elapsed() < KEEP).then_some(if r.is_some() {
        Reachability::Ready
    } else {
        Reachability::NoAccount
    })
}

/// Reads what the account can edit, unless a fresh copy is kept (or `fresh`).
pub async fn warm(app: &AppHandle, account: &str, fresh: bool) -> Reachability {
    if let Some(s) = gate_state(app, account) {
        return s;
    }
    if !fresh && let Some(s) = cached(app, account) {
        return s;
    }
    let turn = gate(&state(app).warming, account);
    let _turn = turn.lock().await;
    if !fresh && let Some(s) = cached(app, account) {
        return s;
    }
    match read(&cosmetics::api(app), account).await {
        Ok(r) => {
            let st = state(app);
            lock(&st.down).remove(account);
            let now = if r.is_some() {
                Reachability::Ready
            } else {
                Reachability::NoAccount
            };
            lock(&st.reach).insert(account.to_owned(), (Instant::now(), r));
            now
        }
        Err(e) => {
            eprintln!("7tv edit: {account}: {e}");
            lock(&state(app).down).insert(account.to_owned());
            Reachability::Down
        }
    }
}

async fn read(api: &Client, account: &str) -> Result<Option<Reach>, Error> {
    let Some(me) = api.reach(account).await? else {
        return Ok(None);
    };
    let ids = reach::set_ids(&me);
    let sets = if ids.is_empty() {
        Vec::new()
    } else {
        api.edit_sets(&ids).await?
    };
    Ok(Some(reach::build(me, sets)))
}

/// Where things stand without a network call; a missing or stale copy starts a warm.
fn reachability(app: &AppHandle, account: &str) -> Reachability {
    if let Some(s) = gate_state(app, account).or_else(|| cached(app, account)) {
        return s;
    }
    let down = lock(&state(app).down).contains(account);
    let (app, owned) = (app.clone(), account.to_owned());
    tauri::async_runtime::spawn(async move {
        warm(&app, &owned, false).await;
    });
    if down {
        Reachability::Down
    } else {
        Reachability::Loading
    }
}

/// What the menus offer for this emote in this chat. No network.
pub fn options(app: &AppHandle, account: &str, login: &str, emote_id: &str, name: &str) -> EditOptions {
    let now = reachability(app, account);
    let found = find_emote(app, login, emote_id);
    let own_name = found
        .as_ref()
        .map_or(name, |e| e.original.as_deref().unwrap_or(&e.name));
    let facts = Facts {
        seventv: found
            .as_ref()
            .map_or(looks_seventv(emote_id), |e| e.provider == Provider::Seventv),
        emote_id,
        name,
        own_name,
        private: found.as_ref().is_some_and(|e| e.private),
        owner_id: found.as_ref().and_then(|e| e.owner_id.as_deref()),
    };
    let (owner, set) = viewed(app, login, emote_id, name);
    let viewed = Viewed {
        owner_id: owner.as_deref(),
        set_id: set.as_ref().map(|(id, _)| id.as_str()),
        in_set: set.as_ref().is_some_and(|(_, has)| *has),
    };
    let reach = lock(&state(app).reach);
    let r = reach.get(account).and_then(|(_, r)| r.as_ref());
    options::options(now, r, &viewed, &facts)
}

/// 26-character ULID, 7TV's id shape.
fn looks_seventv(id: &str) -> bool {
    id.len() == 26
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b.is_ascii_uppercase() && !b"ILOU".contains(&b)))
}

/// This chat's channel: its 7TV user, and its channel set with whether that holds (id, name).
fn viewed(app: &AppHandle, login: &str, id: &str, name: &str) -> (Option<String>, Option<(String, bool)>) {
    let st = app.state::<super::Emotes>();
    let inner = st.lock();
    let Some(c) = inner.channels.get(login) else {
        return (None, None);
    };
    let set = c
        .sets
        .iter()
        .find(|s| s.provider == Provider::Seventv && s.scope == Scope::Channel)
        .map(|s| {
            (
                s.id.clone(),
                s.emotes.iter().any(|e| e.id == id && e.name == name),
            )
        });
    (c.seventv_user.clone(), set)
}

/// The emote by id: this chat's sets, the global ones, personal sets, then other chats. A clone.
pub(super) fn find_emote(app: &AppHandle, login: &str, emote_id: &str) -> Option<Emote> {
    let st = app.state::<super::Emotes>();
    let inner = st.lock();
    let here = inner.channels.get(login).into_iter().flat_map(|c| c.sets.iter());
    let others = inner
        .channels
        .iter()
        .filter(|(l, _)| *l != login)
        .flat_map(|(_, c)| c.sets.iter());
    here.chain(inner.globals.iter())
        .chain(inner.personal_sets.values())
        .chain(others)
        .flat_map(|s| s.emotes.iter())
        .find(|e| e.id == emote_id)
        .map(|e| (**e).clone())
}

/// Whether `alias` is free in the set. Unknown set: free (the server decides).
pub fn name_check(
    app: &AppHandle,
    account: &str,
    set_id: &str,
    alias: &str,
    rename_from: Option<&str>,
) -> NameCheck {
    if rename_from == Some(alias) {
        return NameCheck::Same;
    }
    let reach = lock(&state(app).reach);
    let taken = reach
        .get(account)
        .and_then(|(_, r)| r.as_ref())
        .and_then(|r| r.targets.iter().find(|t| t.set_id == set_id))
        .is_some_and(|t| t.taken(alias));
    if taken { NameCheck::Taken } else { NameCheck::Free }
}

/// 1-100 characters, no spaces or control characters.
fn name_ok(name: &str) -> bool {
    (1..=100).contains(&name.chars().count()) && !name.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Runs one edit (V106-31/32, I106-3).
pub async fn run(app: &AppHandle, account: &str, op: EditOp) -> Result<EditDone, EditFail> {
    // Agents and test runs never edit real 7TV.
    if crate::paths::webview(app).is_some() && !cosmetics::mocked(app) {
        return Err(EditFail::TestRun);
    }
    session(app, account)?;
    if !name_ok(op.name()) {
        return Err(EditFail::BadName);
    }
    let turn = gate(&state(app).gates, op.set_id());
    let _turn = turn.lock().await;
    // Asked again: things may have changed while it waited.
    let token = session(app, account)?;
    lock(&state(app).echoes).expect(&op);
    let api = cosmetics::api(app);
    let secret = token.secret();
    let sent = match &op {
        EditOp::Add {
            set_id,
            emote_id,
            alias,
        } => api.add_emote(secret, set_id, emote_id, alias).await,
        EditOp::Remove {
            set_id,
            emote_id,
            alias,
        } => api.remove_emote(secret, set_id, emote_id, alias).await,
        EditOp::Rename {
            set_id,
            emote_id,
            from,
            to,
        } => api.rename_emote(secret, set_id, emote_id, from, to).await,
    };
    if let Err(e) = sent {
        // 7TV's event already showed it happened: the reply got lost, the edit didn't.
        let happened = lock(&state(app).echoes).forget(&op);
        if !happened {
            eprintln!("7tv edit: {account}: {e:?}");
            let tokens = &app.state::<Auth>().seventv;
            if e == EditError::SignInAgain && cosmetics::forget_if_same(tokens, account, &token) {
                auth::notify(app);
            }
            return Err(e.into());
        }
    }
    lock(&state(app).echoes).settle(&op, Instant::now());
    let channel = applied(app, &op);
    super::live::apply_local(app, &change_of(app, account, &op));
    Ok(EditDone {
        channel,
        undo: op.undo(),
    })
}

/// Consent and a live session, or why not.
fn session(app: &AppHandle, account: &str) -> Result<cyex_auth::SevenTvToken, EditFail> {
    let tokens = &app.state::<Auth>().seventv;
    let token = tokens.get(account);
    let why = standing(
        cosmetics::has_consent(app, account),
        token.as_ref(),
        tokens.rejected(account),
        auth::unix_now(),
    );
    match why {
        None => token.ok_or(EditFail::SignedOut),
        Some(Reachability::NoConsent) => Err(EditFail::NoConsent),
        Some(Reachability::SignInAgain) => Err(EditFail::SignInAgain),
        Some(_) => Err(EditFail::SignedOut),
    }
}

/// Our edit into every cached target of that set; its channel name.
fn applied(app: &AppHandle, op: &EditOp) -> String {
    let mut reach = lock(&state(app).reach);
    let mut channel = None;
    for t in reach
        .values_mut()
        .filter_map(|(_, r)| r.as_mut())
        .flat_map(|r| r.targets.iter_mut())
        .filter(|t| t.set_id == op.set_id())
    {
        t.apply(op);
        channel.get_or_insert_with(|| t.channel.clone());
    }
    channel.unwrap_or_else(|| "7TV".to_owned())
}

/// The change our edit made, as 7TV's event would tell it.
fn change_of(app: &AppHandle, account: &str, op: &EditOp) -> SetChange {
    let me = lock(&state(app).reach)
        .get(account)
        .and_then(|(_, r)| r.as_ref().map(|r| r.me.clone()))
        .unwrap_or_else(|| "You".to_owned());
    let named = |id: &str, name: &str| {
        let mut e = find_emote(app, "", id).unwrap_or_else(|| Emote::new(Provider::Seventv, id, name));
        let own = e.original.take().unwrap_or_else(|| e.name.clone());
        e.original = (own != name).then_some(own);
        e.name = name.to_owned();
        e.added_at = None;
        e.added_by = None;
        e
    };
    let mut c = SetChange {
        set_id: op.set_id().to_owned(),
        actor: Some(me),
        ..SetChange::default()
    };
    match op {
        EditOp::Add { emote_id, alias, .. } => c.added.push(named(emote_id, alias)),
        EditOp::Remove { emote_id, alias, .. } => c.removed.push((emote_id.clone(), alias.clone())),
        EditOp::Rename {
            emote_id, from, to, ..
        } => c
            .renamed
            .push((emote_id.clone(), from.clone(), named(emote_id, to))),
    }
    c
}

/// 7TV's event: every cached target of that set follows it. `c.removed` without our own
/// removals; those are `ours`, exact only (`unseen`).
pub fn on_set_changed(app: &AppHandle, c: &SetChange, ours: &[(String, String)]) {
    let Some(st) = app.try_state::<Edits>() else {
        return;
    };
    let mut reach = lock(&st.reach);
    for t in reach
        .values_mut()
        .filter_map(|(_, r)| r.as_mut())
        .flat_map(|r| r.targets.iter_mut())
        .filter(|t| t.set_id == c.set_id)
    {
        t.apply_change(c, ours);
    }
}

/// C1 What of 7TV's event still needs a line: `c` minus our own edits; and our removals in it,
/// which apply exactly (V105-04: never the id's other name).
pub fn unseen(app: &AppHandle, c: &SetChange) -> (SetChange, Vec<(String, String)>) {
    match app.try_state::<Edits>() {
        Some(st) => lock(&st.echoes).filter(c, Instant::now()),
        None => echo::Echoes::default().filter(c, Instant::now()),
    }
}
