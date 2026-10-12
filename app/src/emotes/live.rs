//! Live 7TV events: paints and badges people wear, personal emote sets (C9), and a
//! channel's set changing while you watch, announced in chat (C10).

use std::sync::Arc;

use cyex_chat::model::Provider;
use cyex_emotes::cosmetics::SevenChange;
use cyex_emotes::seventv::events::{Entitled, Event, SetChange};
use cyex_emotes::seventv::{self};
use cyex_emotes::{Emote, EmoteSet, Scope};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::UnboundedReceiver;

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
        for u in &users {
            let sets = inner.user_sets.entry(u.clone()).or_default();
            if added && !sets.contains(&set_id) {
                sets.push(set_id.clone());
            } else if !added {
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
        added && !inner.personal_sets.contains_key(&set_id)
    };
    if added && !missing {
        crate::chat::retokenize_users(app, &users);
    }
    if missing {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let http = app.state::<Auth>().http.clone();
            let Ok(sets) = seventv::api::sets(&http, std::slice::from_ref(&set_id)).await else {
                return;
            };
            let state = app.state::<Emotes>();
            let mut inner = state.lock();
            for mut set in sets {
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
    let state = app.state::<Emotes>();
    let mut notices: Vec<(String, Vec<String>)> = Vec::new();
    {
        let mut inner = state.lock();
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
                    *s = Arc::new(apply(s, &c));
                }
            }
            inner.rebuild(login);
            if let Some(actor) = &c.actor {
                notices.push((login.clone(), lines(actor, &c)));
            }
        }
        if let Some(set) = inner.personal_sets.get(&c.set_id).cloned() {
            inner
                .personal_sets
                .insert(c.set_id.clone(), Arc::new(apply(&set, &c)));
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

fn apply(set: &EmoteSet, c: &SetChange) -> EmoteSet {
    let mut emotes: Vec<Arc<Emote>> = set
        .emotes
        .iter()
        .filter(|e| !c.removed.iter().any(|(id, _)| *id == e.id))
        .cloned()
        .collect();
    for (id, _, new) in &c.renamed {
        if let Some(slot) = emotes.iter_mut().find(|e| e.id == *id) {
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
        let next = apply(&set, &c);
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
}
