//! The app's own chat commands. Chatty's `/chain`, `/foreach`, `/runin`, `/timer` (S3),
//! `/set var`, `/echo`; viewer tools (`/user`, `/logs`, `/block`, `/hideuser` C28,
//! `/mutepings` C27, `/tl` and `/translate` C17, `/vote` C4, `/search` F1); `/me` and
//! everything else go on to Twitch or say why not.

use cyex_chat::commands::chain::{split_chain, split_foreach};
use cyex_chat::model::NoticeKind;
use cyex_settings::Partition;
use cyex_settings::schema::{Commands, HiddenUser, Highlights};
use cyex_twitch::{gql, helix};
use tauri::{AppHandle, Manager};

use super::{Place, UiAction, fill, line, local, params, send};
use crate::settings::Settings;

/// Twitch's mod and broadcaster commands; they come with the mod tools (phase 10).
const MOD_COMMANDS: &[&str] = &[
    "ban",
    "unban",
    "timeout",
    "untimeout",
    "delete",
    "clear",
    "mod",
    "unmod",
    "vip",
    "unvip",
    "slow",
    "slowoff",
    "followers",
    "followersoff",
    "subscribers",
    "subscribersoff",
    "emoteonly",
    "emoteonlyoff",
    "uniquechat",
    "uniquechatoff",
    "announce",
    "raid",
    "unraid",
    "marker",
    "commercial",
    "shoutout",
    "warn",
    "pin",
    "unpin",
    "shield",
    "shieldoff",
    "nuke",
    "monitor",
    "restrict",
    "unmonitor",
    "unrestrict",
];

const HELP: &str = "/me, /echo, /set var, /chain, /foreach, /runin, /timer, /user, /logs, /block, /unblock, /hideuser, /unhideuser, /mutepings, /tl, /translate, /vote, /search, /popout, /color. Your own commands are in Settings → Commands.";

pub async fn run(
    app: &AppHandle,
    place: &Place,
    name: &str,
    args: &str,
    text: &str,
    reply_to: Option<String>,
    depth: u32,
) -> Result<Option<UiAction>, String> {
    let done = Ok(None);
    match name {
        "me" => send(app, place, text, reply_to).map(|()| None),
        "echo" => {
            local(app, &place.key, NoticeKind::Info, args);
            done
        }
        "help" | "commands" => {
            local(app, &place.key, NoticeKind::Info, HELP);
            done
        }
        "set" => set_var(app, args).map(|()| None),
        "chain" => {
            for part in split_chain(args) {
                line(app, place, &part, None, true, depth + 1).await?;
            }
            done
        }
        "foreach" => {
            let (items, command) = split_foreach(args).ok_or("Usage: /foreach a b c > /command $1")?;
            for item in items {
                let p = params(app, place, &item);
                if let Some(filled) = fill(app, place, &command, p).await? {
                    line(app, place, &filled, None, true, depth + 1).await?;
                }
            }
            done
        }
        "runin" => {
            let (chan, command) = args
                .split_once(char::is_whitespace)
                .ok_or("Usage: /runin channel /command")?;
            let chan = chan.trim_start_matches('#').to_ascii_lowercase();
            let key = super::super::room_for(app, &place.key.0, &chan)
                .ok_or_else(|| format!("Not in {chan}'s chat."))?;
            let there = Place { view: None, key };
            line(app, &there, command, None, true, depth + 1).await
        }
        "timer" => super::timers::command(app, place, args).map(|()| None),
        "user" | "usercard" => {
            let login = login_arg(args)?;
            crate::windows::open_usercard(app, place.channel(), &login).map_err(|e| e.to_string())?;
            done
        }
        "logs" => {
            let login = login_arg(args)?;
            crate::commands::system::open_allowed(app, cyex_twitch::logs::page_url(place.channel(), &login))?;
            done
        }
        "block" | "unblock" => {
            let login = login_arg(args)?;
            block(app, place, &login, name == "block").await?;
            let verb = if name == "block" { "blocked" } else { "unblocked" };
            local(app, &place.key, NoticeKind::Info, &format!("You {verb} {login}."));
            done
        }
        "hideuser" | "unhideuser" => {
            let login = login_arg(args)?;
            hide(app, &login, name == "hideuser").await?;
            let what = if name == "hideuser" {
                "is hidden now. /unhideuser brings them back."
            } else {
                "shows again."
            };
            local(app, &place.key, NoticeKind::Info, &format!("{login} {what}"));
            done
        }
        "mutepings" => {
            let muted = toggle_muted(app, place.channel())?;
            let what = if muted {
                "won't play sounds or flash now."
            } else {
                "plays sounds and flashes again."
            };
            local(app, &place.key, NoticeKind::Info, &format!("This chat {what}"));
            done
        }
        "tl" => {
            let (lang, rest) = args
                .split_once(char::is_whitespace)
                .ok_or("Usage: /tl de your message")?;
            let out = translate_one(app, rest.trim(), lang).await?;
            send(app, place, &out, reply_to).map(|()| None)
        }
        "translate" => {
            if args.is_empty() {
                return Err("Usage: /translate text".into());
            }
            let to = super::super::translate::target(app);
            let out = translate_one(app, args, &to).await?;
            local(app, &place.key, NoticeKind::Info, &out);
            done
        }
        "vote" => {
            let n: usize = args.trim().parse().map_err(|_| "Usage: /vote 1")?;
            super::super::votes::vote_index(app, place, n)
                .await
                .map(|()| None)
        }
        "search" => Ok(Some(UiAction::Search {
            query: args.to_owned(),
        })),
        "popout" => Ok(Some(UiAction::Popout)),
        "color" => {
            let account = place.account().ok_or("Sign in to set a color.")?;
            super::super::identity::set_color(app, account, args.trim()).await?;
            local(app, &place.key, NoticeKind::Info, "Name color changed.");
            done
        }
        "w" | "whisper" => Err("Whispers come in a later update.".into()),
        "nuke" => {
            let account = place.account().ok_or("Sign in to use /nuke.")?.to_owned();
            crate::moderation::nuke::run(app, &account, place.channel(), args).await?;
            done
        }
        _ if is_mod_command(name) => run_mod(app, place, name, args).await,
        _ => Err(format!("Unknown command /{name}. /help lists them.")),
    }
}

/// Twitch's own mod and broadcaster commands (1.49, 1.50), run through `moderation`.
fn is_mod_command(name: &str) -> bool {
    cyex_moderation::command::NAMES.contains(&name) || MOD_COMMANDS.contains(&name)
}

async fn run_mod(app: &AppHandle, place: &Place, name: &str, args: &str) -> Result<Option<UiAction>, String> {
    let account = place.account().ok_or("Sign in with a moderator account to use that.")?.to_owned();
    let Some(parsed) = cyex_moderation::command::parse(name, args) else {
        return Err(format!("/{name} isn't supported yet."));
    };
    let cmd = parsed?;
    if let Some(line) = crate::moderation::run::command(app, &account, place.channel(), cmd).await? {
        local(app, &place.key, NoticeKind::Info, &line);
    }
    Ok(None)
}

fn login_arg(args: &str) -> Result<String, String> {
    let login = args
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_start_matches('@')
        .to_ascii_lowercase();
    if login.is_empty() {
        Err("Say who: a login after the command.".into())
    } else {
        Ok(login)
    }
}

/// `/set var key value`; an empty value removes the key.
fn set_var(app: &AppHandle, args: &str) -> Result<(), String> {
    let mut words = args.splitn(3, ' ');
    if words.next() != Some("var") {
        return Err("Only /set var key value works.".into());
    }
    let key = words
        .next()
        .filter(|k| !k.is_empty())
        .ok_or("Usage: /set var key value")?
        .to_owned();
    let value = words.next().unwrap_or("").to_owned();
    app.state::<Settings>()
        .update::<Commands>(app, Partition::Commands, |c| {
            if value.is_empty() {
                c.vars.remove(&key).is_some()
            } else {
                c.vars.insert(key, value.clone()).as_deref() != Some(value.as_str())
            }
        })
}

async fn user_id(app: &AppHandle, login: &str) -> Result<String, String> {
    let http = app.state::<crate::auth::Auth>().http.clone();
    gql::profile(&http, Some(login), None)
        .await
        .map_err(|e| e.to_string())?
        .map(|p| p.id)
        .ok_or_else(|| format!("No user called {login}."))
}

/// F2 Blocks or unblocks on Twitch, Helix first, else GQL.
async fn block(app: &AppHandle, place: &Place, login: &str, on: bool) -> Result<(), String> {
    let account = place.account().ok_or("Sign in to block people.")?.to_owned();
    let target = user_id(app, login).await?;
    let http = app.state::<crate::auth::Auth>().http.clone();
    let result = if let Some((token, client_id)) =
        crate::auth::helix_scoped(app, &account, "user:manage:blocked_users")
    {
        let auth = helix::Auth {
            client_id: &client_id,
            token: &token,
        };
        if on {
            helix::blocks::block(&http, auth, &target).await
        } else {
            helix::blocks::unblock(&http, auth, &target).await
        }
    } else if let Some((token, client_id)) = crate::auth::gql_token(app, &account) {
        let auth = gql::Auth {
            client_id: &client_id,
            token: Some(&token),
        };
        if on {
            gql::blocks::block(&http, auth, &target).await
        } else {
            gql::blocks::unblock(&http, auth, &target).await
        }
    } else {
        return Err("This account can't block people. Sign in again in Settings → Accounts.".into());
    };
    result.map_err(|e| e.to_string())?;
    super::super::rules::note_block(app, &account, &target, on);
    Ok(())
}

/// C28 Hidden only here, not on Twitch.
async fn hide(app: &AppHandle, login: &str, on: bool) -> Result<(), String> {
    let id = user_id(app, login).await?;
    app.state::<Settings>()
        .update::<Highlights>(app, Partition::Highlights, |h| {
            let had = h.hidden_users.iter().any(|u| u.id == id);
            if on && !had {
                h.hidden_users.push(HiddenUser {
                    id,
                    login: login.to_owned(),
                });
                true
            } else if !on && had {
                h.hidden_users.retain(|u| u.id != id);
                true
            } else {
                false
            }
        })
}

/// C27 Returns whether the chat is muted now.
fn toggle_muted(app: &AppHandle, channel: &str) -> Result<bool, String> {
    let mut muted = false;
    app.state::<Settings>()
        .update::<Highlights>(app, Partition::Highlights, |h| {
            if let Some(i) = h.muted.iter().position(|c| c == channel) {
                h.muted.remove(i);
            } else {
                h.muted.push(channel.to_owned());
                muted = true;
            }
            true
        })?;
    Ok(muted)
}

async fn translate_one(app: &AppHandle, text: &str, to: &str) -> Result<String, String> {
    let http = app.state::<crate::auth::Auth>().http.clone();
    let out = cyex_twitch::translate::translate(&http, &[text.to_owned()], to)
        .await
        .map_err(|_| "Translation isn't working right now.".to_owned())?;
    out.into_iter()
        .next()
        .map(|t| t.text)
        .ok_or_else(|| "Nothing came back.".into())
}
