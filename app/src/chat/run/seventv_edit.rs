//! I106-3 `/add`, `/remove`, `/rename`: change the 7TV emotes of the channel this chat is in,
//! through the same `edit::run` as the menus (consent, session, test-run guard, name check,
//! one edit per set). Results are local lines or a toast; nothing goes to the channel.

use tauri::AppHandle;

use super::{Place, UiAction};
use crate::cosmetics::{SelectError, Why, select_error, why};
use crate::emotes::edit::{self, Blocked, EditFail, EditOp, Reachability, Target};

/// The arm in `builtin::run` and `OWN` must list these (checked in builtin's tests).
#[cfg(test)]
pub(super) const NAMES: [&str; 3] = ["add", "remove", "rename"];

const USAGE_ADD: &str = "Say which emote: /add <7TV link or id> [name].";
const USAGE_REMOVE: &str = "Say which emote: /remove <name>.";
const USAGE_RENAME: &str = "Say which emote: /rename <name> [new name].";
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
    Remove {
        name: String,
    },
    /// `to: None` goes back to the emote's own 7TV name (C2).
    Rename {
        from: String,
        to: Option<String>,
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

/// Words are split on white space: emote names have no spaces, so there are no quotes.
fn parse(command: &str, args: &str) -> Result<Parsed, String> {
    let words: Vec<&str> = args.split_whitespace().collect();
    let usage = match command {
        "add" => USAGE_ADD,
        "remove" => USAGE_REMOVE,
        _ => USAGE_RENAME,
    };
    let max = if command == "remove" { 1 } else { 2 };
    if words.is_empty() {
        return Err(usage.into());
    }
    if words.len() > max {
        return Err(SPACES.into());
    }
    let second = words.get(1).map(|w| (*w).to_owned());
    Ok(match command {
        "add" => Parsed::Add {
            id: emote_id(words[0]).ok_or(NOT_LINK)?,
            name: second,
        },
        "remove" => Parsed::Remove {
            name: words[0].to_owned(),
        },
        _ => Parsed::Rename {
            from: words[0].to_owned(),
            to: second,
        },
    })
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

pub(super) async fn run(
    app: &AppHandle,
    place: &Place,
    name: &str,
    args: &str,
) -> Result<Option<UiAction>, String> {
    let account = place
        .account()
        .ok_or_else(|| format!("Sign in to use /{name}."))?;
    if place.automation {
        return Err(format!("/{name} doesn’t run in automations."));
    }
    let cmd = parse(name, args)?;
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
    let login = place.channel();
    let t =
        edit::here_target(app, account, login).ok_or_else(|| fail_line(&EditFail::NotAllowed, login, ""))?;
    let channel = t.channel.as_str();
    let (op, alias) = match cmd {
        Parsed::Add { id, name } => {
            let facts = edit::emote_facts(app, login, &id)
                .await
                .map_err(down)?
                .ok_or(NO_EMOTE)?;
            let alias = name.unwrap_or(facts.name);
            // R14 The menu's checks, in its order.
            match t.blocked(&id, facts.private, facts.owner_id.as_deref()) {
                Some(Blocked::Already) => return Err(ALREADY.replace("{channel}", channel)),
                Some(Blocked::Private) => return Err(fail_line(&EditFail::Private, channel, &alias)),
                Some(Blocked::Full) => return Err(fail_line(&EditFail::Full, channel, "")),
                None => {}
            }
            if t.taken(&alias) {
                return Err(fail_line(&EditFail::NameTaken, channel, &alias));
            }
            let op = EditOp::Add {
                set_id: t.set_id.clone(),
                emote_id: id,
                alias: alias.clone(),
            };
            (op, alias)
        }
        Parsed::Remove { name } => {
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
                    edit::emote_facts(app, login, id)
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

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "01HZX5V4Q9000A1B2C3D4E5F6G";

    fn add(name: Option<&str>) -> Parsed {
        Parsed::Add {
            id: ID.into(),
            name: name.map(str::to_owned),
        }
    }

    #[test]
    fn parse_table() {
        assert_eq!(parse("add", ""), Err(USAGE_ADD.into()));
        assert_eq!(parse("add", "  "), Err(USAGE_ADD.into()));
        assert_eq!(parse("add", ID), Ok(add(None)));
        assert_eq!(parse("add", &format!("  {ID}   happy ")), Ok(add(Some("happy"))));
        assert_eq!(parse("add", &format!("{ID} a b")), Err(SPACES.into()));
        assert_eq!(parse("add", "notalink"), Err(NOT_LINK.into()));
        assert_eq!(parse("remove", ""), Err(USAGE_REMOVE.into()));
        assert_eq!(
            parse("remove", "happy"),
            Ok(Parsed::Remove { name: "happy".into() })
        );
        assert_eq!(parse("remove", "a b"), Err(SPACES.into()));
        assert_eq!(parse("rename", ""), Err(USAGE_RENAME.into()));
        assert_eq!(
            parse("rename", "happy"),
            Ok(Parsed::Rename {
                from: "happy".into(),
                to: None
            })
        );
        assert_eq!(
            parse("rename", "happy sad"),
            Ok(Parsed::Rename {
                from: "happy".into(),
                to: Some("sad".into())
            })
        );
        assert_eq!(parse("rename", "a b c"), Err(SPACES.into()));
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
