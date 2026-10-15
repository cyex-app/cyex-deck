//! The app's own chat commands. Chatty's `/chain`, `/foreach`, `/runin`, `/timer` (S3),
//! `/set var`, `/echo`; viewer tools (`/user`, `/logs`, `/block`, `/hideuser` C28,
//! `/mutepings` C27, `/tl` and `/translate` C17, `/vote` C4, `/search` F1); `/me` and
//! everything else go on to Twitch or say why not.

use cyex_chat::commands::chain::{split_chain, split_foreach};
use cyex_chat::model::{ModRole, NoticeKind};
use cyex_moderation::command::Need;
use cyex_settings::Partition;
use cyex_settings::schema::{Commands, HiddenUser, Highlights};
use cyex_twitch::{gql, helix};
use tauri::{AppHandle, Manager};

use super::{Place, UiAction, fill, line, local, params, send};
use crate::settings::Settings;

/// Twitch's mod and broadcaster commands; they come with the mod tools (phase 10).
pub(super) const MOD_COMMANDS: &[&str] = &[
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
        "me" => send(app, place, text, reply_to).await.map(|()| None),
        "echo" => {
            local(app, &place.key, NoticeKind::Info, args);
            done
        }
        "help" | "commands" => {
            local(app, &place.key, NoticeKind::Info, &super::registry::help_line());
            done
        }
        // V1051-04
        "combo" | "paint" | "badge" => super::cosmetics::run(app, place, name, args).await,
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
                .ok_or_else(|| format!("Not in {chan}’s chat."))?;
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
                "won’t play sounds or flash now."
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
            send(app, place, &out, reply_to).await.map(|()| None)
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
        // V1051-03 Needs an account; the pop-up itself is in the window.
        "cosmetics" => {
            if place.account().is_none() {
                local(app, &place.key, NoticeKind::Info, "Sign in to pick a 7TV paint and badge.");
                return Ok(None);
            }
            Ok(Some(UiAction::Cosmetics))
        }
        "color" => {
            let account = place.account().ok_or("Sign in to set a color.")?;
            super::super::identity::set_color(app, account, args.trim()).await?;
            local(app, &place.key, NoticeKind::Info, "Name color changed.");
            done
        }
        // 1.26 Whispers go out from the default account, like the whispers view.
        "w" | "whisper" => {
            let (to, text) = args.trim().split_once(' ').ok_or("Usage: /w name message")?;
            whisper(app, place, to, text).await
        }
        "r" => {
            let to = crate::whispers::reply_target(app).ok_or("Nobody has whispered you yet.")?;
            whisper(app, place, &to, args).await
        }
        "nuke" => {
            let account = place.account().ok_or("Sign in to use /nuke.")?.to_owned();
            crate::moderation::nuke::run(app, &account, place.channel(), args).await?;
            done
        }
        _ if is_mod_command(name) => run_mod(app, place, name, args).await,
        _ => Err(format!("Unknown command /{name}. /help lists them.")),
    }
}

/// Sends a whisper; with inline whispers off, a short note says it went.
async fn whisper(app: &AppHandle, place: &Place, to: &str, text: &str) -> Result<Option<UiAction>, String> {
    crate::whispers::send(app, to, text).await?;
    let n: cyex_settings::schema::Notifications = app.state::<Settings>().read(Partition::Notifications);
    if !n.whispers.inline {
        local(app, &place.key, NoticeKind::Info, &format!("Whisper sent to {}.", to.trim_start_matches('@')));
    }
    Ok(None)
}

/// The app's own commands, which plugins can't take over (S5).
pub(super) const OWN: &[&str] = &[
    "me", "echo", "help", "commands", "set", "chain", "foreach", "runin", "timer", "user", "usercard", "logs",
    "block", "unblock", "hideuser", "unhideuser", "mutepings", "tl", "translate", "vote", "search", "popout",
    "color", "cosmetics", "w", "whisper", "r", "nuke",
    // V1051-04
    "combo", "paint", "badge",
];

/// S5 Names plugins can't register: the app's commands and Twitch's.
pub fn reserved(name: &str) -> bool {
    OWN.contains(&name) || is_mod_command(name)
}

/// Twitch's own mod and broadcaster commands (1.49, 1.50), run through `moderation`.
fn is_mod_command(name: &str) -> bool {
    cyex_moderation::command::NAMES.contains(&name) || MOD_COMMANDS.contains(&name)
}

/// V1041-01 The app's commands that send or act as the chat's account.
pub(super) const SIGNED_IN: &[&str] = &["me", "tl", "vote", "color", "cosmetics", "block", "unblock", "w", "whisper", "r",
    // V1051-04
    "combo", "paint", "badge",
];

/// V1041-01 Who may run a built-in (1.57): Twitch's mod commands as they parse, `/nuke` any
/// mod, the rest anyone.
pub(super) fn need(name: &str) -> Option<Need> {
    cyex_moderation::command::need(name).or_else(|| is_mod_command(name).then_some(Need::Mod))
}

/// V1041-01 The app's and Twitch's commands a chat can use, for Tab completion: mod-only
/// ones where the role allows (`Need::allows`, V102-21), none that send in an anonymous
/// chat. Sorted, no `/`.
pub fn usable(role: Option<ModRole>, signed_in: bool) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = OWN
        .iter()
        .chain(MOD_COMMANDS)
        .chain(cyex_moderation::command::NAMES)
        .copied()
        .filter(|n| need(n).is_none_or(|it| it.allows(role)) && (signed_in || !SIGNED_IN.contains(n)))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

async fn run_mod(app: &AppHandle, place: &Place, name: &str, args: &str) -> Result<Option<UiAction>, String> {
    let account = place.account().ok_or("Sign in with a moderator account to use that.")?.to_owned();
    let Some(parsed) = cyex_moderation::command::parse(name, args) else {
        return Err(format!("/{name} isn’t supported yet."));
    };
    let cmd = parsed?;
    if let Some(line) = crate::moderation::run::command(app, &account, place.channel(), cmd).await? {
        local(app, &place.key, NoticeKind::Info, &line);
    }
    Ok(None)
}

/// The name after `/user`, `/logs`, `/block` and the like: `@Name`, `name` or `name,`,
/// checked like the mod commands' (V102-10).
fn login_arg(args: &str) -> Result<String, String> {
    match args.split_whitespace().next() {
        None => Err("Say who: a name after the command.".into()),
        Some(word) => cyex_moderation::command::login(word).ok_or_else(|| format!("{word} isn’t a Twitch name.")),
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
    block_user(app, &account, login, on).await
}

/// F2 Blocks or unblocks `login` for `account`; also used by whispers (1.26).
pub async fn block_user(app: &AppHandle, account: &str, login: &str, on: bool) -> Result<(), String> {
    let account = account.to_owned();
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
        return Err("This account can’t block people. Sign in again in Settings → Accounts.".into());
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
        .map_err(|_| "Translation isn’t working right now.".to_owned())?;
    out.into_iter()
        .next()
        .map(|t| t.text)
        .ok_or_else(|| "Nothing came back.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_takes_one_twitch_name() {
        // V102-10 `/user <name>` opens that name's card in this chat.
        assert_eq!(login_arg("@SomeOne").unwrap(), "someone");
        assert_eq!(login_arg("  foo_bar,  extra words").unwrap(), "foo_bar");
        assert!(login_arg("").unwrap_err().starts_with("Say who"));
        // Nothing that would break the card's window route.
        assert!(login_arg("a/b").is_err());
        assert!(login_arg("ünïcode").is_err());
    }

    #[test]
    fn mod_commands_are_twitchs() {
        // V102-11 They run through the mod tools, so plugins can't take them over.
        for name in ["ban", "timeout", "unban", "untimeout", "warn"] {
            assert!(is_mod_command(name) && reserved(name), "{name}");
        }
        assert!(reserved("user"));
    }

    #[test]
    fn completion_offers_what_the_chat_can_use() {
        // V1041-01 Every built-in is offered somewhere: mod commands have a need.
        for name in OWN.iter().chain(MOD_COMMANDS) {
            assert!(!is_mod_command(name) || need(name).is_some(), "{name}");
        }
        let has = |list: &[&str], names: &[&str]| names.iter().all(|n| list.contains(n));
        let none = |list: &[&str], names: &[&str]| names.iter().all(|n| !list.contains(n));
        // Anonymous: nothing that sends.
        let anon = usable(None, false);
        assert!(has(&anon, &["user", "usercard", "logs", "search", "popout", "help", "echo"]));
        assert!(none(&anon, &["me", "w", "r", "tl", "color", "cosmetics", "block", "vote", "ban", "nuke"]));
        let viewer = usable(None, true);
        assert!(has(&viewer, &["me", "w", "whisper", "tl", "color", "cosmetics", "user"]));
        assert!(none(&viewer, &["ban", "timeout", "unban", "warn", "nuke", "slow", "mods"]));
        let m = usable(Some(ModRole::Moderator), true);
        assert!(has(&m, &["ban", "timeout", "unban", "untimeout", "warn", "nuke", "so", "pin", "r9kbeta"]));
        assert!(none(&m, &["mod", "unmod", "vip", "unvip", "raid", "mods", "commercial"]));
        let lead = usable(Some(ModRole::LeadModerator), true);
        assert!(has(&lead, &["mod", "unvip"]) && none(&lead, &["raid", "vips", "marker"]));
        let owner = usable(Some(ModRole::Broadcaster), true);
        assert!(has(&owner, &["raid", "unraid", "mods", "vips", "commercial", "marker", "ban"]));
        // V1051-04 The 7TV commands are offered signed in, never anonymous.
        assert!(has(&viewer, &["combo", "paint", "badge"]) && none(&anon, &["combo", "paint", "badge"]));
        assert!(super::super::cosmetics::NAMES.iter().all(|n| OWN.contains(n)));
        // Sorted, once each.
        assert!(owner.windows(2).all(|w| w[0] < w[1]));
    }
}
