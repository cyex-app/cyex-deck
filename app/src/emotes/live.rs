//! Live 7TV events: paints and badges people wear, personal emote sets (C9), and a
//! channel's set changing while you watch, announced in chat (C10).

use std::sync::Arc;

use cyex_chat::model::Provider;
use cyex_emotes::cosmetics::SevenChange;
use cyex_emotes::seventv::events::{Entitled, Event, SetChange};
use cyex_emotes::seventv::{self};
use cyex_emotes::{Emote, EmoteSet, Scope};
use cyex_twitch::pace::Lane;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::UnboundedReceiver;

use super::edit::slot_of;
use super::{Emotes, Inner, load, looks};
use crate::auth::Auth;

/// Sets pushed before anyone claimed them are kept this many at most.
const MAX_UNCLAIMED: usize = 500;

pub fn spawn(app: AppHandle, mut rx: UnboundedReceiver<Event>) {
    tauri::async_runtime::spawn(async move {
        while let Some(e) = rx.recv().await {
            handle(&app, e);
        }
    });
}

fn handle(app: &AppHandle, e: Event) {
    let state = app.state::<Emotes>();
    match e {
        Event::Paint(p) => state.lock().cosmetics.add_paint(p.style()),
        Event::Badge(b) => state.lock().cosmetics.add_badge(b),
        Event::Entitlement {
            added,
            kind,
            ref_id,
            twitch_ids,
        } => match kind {
            Entitled::Paint | Entitled::Badge => {
                let mut inner = state.lock();
                let changed: Vec<String> = twitch_ids
                    .into_iter()
                    .filter(|u| {
                        let change = match kind {
                            Entitled::Paint => SevenChange::Paint(Some(&ref_id)),
                            _ => SevenChange::Badge(Some(&ref_id)),
                        };
                        if added {
                            inner.cosmetics.set_seventv(u, change)
                        } else {
                            inner.cosmetics.take_seventv(u, change)
                        }
                    })
                    .collect();
                drop(inner);
                looks::changed(app, changed);
            }
            Entitled::EmoteSet => personal(app, added, ref_id, twitch_ids),
        },
        Event::SetChanged(c) => set_changed(app, c),
        Event::ActiveSet { twitch_id, set_id } => {
            let login = state
                .lock()
                .channels
                .iter()
                .find(|(_, c)| c.id == twitch_id)
                .map(|(l, _)| l.clone());
            if let Some(login) = login {
                crate::chat::system_line(app, &login, "The 7TV emote set changed.");
                load::seventv_set(app.clone(), login, set_id);
            }
        }
    }
}

/// C9 A user got or lost a personal (or special) 7TV set.
fn personal(app: &AppHandle, added: bool, set_id: String, users: Vec<String>) {
    let state = app.state::<Emotes>();
    let missing = {
        let mut inner = state.lock();
        // B9 The whole batch is stamped first, then rebuilt, then trimmed once: none of
        // its users is the least recent.
        for u in &users {
            if added {
                let sets = inner.user_sets.get_or_default(u.clone());
                if !sets.contains(&set_id) {
                    sets.push(set_id.clone());
                }
            } else if let Some(sets) = inner.user_sets.get_mut(u) {
                // V107-M3 Emptied lists leave the map (`rebuild_personal`).
                sets.retain(|s| *s != set_id);
            }
        }
        if added
            && !inner.personal_sets.contains_key(&set_id)
            && let Some(emotes) = inner.unclaimed.remove(&set_id)
        {
            let set = EmoteSet::new(set_id.clone(), Provider::Seventv, Scope::Personal, emotes);
            inner.personal_sets.insert(set_id.clone(), Arc::new(set));
        }
        for u in &users {
            inner.rebuild_personal(u);
        }
        inner.trim_personal();
        if !added {
            inner.prune_personal_sets(std::slice::from_ref(&set_id));
        }
        added && !inner.personal_sets.contains_key(&set_id)
    };
    if added && !missing {
        crate::chat::retokenize_users(app, &users);
    }
    if missing {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let http = app.state::<Auth>().http.clone();
            let sets = match seventv::api::sets(&http, std::slice::from_ref(&set_id), Lane::Back).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("emotes: no 7TV personal set {set_id}: {}", e.describe("7TV"));
                    return;
                }
            };
            let state = app.state::<Emotes>();
            let mut inner = state.lock();
            for mut set in sets {
                // B9 Only while one of its users is still kept, else it'd be orphaned.
                let held = users
                    .iter()
                    .any(|u| inner.user_sets.get(u.as_str()).is_some_and(|s| s.contains(&set.id)));
                if !held {
                    continue;
                }
                set.scope = Scope::Personal;
                inner.personal_sets.insert(set.id.clone(), Arc::new(set));
            }
            for u in &users {
                inner.rebuild_personal(u);
            }
            drop(inner);
            crate::chat::retokenize_users(&app, &users);
        });
    }
}

fn set_changed(app: &AppHandle, c: SetChange) {
    // C1 Our own edits already printed their line (`apply_local`); their removals apply exactly.
    let (shown, ours) = super::edit::unseen(app, &c);
    let c = SetChange {
        removed: shown.removed.clone(),
        ..c
    };
    super::edit::on_set_changed(app, &c, &ours);
    let state = app.state::<Emotes>();
    let mut notices: Vec<(String, Vec<String>)> = Vec::new();
    {
        let mut inner = state.lock();
        let logins = apply_to_channels(&mut inner, &c, &ours);
        if let Some(actor) = &c.actor {
            for login in &logins {
                notices.push((login.clone(), lines(actor, &shown)));
            }
        }
        if let Some(set) = inner.personal_sets.get(&c.set_id).cloned() {
            inner
                .personal_sets
                .insert(c.set_id.clone(), Arc::new(apply(&set, &c, &ours)));
            let owners: Vec<String> = inner
                .user_sets
                .iter()
                .filter(|(_, sets)| sets.contains(&c.set_id))
                .map(|(u, _)| u.clone())
                .collect();
            for u in owners {
                inner.rebuild_personal(&u);
            }
        } else if logins.is_empty() {
            unclaimed(&mut inner, c);
        }
    }
    for (login, texts) in notices {
        for t in texts {
            crate::chat::system_line(app, &login, &t);
        }
    }
}

/// The change into every loaded chat whose channel holds the set; their logins. `ours`: removals
/// that are our own, exact only.
fn apply_to_channels(inner: &mut Inner, c: &SetChange, ours: &[(String, String)]) -> Vec<String> {
    let logins: Vec<String> = inner
        .channels
        .iter()
        .filter(|(_, ch)| {
            ch.sets
                .iter()
                .any(|s| s.provider == Provider::Seventv && s.id == c.set_id)
        })
        .map(|(l, _)| l.clone())
        .collect();
    for login in &logins {
        if let Some(ch) = inner.channels.get_mut(login) {
            for s in ch
                .sets
                .iter_mut()
                .filter(|s| s.provider == Provider::Seventv && s.id == c.set_id)
            {
                *s = Arc::new(apply(s, c, ours));
            }
        }
        inner.rebuild(login);
    }
    logins
}

/// V106-31/32 Our own edit, before 7TV's event says so: only chats that loaded the set, no personal
/// or unclaimed sets. C1 Its C10 line once per chat; `edit::unseen` keeps the event from repeating it.
pub(crate) fn apply_local(app: &AppHandle, c: &SetChange) {
    let logins = {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        // V105-04 Our removal takes only the named entry.
        let rest = SetChange {
            set_id: c.set_id.clone(),
            actor: c.actor.clone(),
            added: c.added.clone(),
            removed: Vec::new(),
            renamed: c.renamed.clone(),
        };
        apply_to_channels(&mut inner, &rest, &c.removed)
    };
    if let Some(actor) = &c.actor {
        for login in logins {
            for t in lines(actor, c) {
                crate::chat::system_line(app, &login, &t);
            }
        }
    }
}

/// Personal sets fill before their entitlement arrives; keep the emotes until then.
fn unclaimed(inner: &mut Inner, c: SetChange) {
    if c.added.is_empty() {
        return;
    }
    if inner.unclaimed.len() >= MAX_UNCLAIMED && !inner.unclaimed.contains_key(&c.set_id) {
        let drop = inner.unclaimed.keys().next().cloned();
        if let Some(k) = drop {
            inner.unclaimed.remove(&k);
        }
    }
    inner.unclaimed.entry(c.set_id).or_default().extend(c.added);
}

/// `ours`: our own removals, which take only the named entry.
fn apply(set: &EmoteSet, c: &SetChange, ours: &[(String, String)]) -> EmoteSet {
    fn key(e: &Arc<Emote>) -> (&str, &str) {
        (e.id.as_str(), e.name.as_str())
    }
    // Ours: only the named entry.
    let mut emotes: Vec<Arc<Emote>> = set
        .emotes
        .iter()
        .filter(|e| !ours.iter().any(|(id, name)| *id == e.id && *name == e.name))
        .cloned()
        .collect();
    // V105-04 R13 Of two names for one emote, only the named one goes; an id held once goes
    // whatever its name (7TV's names can lag). Same rule as the edit cache (`slot_of`).
    for (id, name) in &c.removed {
        if let Some(n) = slot_of(&emotes, id, name, key) {
            emotes.remove(n);
        }
    }
    for (id, old, new) in &c.renamed {
        if let Some(n) = slot_of(&emotes, id, old, key) {
            let slot = &mut emotes[n];
            let mut e = new.clone();
            e.owner = e.owner.or_else(|| slot.owner.clone());
            e.added_at = slot.added_at.clone();
            e.added_by = e.added_by.or_else(|| slot.added_by.clone());
            *slot = Arc::new(e);
        }
    }
    for e in &c.added {
        if !emotes.iter().any(|x| x.id == e.id && x.name == e.name) {
            emotes.push(Arc::new(e.clone()));
        }
    }
    EmoteSet {
        id: set.id.clone(),
        provider: set.provider,
        scope: set.scope,
        emotes,
    }
}

/// C10 "xQc added the 7TV emote notL."
fn lines(actor: &str, c: &SetChange) -> Vec<String> {
    let mut out = Vec::new();
    for e in &c.added {
        out.push(format!("{actor} added the 7TV emote {}.", e.name));
    }
    for (_, name) in &c.removed {
        out.push(format!("{actor} removed the 7TV emote {name}."));
    }
    for (_, old, new) in &c.renamed {
        if *old != new.name {
            out.push(format!("{actor} renamed the 7TV emote {old} to {}.", new.name));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emote(id: &str, name: &str) -> Emote {
        Emote::new(Provider::Seventv, id, name)
    }

    #[test]
    fn changes_apply_and_read_well() {
        let set = EmoteSet::new(
            "s",
            Provider::Seventv,
            Scope::Channel,
            vec![emote("1", "a"), emote("2", "b")],
        );
        let c = SetChange {
            set_id: "s".into(),
            actor: Some("xQc".into()),
            added: vec![emote("3", "c")],
            removed: vec![("1".into(), "a".into())],
            renamed: vec![("2".into(), "b".into(), emote("2", "bee"))],
        };
        let next = apply(&set, &c, &[]);
        let names: Vec<&str> = next.emotes.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["bee", "c"]);
        assert_eq!(
            lines("xQc", &c),
            [
                "xQc added the 7TV emote c.",
                "xQc removed the 7TV emote a.",
                "xQc renamed the 7TV emote b to bee."
            ]
        );
    }

    #[test]
    fn remove_takes_one_of_two_names() {
        let set = EmoteSet::new(
            "s",
            Provider::Seventv,
            Scope::Channel,
            vec![emote("A", "a1"), emote("A", "a2")],
        );
        let c = SetChange {
            set_id: "s".into(),
            removed: vec![("A".into(), "a1".into())],
            ..SetChange::default()
        };
        let names: Vec<String> = apply(&set, &c, &[])
            .emotes
            .iter()
            .map(|e| e.name.clone())
            .collect();
        assert_eq!(names, ["a2"]);
        let one = EmoteSet::new("s", Provider::Seventv, Scope::Channel, vec![emote("A", "a1")]);
        let c = SetChange {
            set_id: "s".into(),
            removed: vec![("A".into(), "other".into())],
            ..SetChange::default()
        };
        assert!(apply(&one, &c, &[]).emotes.is_empty());
    }

    fn names(set: &EmoteSet) -> Vec<String> {
        set.emotes.iter().map(|e| e.name.clone()).collect()
    }

    #[test]
    fn rename_takes_the_named_of_two() {
        let set = EmoteSet::new(
            "s",
            Provider::Seventv,
            Scope::Channel,
            vec![emote("A", "a1"), emote("A", "a2")],
        );
        let c = SetChange {
            set_id: "s".into(),
            renamed: vec![("A".into(), "a2".into(), emote("A", "b"))],
            ..SetChange::default()
        };
        assert_eq!(names(&apply(&set, &c, &[])), ["a1", "b"]);
        // Held once: whatever its name.
        let one = EmoteSet::new("s", Provider::Seventv, Scope::Channel, vec![emote("A", "a1")]);
        let c = SetChange {
            set_id: "s".into(),
            renamed: vec![("A".into(), "lagging".into(), emote("A", "b"))],
            ..SetChange::default()
        };
        assert_eq!(names(&apply(&one, &c, &[])), ["b"]);
    }

    #[test]
    fn our_remove_keeps_the_second_name_either_order() {
        use crate::emotes::edit::Echoes;
        let set = EmoteSet::new(
            "s",
            Provider::Seventv,
            Scope::Channel,
            vec![emote("A", "a1"), emote("A", "a2")],
        );
        let op = crate::emotes::edit::EditOp::Remove {
            set_id: "s".into(),
            emote_id: "A".into(),
            alias: "a1".into(),
        };
        let mine = vec![("A".to_string(), "a1".to_string())];
        let local = |s: &EmoteSet| apply(s, &SetChange::default(), &mine);
        let event = SetChange {
            set_id: "s".into(),
            actor: Some("me".into()),
            removed: mine.clone(),
            ..SetChange::default()
        };
        // As `set_changed` does: the echo filter's own removals exact, the rest as 7TV says.
        let from_event = |e: &mut Echoes, s: &EmoteSet| {
            let (shown, ours) = e.filter(&event, std::time::Instant::now());
            assert!(shown.removed.is_empty());
            let c = SetChange {
                set_id: "s".into(),
                removed: shown.removed,
                ..SetChange::default()
            };
            apply(s, &c, &ours)
        };
        // Reply, then event.
        let mut e = Echoes::default();
        e.expect(&op);
        e.settle(&op, std::time::Instant::now());
        let after = from_event(&mut e, &local(&set));
        assert_eq!(names(&after), ["a2"]);
        // Event, then reply.
        let mut e = Echoes::default();
        e.expect(&op);
        let mid = from_event(&mut e, &set);
        e.settle(&op, std::time::Instant::now());
        assert_eq!(names(&local(&mid)), ["a2"]);
    }
}
