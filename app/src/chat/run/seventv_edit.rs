//! I106-3 `/add`, `/remove`, `/rename`: change the 7TV emotes of the channel this chat is in,
//! through the same `edit::run` as the menus (consent, session, test-run guard, name check,
//! one edit per set). Results are local lines or a toast; nothing goes to the channel.
//! V111-03 A `#channel` word does it in that channel's set instead (the change line goes to
//! its chats, the reply and Undo stay here); `/remove` takes several names; `/steal` (`/yoink`)
//! takes an emote from `#channel` into this one; `/whoadded` says who added an emote and when.

use cyex_chat::model::NoticeKind;
use cyex_emotes::Emote;
use tauri::AppHandle;

use super::{Place, UiAction, local};
use crate::cosmetics::{SelectError, Why, select_error, why};
use crate::emotes::edit::{self, Blocked, EditDone, EditFail, EditOp, Reachability, SourceFail, Target};

/// The arm in `builtin::run` and `OWN` must list these (checked in builtin's tests).
#[cfg(test)]
pub(super) const NAMES: [&str; 6] = ["add", "remove", "rename", "steal", "yoink", "whoadded"];

const USAGE_ADD: &str = "Say which emote: /add <7TV link or id> [name] [#channel].";
const USAGE_REMOVE: &str = "Say which emote: /remove <name> [more names] [#channel].";
const USAGE_RENAME: &str = "Say which emote: /rename <name> [new name] [#channel].";
// V111-03
const USAGE_STEAL: &str = "Say which emote and where from: /steal <name> [new name] #channel.";
const USAGE_WHO: &str = "Say which emote: /whoadded <name> [#channel].";
const STEAL_HERE: &str = "That’s this chat’s channel. Say where from: /steal <name> #channel.";
const ONE_CHANNEL: &str = "Name one channel with #.";
const TOO_MANY: &str = "At most 20 at a time.";
const MAX_REMOVE: usize = 20;
const SPACES: &str = "Emote names have no spaces.";
const NOT_LINK: &str = "That isn’t a 7TV emote link or id.";
const NO_EMOTE: &str = "No 7TV emote with that id.";
const ALREADY: &str = "Already in {channel}’s 7TV emotes.";
const NO_SUCH: &str = "{channel} has no 7TV emote called {name}.";
const SAME: &str = "It’s called {from} already.";

#[derive(Debug, PartialEq, Eq)]
enum Parsed {
    Add {
        id: String,
        name: Option<String>,
    },
    /// V111-03 One or more, once each, in the order typed.
    Remove {
        names: Vec<String>,
    },
    /// `to: None` goes back to the emote's own 7TV name (C2).
    Rename {
        from: String,
        to: Option<String>,
    },
    /// V111-03 From the `#channel`, into this chat's channel.
    Steal {
        name: String,
        alias: Option<String>,
    },
    /// V111-03
    WhoAdded {
        name: String,
    },
}

/// `/add`'s first word: a 7tv.app emote link or a bare id, as an upper-case ULID.
fn emote_id(word: &str) -> Option<String> {
    const MARK: &str = "7tv.app/emotes/";
    let lower = word.to_ascii_lowercase();
    let id = match lower.find(MARK) {
        Some(at) => {
            let head = &lower[..at];
            let head = head
                .strip_prefix("https://")
                .or_else(|| head.strip_prefix("http://"))
                .unwrap_or(head);
            let head = head
                .strip_prefix("www.")
                .or_else(|| head.strip_prefix("old."))
                .unwrap_or(head);
            if !head.is_empty() {
                return None;
            }
            let tail = &word[at + MARK.len()..];
            &tail[..tail.find(['/', '?', '#']).unwrap_or(tail.len())]
        }
        None => word,
    };
    let id = id.to_ascii_uppercase();
    cyex_emotes::seventv::is_id(&id).then_some(id)
}

/// V111-03 The words without the `#channel` one (anywhere, at most one), and that channel's
/// login in lower case.
fn split_channel(args: &str) -> Result<(Vec<&str>, Option<String>), String> {
    let mut words = Vec::new();
    let mut channel = None;
    for w in args.split_whitespace() {
        let Some(login) = w.strip_prefix('#') else {
            words.push(w);
            continue;
        };
        let login = login.to_ascii_lowercase();
        if !edit::is_login(&login) {
            return Err(format!("{w} isn’t a channel name."));
        }
        if channel.is_some() {
            return Err(ONE_CHANNEL.into());
        }
        channel = Some(login);
    }
    Ok((words, channel))
}

/// Words are split on white space: emote names have no spaces, so there are no quotes.
/// V111-03 Also gives the `#channel`.
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
            name: second,
        },
        "remove" => {
            let mut names: Vec<String> = Vec::new();
            for w in words {
                if !names.iter().any(|n| n == w) {
                    names.push(w.to_owned());
                }
            }
            Parsed::Remove { names }
        }
        "steal" | "yoink" => {
            if channel.is_none() {
                return Err(usage.into());
            }
            Parsed::Steal {
                name: first,
                alias: second,
            }
        }
        "whoadded" => Parsed::WhoAdded { name: first },
        _ => Parsed::Rename { from: first, to: second },
    };
    Ok((cmd, channel))
}

/// V111-03 Why `pick` found nothing.
#[derive(Debug, PartialEq, Eq)]
enum Pick {
    Missing,
    /// The names that match up to case.
    Ambiguous(Vec<String>),
}

/// V111-03 The emote named `name`: the exact name, else the only one that matches up to case.
fn pick<'a>(emotes: &'a [Emote], name: &str) -> Result<&'a Emote, Pick> {
    if let Some(e) = emotes.iter().find(|e| e.name == name) {
        return Ok(e);
    }
    let like: Vec<&Emote> = emotes.iter().filter(|e| e.name.eq_ignore_ascii_case(name)).collect();
    match like.as_slice() {
        [] => Err(Pick::Missing),
        [one] => Ok(one),
        many => Err(Pick::Ambiguous(many.iter().map(|e| e.name.clone()).collect())),
    }
}

fn pick_line(p: Pick, channel: &str, name: &str) -> String {
    match p {
        Pick::Missing => NO_SUCH.replace("{channel}", channel).replace("{name}", name),
        Pick::Ambiguous(names) => {
            let shown: Vec<&str> = names.iter().take(5).map(String::as_str).collect();
            format!("{channel} has more than one: {}. Type it exactly.", shown.join(", "))
        }
    }
}

/// V111-03 Why a channel's emotes can't be read, as a line.
fn source_line(f: SourceFail, channel: &str) -> String {
    match f {
        SourceFail::NoChannel => format!("No Twitch channel called {channel}."),
        SourceFail::NoSet => format!("{channel} has no 7TV emotes."),
        SourceFail::Down => why(&Why::Down).to_owned(),
    }
}

/// V111-03 "a", "a and b", "a, b and c" (`last` = "and" or "or").
fn joined(names: &[String], last: &str) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [head @ .., tail] => format!("{} {last} {tail}", head.join(", ")),
    }
}

/// A stop for sign-in becomes the pop-up in the chat view that ran it, else the usual line.
fn sign_in(ui: bool, e: SelectError, command: String) -> Result<Option<UiAction>, String> {
    if ui {
        Ok(Some(UiAction::SevenTvSignIn {
            command,
            again: e == SelectError::SignInAgain,
        }))
    } else {
        Err(select_error(&e))
    }
}

/// The line for a failed edit; the same words as the UI's `failText`.
pub(super) fn fail_line(e: &EditFail, channel: &str, alias: &str) -> String {
    match e {
        EditFail::SignInAgain => select_error(&SelectError::SignInAgain),
        EditFail::SignedOut => select_error(&SelectError::SignedOut),
        EditFail::NoConsent => why(&Why::NoConsent).to_owned(),
        EditFail::RateLimited { seconds } => select_error(&SelectError::RateLimited { seconds: *seconds }),
        EditFail::NotAllowed => format!("You can’t edit {channel}’s 7TV emotes."),
        EditFail::Full => format!("{channel}’s 7TV emotes are full."),
        EditFail::NameTaken => format!("{channel} already has an emote named {alias}."),
        EditFail::Private => "That emote is private. Only its owner can add it.".into(),
        EditFail::NotFound => "That emote isn’t there anymore.".into(),
        EditFail::BadName => "7TV doesn’t allow that name.".into(),
        EditFail::Down => "Couldn’t reach 7TV. Nothing changed.".into(),
        EditFail::TestRun => "7TV edits only go to the test mock here.".into(),
        EditFail::Bad => "Something went wrong at 7TV. Nothing changed.".into(),
    }
}

/// V111-03 A failure after which the other names of a `/remove` aren't tried.
fn stops(e: &EditFail) -> bool {
    matches!(
        e,
        EditFail::SignInAgain
            | EditFail::SignedOut
            | EditFail::NoConsent
            | EditFail::RateLimited { .. }
            | EditFail::TestRun
            | EditFail::NotAllowed
    )
}

/// The set's entry with that name (exact, case-sensitive).
fn entry<'a>(t: &'a Target, name: &str) -> Result<&'a (String, String), String> {
    t.entries
        .iter()
        .find(|(_, a)| a == name)
        .ok_or_else(|| NO_SUCH.replace("{channel}", &t.channel).replace("{name}", name))
}

fn down<T>(_: T) -> String {
    why(&Why::Down).to_owned()
}

/// R14 The menu's checks for an add, in its order.
fn add_checks(t: &Target, id: &str, private: bool, owner_id: Option<&str>, alias: &str) -> Result<(), String> {
    let channel = t.channel.as_str();
    match t.blocked(id, private, owner_id) {
        Some(Blocked::Already) => return Err(ALREADY.replace("{channel}", channel)),
        Some(Blocked::Private) => return Err(fail_line(&EditFail::Private, channel, alias)),
        Some(Blocked::Full) => return Err(fail_line(&EditFail::Full, channel, "")),
        None => {}
    }
    if t.taken(alias) {
        return Err(fail_line(&EditFail::NameTaken, channel, alias));
    }
    Ok(())
}

pub(super) async fn run(
    app: &AppHandle,
    place: &Place,
    name: &str,
    args: &str,
) -> Result<Option<UiAction>, String> {
    // V111-03 Read-only: no account, no 7TV sign-in, fine in automations.
    if name == "whoadded" {
        let (cmd, other) = parse(name, args)?;
        let Parsed::WhoAdded { name: wanted } = cmd else {
            return Err(USAGE_WHO.into());
        };
        return who_added(app, place, &wanted, other.as_deref()).await;
    }
    let account = place
        .account()
        .ok_or_else(|| format!("Sign in to use /{name}."))?;
    if place.automation {
        return Err(format!("/{name} doesn’t run in automations."));
    }
    let (cmd, other) = parse(name, args)?;
    // V111-03 `#` naming this chat's own channel is the same as none.
    let other = other.filter(|c| !c.eq_ignore_ascii_case(place.channel()));
    if matches!(cmd, Parsed::Steal { .. }) && other.is_none() {
        return Err(STEAL_HERE.into());
    }
    let ui = place.ui;
    let again = || format!("/{name} {}", args.trim());
    match edit::warm(app, account, false).await {
        Reachability::Ready => {}
        Reachability::NoConsent => return Err(why(&Why::NoConsent).into()),
        Reachability::SignedOut => return sign_in(ui, SelectError::SignedOut, again()),
        Reachability::SignInAgain => return sign_in(ui, SelectError::SignInAgain, again()),
        Reachability::NoAccount => return Err(why(&Why::NoAccount).into()),
        Reachability::Down | Reachability::Loading => return Err(why(&Why::Down).into()),
    }
    let here = place.channel();
    // V111-03 Where the change goes: `#channel`, except for /steal (there it's the source).
    let into = match (&cmd, &other) {
        (Parsed::Steal { .. }, _) | (_, None) => here,
        (_, Some(o)) => o.as_str(),
    };
    let t = if into == here {
        edit::here_target(app, account, here)
    } else {
        edit::target_for(app, account, into).await
    };
    let t = t.ok_or_else(|| fail_line(&EditFail::NotAllowed, into, ""))?;
    let channel = t.channel.as_str();
    let (op, alias) = match cmd {
        Parsed::Add { id, name } => {
            let facts = edit::emote_facts(app, here, &id)
                .await
                .map_err(down)?
                .ok_or(NO_EMOTE)?;
            let alias = name.unwrap_or(facts.name);
            add_checks(&t, &id, facts.private, facts.owner_id.as_deref(), &alias)?;
            let op = EditOp::Add {
                set_id: t.set_id.clone(),
                emote_id: id,
                alias: alias.clone(),
            };
            (op, alias)
        }
        Parsed::Remove { names } if names.len() > 1 => {
            return remove_many(app, place, account, &t, names, again()).await;
        }
        Parsed::Remove { mut names } => {
            let name = names.pop().unwrap_or_default();
            let (id, _) = entry(&t, &name)?;
            let op = EditOp::Remove {
                set_id: t.set_id.clone(),
                emote_id: id.clone(),
                alias: name.clone(),
            };
            (op, name)
        }
        Parsed::Rename { from, to } => {
            let (id, _) = entry(&t, &from)?;
            let to = match to {
                Some(to) => to,
                None => {
                    edit::emote_facts(app, into, id)
                        .await
                        .map_err(down)?
                        .ok_or(NO_EMOTE)?
                        .name
                }
            };
            if to == from {
                return Err(SAME.replace("{from}", &from));
            }
            if t.taken(&to) {
                return Err(fail_line(&EditFail::NameTaken, channel, &to));
            }
            let op = EditOp::Rename {
                set_id: t.set_id.clone(),
                emote_id: id.clone(),
                from,
                to: to.clone(),
            };
            (op, to)
        }
        Parsed::Steal { name: wanted, alias } => {
            let src = other.as_deref().unwrap_or_default();
            let emotes = edit::channel_emotes(app, src)
                .await
                .map_err(|f| source_line(f, src))?;
            let e = pick(&emotes, &wanted).map_err(|p| pick_line(p, src, &wanted))?;
            let alias = alias.unwrap_or_else(|| e.name.clone());
            add_checks(&t, &e.id, e.private, e.owner_id.as_deref(), &alias)?;
            let op = EditOp::Add {
                set_id: t.set_id.clone(),
                emote_id: e.id.clone(),
                alias: alias.clone(),
            };
            (op, alias)
        }
        Parsed::WhoAdded { .. } => return Err(USAGE_WHO.into()),
    };
    match edit::run(app, account, op.clone()).await {
        // C1 The chat line comes from `edit::run`; a chat view adds the Undo toast.
        Ok(done) if ui => Ok(Some(UiAction::SevenTvEdited { op, done })),
        Ok(_) => Ok(None),
        Err(EditFail::SignedOut) => sign_in(ui, SelectError::SignedOut, again()),
        Err(EditFail::SignInAgain) => sign_in(ui, SelectError::SignInAgain, again()),
        Err(e) => Err(fail_line(&e, channel, &alias)),
    }
}

/// V111-03 `/remove a b c`: one edit each, one line for all. A chat view gets one Undo for the
/// ones that went; what didn't is a line next to it.
async fn remove_many(
    app: &AppHandle,
    place: &Place,
    account: &str,
    t: &Target,
    names: Vec<String>,
    again: String,
) -> Result<Option<UiAction>, String> {
    let channel = t.channel.as_str();
    let (known, missing): (Vec<String>, Vec<String>) = names.into_iter().partition(|n| t.taken(n));
    let missing_line = match missing.as_slice() {
        [] => None,
        [one] => Some(NO_SUCH.replace("{channel}", channel).replace("{name}", one)),
        many => Some(format!("{channel} has no 7TV emotes called {}.", joined(many, "or"))),
    };
    if known.is_empty() {
        return Err(missing_line.unwrap_or_else(|| USAGE_REMOVE.into()));
    }
    let mut dones: Vec<EditDone> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    // (reason, names), in the order they came.
    let mut failed: Vec<(String, Vec<String>)> = Vec::new();
    let mut stopped: Option<EditFail> = None;
    for name in known {
        if let Some(e) = &stopped {
            let line = fail_line(e, channel, &name);
            add_fail(&mut failed, line, name);
            continue;
        }
        let Ok((id, _)) = entry(t, &name).cloned() else { continue };
        let op = EditOp::Remove {
            set_id: t.set_id.clone(),
            emote_id: id,
            alias: name.clone(),
        };
        match edit::run(app, account, op).await {
            Ok(done) => {
                dones.push(done);
                removed.push(name);
            }
            // Nothing went yet: the sign-in pop-up, then all of it again.
            Err(e @ (EditFail::SignedOut | EditFail::SignInAgain)) if removed.is_empty() => {
                let why = if e == EditFail::SignedOut { SelectError::SignedOut } else { SelectError::SignInAgain };
                return sign_in(place.ui, why, again);
            }
            Err(e) => {
                add_fail(&mut failed, fail_line(&e, channel, &name), name);
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
        Some(line) if place.ui => {
            if !rest.is_empty() {
                local(app, &place.key, NoticeKind::Error, &rest);
            }
            Ok(Some(UiAction::SevenTvRemoved { line, dones }))
        }
        Some(line) => {
            let (kind, text) = if rest.is_empty() {
                (NoticeKind::Info, line)
            } else {
                (NoticeKind::Error, format!("{line} {rest}"))
            };
            local(app, &place.key, kind, &text);
            Ok(None)
        }
    }
}

fn add_fail(failed: &mut Vec<(String, Vec<String>)>, reason: String, name: String) {
    match failed.iter_mut().find(|(r, _)| *r == reason) {
        Some((_, names)) => names.push(name),
        None => failed.push((reason, vec![name])),
    }
}

/// V111-03 `/whoadded name [#channel]`: who added it and when, from the set's data.
async fn who_added(app: &AppHandle, place: &Place, wanted: &str, other: Option<&str>) -> Result<Option<UiAction>, String> {
    let other = other.filter(|c| !c.eq_ignore_ascii_case(place.channel()));
    let login = other.unwrap_or(place.channel());
    let emotes = edit::channel_emotes(app, login)
        .await
        .map_err(|f| source_line(f, login))?;
    let e = pick(&emotes, wanted).map_err(|p| pick_line(p, login, wanted))?;
    let who = match &e.added_by {
        Some(id) => edit::user_names(app, std::slice::from_ref(id)).await.remove(id),
        None => None,
    };
    let when = e.added_at.as_deref().map(crate::bell_dates::local_dates);
    let text = who_line(&e.name, other, who.as_deref(), when.as_deref());
    local(app, &place.key, NoticeKind::Info, &text);
    Ok(None)
}

/// V111-03 The `/whoadded` line. `channel`: named only when it isn't this chat's.
fn who_line(name: &str, channel: Option<&str>, who: Option<&str>, when: Option<&str>) -> String {
    let to = channel.map(|c| format!(" to {c}")).unwrap_or_default();
    match (who, when) {
        (Some(w), Some(d)) => format!("{name} was added{to} by {w} on {d}."),
        (Some(w), None) => format!("{name} was added{to} by {w}."),
        (None, Some(d)) => format!("{name} was added{to} on {d}. 7TV doesn’t say by whom."),
        (None, None) => format!("7TV doesn’t say who added {name}."),
    }
}

#[cfg(test)]
mod tests {
    use cyex_chat::model::Provider;

    use super::*;

    const ID: &str = "01HZX5V4Q9000A1B2C3D4E5F6G";

    fn add(name: Option<&str>) -> Parsed {
        Parsed::Add {
            id: ID.into(),
            name: name.map(str::to_owned),
        }
    }

    fn here(p: Parsed) -> Result<(Parsed, Option<String>), String> {
        Ok((p, None))
    }

    fn there(p: Parsed, c: &str) -> Result<(Parsed, Option<String>), String> {
        Ok((p, Some(c.to_owned())))
    }

    fn remove(names: &[&str]) -> Parsed {
        Parsed::Remove {
            names: names.iter().map(|n| (*n).to_owned()).collect(),
        }
    }

    #[test]
    fn parse_table() {
        assert_eq!(parse("add", ""), Err(USAGE_ADD.into()));
        assert_eq!(parse("add", "  "), Err(USAGE_ADD.into()));
        assert_eq!(parse("add", ID), here(add(None)));
        assert_eq!(parse("add", &format!("  {ID}   happy ")), here(add(Some("happy"))));
        assert_eq!(parse("add", &format!("{ID} a b")), Err(SPACES.into()));
        assert_eq!(parse("add", "notalink"), Err(NOT_LINK.into()));
        assert_eq!(parse("remove", ""), Err(USAGE_REMOVE.into()));
        assert_eq!(parse("remove", "happy"), here(remove(&["happy"])));
        assert_eq!(parse("rename", ""), Err(USAGE_RENAME.into()));
        assert_eq!(
            parse("rename", "happy"),
            here(Parsed::Rename {
                from: "happy".into(),
                to: None
            })
        );
        assert_eq!(
            parse("rename", "happy sad"),
            here(Parsed::Rename {
                from: "happy".into(),
                to: Some("sad".into())
            })
        );
        assert_eq!(parse("rename", "a b c"), Err(SPACES.into()));
    }

    /// V111-03
    #[test]
    fn channel_word_anywhere() {
        assert_eq!(parse("add", &format!("{ID} happy #Forsen")), there(add(Some("happy")), "forsen"));
        assert_eq!(parse("add", &format!("#forsen {ID}")), there(add(None), "forsen"));
        assert_eq!(parse("add", &format!("{ID} #x_y_1 happy")), there(add(Some("happy")), "x_y_1"));
        assert_eq!(parse("add", "#forsen"), Err(USAGE_ADD.into()));
        assert_eq!(parse("add", &format!("{ID} #a #b")), Err(ONE_CHANNEL.into()));
        assert_eq!(parse("add", &format!("{ID} #")), Err("# isn’t a channel name.".into()));
        assert_eq!(parse("add", &format!("{ID} #not-a-name")), Err("#not-a-name isn’t a channel name.".into()));
        assert_eq!(
            parse("add", &format!("{ID} #{}", "a".repeat(26))),
            Err(format!("#{} isn’t a channel name.", "a".repeat(26)))
        );
        assert_eq!(
            parse("rename", "happy sad #forsen"),
            there(
                Parsed::Rename {
                    from: "happy".into(),
                    to: Some("sad".into())
                },
                "forsen"
            )
        );
        assert_eq!(parse("rename", "happy sad x #forsen"), Err(SPACES.into()));
    }

    /// V111-03
    #[test]
    fn remove_takes_several() {
        assert_eq!(parse("remove", "a b c"), here(remove(&["a", "b", "c"])));
        assert_eq!(parse("remove", "a #forsen b a"), there(remove(&["a", "b"]), "forsen"));
        let twenty: Vec<String> = (0..20).map(|n| format!("e{n}")).collect();
        assert!(parse("remove", &twenty.join(" ")).is_ok());
        assert_eq!(parse("remove", &format!("{} x", twenty.join(" "))), Err(TOO_MANY.into()));
        assert_eq!(parse("remove", "#forsen"), Err(USAGE_REMOVE.into()));
    }

    /// V111-03
    #[test]
    fn steal_and_whoadded() {
        let steal = |name: &str, alias: Option<&str>| Parsed::Steal {
            name: name.into(),
            alias: alias.map(str::to_owned),
        };
        assert_eq!(parse("steal", "KEKW #forsen"), there(steal("KEKW", None), "forsen"));
        assert_eq!(parse("yoink", "#Forsen KEKW lol"), there(steal("KEKW", Some("lol")), "forsen"));
        assert_eq!(parse("steal", "KEKW"), Err(USAGE_STEAL.into()));
        assert_eq!(parse("steal", "#forsen"), Err(USAGE_STEAL.into()));
        assert_eq!(parse("steal", "a b c #forsen"), Err(SPACES.into()));
        let who = |n: &str| Parsed::WhoAdded { name: n.into() };
        assert_eq!(parse("whoadded", "KEKW"), here(who("KEKW")));
        assert_eq!(parse("whoadded", "KEKW #forsen"), there(who("KEKW"), "forsen"));
        assert_eq!(parse("whoadded", ""), Err(USAGE_WHO.into()));
        assert_eq!(parse("whoadded", "a b"), Err(SPACES.into()));
    }

    /// V111-03
    #[test]
    fn steal_picks_exact_then_one_up_to_case() {
        let set = |names: &[&str]| -> Vec<Emote> {
            names
                .iter()
                .enumerate()
                .map(|(i, n)| Emote::new(Provider::Seventv, format!("id{i}"), *n))
                .collect()
        };
        let s = set(&["KEKW", "kekw", "Pog"]);
        assert_eq!(pick(&s, "KEKW").map(|e| e.id.as_str()), Ok("id0"));
        assert_eq!(pick(&s, "kekw").map(|e| e.id.as_str()), Ok("id1"));
        assert_eq!(pick(&s, "pog").map(|e| e.id.as_str()), Ok("id2"));
        assert_eq!(
            pick(&s, "Kekw").map(|e| e.id.as_str()),
            Err(Pick::Ambiguous(vec!["KEKW".into(), "kekw".into()]))
        );
        assert_eq!(pick(&s, "nope").map(|e| e.id.as_str()), Err(Pick::Missing));
        assert_eq!(
            pick_line(Pick::Ambiguous(vec!["KEKW".into(), "kekw".into()]), "forsen", "Kekw"),
            "forsen has more than one: KEKW, kekw. Type it exactly."
        );
        assert_eq!(pick_line(Pick::Missing, "forsen", "x"), "forsen has no 7TV emote called x.");
    }

    /// V111-03
    #[test]
    fn lines() {
        let names = |n: &[&str]| n.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(joined(&names(&["a"]), "and"), "a");
        assert_eq!(joined(&names(&["a", "b"]), "and"), "a and b");
        assert_eq!(joined(&names(&["a", "b", "c"]), "or"), "a, b or c");
        assert_eq!(
            who_line("KEKW", None, Some("cyexbot"), Some("2026/11/14 13:05")),
            "KEKW was added by cyexbot on 2026/11/14 13:05."
        );
        assert_eq!(
            who_line("KEKW", Some("forsen"), None, Some("2026/11/14 13:05")),
            "KEKW was added to forsen on 2026/11/14 13:05. 7TV doesn’t say by whom."
        );
        assert_eq!(who_line("KEKW", None, None, None), "7TV doesn’t say who added KEKW.");
        assert_eq!(source_line(SourceFail::NoSet, "forsen"), "forsen has no 7TV emotes.");
        assert_eq!(source_line(SourceFail::NoChannel, "nobody"), "No Twitch channel called nobody.");
        let mut failed = Vec::new();
        add_fail(&mut failed, "r1".into(), "a".into());
        add_fail(&mut failed, "r2".into(), "b".into());
        add_fail(&mut failed, "r1".into(), "c".into());
        assert_eq!(failed, vec![("r1".into(), names(&["a", "c"])), ("r2".into(), names(&["b"]))]);
    }

    #[test]
    fn emote_ids() {
        let some = Some(ID.to_owned());
        for w in [
            format!("https://7tv.app/emotes/{ID}"),
            format!("http://7tv.app/emotes/{ID}"),
            format!("https://www.7tv.app/emotes/{ID}"),
            format!("https://old.7tv.app/emotes/{ID}"),
            format!("7tv.app/emotes/{ID}"),
            format!("HTTPS://7TV.APP/emotes/{ID}"),
            format!("https://7tv.app/emotes/{ID}?x=1"),
            format!("https://7tv.app/emotes/{ID}/"),
            format!("https://7tv.app/emotes/{ID}#top"),
            ID.to_lowercase(),
            ID.to_owned(),
        ] {
            assert_eq!(emote_id(&w), some, "{w}");
        }
        for w in [
            format!("https://evil.app/7tv.app/emotes/{ID}"),
            format!("ftp://7tv.app/emotes/{ID}"),
            ID[..25].to_owned(),
            format!("I{}", &ID[1..]),
            format!("L{}", &ID[1..]),
            format!("O{}", &ID[1..]),
            format!("U{}", &ID[1..]),
            "60ae958e229664e8667aea38".to_owned(),
            String::new(),
        ] {
            assert_eq!(emote_id(&w), None, "{w}");
        }
    }

    #[test]
    fn sign_in_asks_in_the_chat_view_only() {
        let ask = sign_in(true, SelectError::SignedOut, "/add x".into()).unwrap();
        assert!(
            matches!(ask, Some(UiAction::SevenTvSignIn { ref command, again: false }) if command == "/add x")
        );
        let again = sign_in(true, SelectError::SignInAgain, "/add x".into()).unwrap();
        assert!(matches!(again, Some(UiAction::SevenTvSignIn { again: true, .. })));
        assert_eq!(
            sign_in(false, SelectError::SignedOut, "/add x".into()).unwrap_err(),
            "Sign in to 7TV first: /cosmetics."
        );
    }

    /// The UI's `failText` and this file say the same.
    #[test]
    fn fail_lines_match_the_ui_words() {
        const WORDS_TS: &str = include_str!("../../../../ui/src/features/emotes/edit/words.ts");
        for e in [
            EditFail::NotAllowed,
            EditFail::Full,
            EditFail::NameTaken,
            EditFail::Private,
            EditFail::NotFound,
            EditFail::BadName,
            EditFail::Down,
            EditFail::TestRun,
            EditFail::Bad,
        ] {
            let line = fail_line(&e, "${channel}", "${alias}");
            assert!(WORDS_TS.contains(&line), "{e:?}: {line}");
        }
    }

    #[test]
    fn fail_lines_for_7tv_stops() {
        assert_eq!(
            fail_line(&EditFail::RateLimited { seconds: 12 }, "c", "a"),
            "7TV needs a short break. Try again in 12 s."
        );
        assert_eq!(
            fail_line(&EditFail::NoConsent, "c", "a"),
            "Allow 7TV first: /cosmetics."
        );
        assert_eq!(
            fail_line(&EditFail::NameTaken, "cyexbot", "happy"),
            "cyexbot already has an emote named happy."
        );
    }
}
