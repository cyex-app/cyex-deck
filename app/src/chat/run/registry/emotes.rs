//! I106-3 The 7TV emote commands: `/add`, `/remove`, `/rename`.

use super::Changes::SevenTv;
use super::{Cmd, ex, opt, req};

const NOTES: &str = "For channels you own or edit on 7TV. Each change shows a line in that channel’s chat, like 7TV’s own; in the chat you typed it in, Undo shows for a few seconds too. Not in automations. Signed out of 7TV, the chat asks you to sign in, then runs it again. Nothing goes to the channel. Your own command with the same name runs instead.";

pub(super) const CMDS: &[Cmd] = &[
    Cmd::new(
        "add",
        "/add link [name]",
        "Adds a 7TV emote to this chat’s channel, by its 7tv.app link or id.",
        "I106-3",
    )
    .help("/add")
    .args(&[
        req("link", "word").note("A 7tv.app/emotes link or the emote’s id."),
        opt("name", "word").note("Its name here. Leave it out for the emote’s own name."),
    ])
    .changes(SevenTv, "Changes this channel’s 7TV emotes.")
    .ex(&[
        ex(
            "/add https://7tv.app/emotes/01HZX5V4Q9000A1B2C3D4E5F6G",
            "cyexbot added the 7TV emote peepoHappy.",
        ),
        ex(
            "/add 01HZX5V4Q9000A1B2C3D4E5F6G happy",
            "cyexbot added the 7TV emote happy.",
        ),
    ])
    .errs(&[
        "Sign in to use /add.",
        "/add doesn’t run in automations.",
        "Say which emote: /add <7TV link or id> [name].",
        "Emote names have no spaces.",
        "That isn’t a 7TV emote link or id.",
        "No 7TV emote with that id.",
        "Already in {channel}’s 7TV emotes.",
        "You can’t edit {channel}’s 7TV emotes.",
        "{channel}’s 7TV emotes are full.",
        "{channel} already has an emote named {alias}.",
        "That emote is private. Only its owner can add it.",
        "That emote isn’t there anymore.",
        "7TV doesn’t allow that name.",
        "Couldn’t reach 7TV. Nothing changed.",
        "7TV edits only go to the test mock here.",
        "Something went wrong at 7TV. Nothing changed.",
        "Allow 7TV first: /cosmetics.",
        "Sign in to 7TV first: /cosmetics.",
        "Your 7TV sign-in ran out. /cosmetics signs you in again.",
        "This account has no 7TV account yet. Sign in once on 7tv.app.",
        "Couldn’t reach 7TV. Try again in a minute.",
        "7TV needs a short break. Try again in {seconds} s.",
        "7TV needs a short break. Try again in {minutes} min.",
    ])
    .notes(NOTES),
    Cmd::new(
        "remove",
        "/remove name",
        "Removes a 7TV emote from this chat’s channel.",
        "I106-3",
    )
    .help("/remove")
    .args(&[req("name", "word").note("Its name here. Tab completes them.")])
    .changes(SevenTv, "Changes this channel’s 7TV emotes.")
    .ex(&[ex("/remove happy", "cyexbot removed the 7TV emote happy.")])
    .errs(&[
        "Sign in to use /remove.",
        "/remove doesn’t run in automations.",
        "Say which emote: /remove <name>.",
        "Emote names have no spaces.",
        "{channel} has no 7TV emote called {name}.",
        "You can’t edit {channel}’s 7TV emotes.",
        "That emote isn’t there anymore.",
        "7TV doesn’t allow that name.",
        "Couldn’t reach 7TV. Nothing changed.",
        "7TV edits only go to the test mock here.",
        "Something went wrong at 7TV. Nothing changed.",
        "Allow 7TV first: /cosmetics.",
        "Sign in to 7TV first: /cosmetics.",
        "Your 7TV sign-in ran out. /cosmetics signs you in again.",
        "This account has no 7TV account yet. Sign in once on 7tv.app.",
        "Couldn’t reach 7TV. Try again in a minute.",
        "7TV needs a short break. Try again in {seconds} s.",
        "7TV needs a short break. Try again in {minutes} min.",
    ])
    .notes(NOTES),
    Cmd::new(
        "rename",
        "/rename name [new]",
        "Renames a 7TV emote in this chat’s channel.",
        "I106-3",
    )
    .help("/rename")
    .args(&[
        req("name", "word").note("Its name here. Tab completes them."),
        opt("new", "word").note("Leave it out to go back to the emote’s own name."),
    ])
    .changes(SevenTv, "Changes this channel’s 7TV emotes.")
    .ex(&[
        ex(
            "/rename peepoHappy happy",
            "cyexbot renamed the 7TV emote peepoHappy to happy.",
        ),
        ex(
            "/rename happy",
            "cyexbot renamed the 7TV emote happy to peepoHappy.",
        ),
    ])
    .errs(&[
        "Sign in to use /rename.",
        "/rename doesn’t run in automations.",
        "Say which emote: /rename <name> [new name].",
        "Emote names have no spaces.",
        "{channel} has no 7TV emote called {name}.",
        "No 7TV emote with that id.",
        "It’s called {from} already.",
        "You can’t edit {channel}’s 7TV emotes.",
        "{channel} already has an emote named {alias}.",
        "That emote isn’t there anymore.",
        "7TV doesn’t allow that name.",
        "Couldn’t reach 7TV. Nothing changed.",
        "7TV edits only go to the test mock here.",
        "Something went wrong at 7TV. Nothing changed.",
        "Allow 7TV first: /cosmetics.",
        "Sign in to 7TV first: /cosmetics.",
        "Your 7TV sign-in ran out. /cosmetics signs you in again.",
        "This account has no 7TV account yet. Sign in once on 7tv.app.",
        "Couldn’t reach 7TV. Try again in a minute.",
        "7TV needs a short break. Try again in {seconds} s.",
        "7TV needs a short break. Try again in {minutes} min.",
    ])
    .notes(NOTES),
];
