//! V106-31/32 Runs one edit and shows it at once (C1).

use std::time::Instant;

use cyex_chat::model::Provider;
use cyex_emotes::Emote;
use cyex_emotes::seventv::account::EditError;
use cyex_emotes::seventv::events::SetChange;
use tauri::{AppHandle, Manager};

use super::ask::find_emote;
use super::ops::*;
use super::{gate, lock, session, state};
use crate::auth::{self, Auth};
use crate::cosmetics;

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
            // R2 A new sign-in replaced the token while this was out: not "Sign in again".
            if e == EditError::SignInAgain {
                if cosmetics::forget_if_same(tokens, account, &token) {
                    auth::notify(app);
                    return Err(EditFail::SignInAgain);
                }
                return Err(EditFail::Down);
            }
            return Err(e.into());
        }
    }
    lock(&state(app).echoes).settle(&op, Instant::now());
    let channel = applied(app, &op);
    crate::emotes::live::apply_local(app, &change_of(app, account, &op));
    Ok(EditDone {
        channel,
        undo: op.undo(),
    })
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
