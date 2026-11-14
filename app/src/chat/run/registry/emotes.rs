//! I106-3 The 7TV emote commands: `/add`, `/remove`, `/rename`. V111-03 `#channel` on each,
//! several names for `/remove`, `/steal` (`/yoink`) and `/whoadded`.

use super::Changes::SevenTv;
use super::{Cmd, ex, opt, req};

const NOTES: &str = "For channels you own or edit on 7TV. Add #channel to change that channel’s emotes instead of this one’s; Undo still shows here. Each change shows a line in that channel’s chat, like 7TV’s own; in the chat you typed it in, Undo shows for a few seconds too. Not in automations. Signed out of 7TV, the chat asks you to sign in, then runs it again. Nothing goes to the channel. Your own command with the same name runs instead.";

pub(super) const CMDS: &[Cmd] = &[
    Cmd::new(
        "add",
        "/add link [name] [#channel]",
        "Adds a 7TV emote to this chat’s channel, by its 7tv.app link or id.",
        "I106-3",
    )
    .help("/add")
    .args(&[
        req("link", "word").note("A 7tv.app/emotes link or the emote’s id."),
        opt("name", "word").note("Its name here. Leave it out for the emote’s own name."),
        opt("#channel", "channel").note("Another channel whose 7TV emotes you edit. Tab after # completes it."),
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
        ex(
            "/add 01HZX5V4Q9000A1B2C3D4E5F6G #forsen",
            "Added to forsen’s 7TV emotes; the line shows in forsen’s chat.",
        ),
    ])
    .errs(&[
        "Sign in to use /add.",
        "/add doesn’t run in automations.",
        "Say which emote: /add <7TV link or id> [name] [#channel].",
        "{word} isn’t a channel name.",
        "Name one channel with #.",
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
        "/remove name [more names] [#channel]",
        "Removes 7TV emotes from this chat’s channel, up to 20 at a time.",
        "I106-3",
    )
    .help("/remove")
    .args(&[
        req("name", "word").note("Its name here. Tab completes them."),
        opt("more names", "word").note("More to remove at once, with spaces between."),
        opt("#channel", "channel").note("Another channel whose 7TV emotes you edit. Tab after # completes it."),
    ])
    .changes(SevenTv, "Changes this channel’s 7TV emotes.")
    .ex(&[
        ex("/remove happy", "cyexbot removed the 7TV emote happy."),
        ex("/remove happy sad", "Removed happy and sad."),
        ex("/remove happy #forsen", "Removed from forsen’s 7TV emotes; the line shows in forsen’s chat."),
    ])
    .errs(&[
        "Sign in to use /remove.",
        "/remove doesn’t run in automations.",
        "Say which emote: /remove <name> [more names] [#channel].",
        "{word} isn’t a channel name.",
        "Name one channel with #.",
        "At most 20 at a time.",
        "{channel} has no 7TV emotes called {names}.",
        "Not removed: {names}. {reason}",
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
        "/rename name [new] [#channel]",
        "Renames a 7TV emote in this chat’s channel.",
        "I106-3",
    )
    .help("/rename")
    .args(&[
        req("name", "word").note("Its name here. Tab completes them."),
        opt("new", "word").note("Leave it out to go back to the emote’s own name."),
        opt("#channel", "channel").note("Another channel whose 7TV emotes you edit. Tab after # completes it."),
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
        "Say which emote: /rename <name> [new name] [#channel].",
        "{word} isn’t a channel name.",
        "Name one channel with #.",
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
    // V111-03
    Cmd::new(
        "steal",
        "/steal name [new] #channel",
        "Takes a 7TV emote from another channel and adds it to this chat’s channel.",
        "V111-03",
    )
    .alias(&["yoink"])
    .help("/steal")
    .args(&[
        req("name", "word").note("Its name in that channel. Case doesn’t matter when only one fits. Tab completes them when that chat is open."),
        opt("new", "word").note("Its name here. Leave it out to keep the name it has there."),
        req("#channel", "channel").note("Where it’s from. Tab after # completes it."),
    ])
    .changes(SevenTv, "Changes this channel’s 7TV emotes.")
    .ex(&[
        ex("/steal peepoHappy #forsen", "cyexbot added the 7TV emote peepoHappy."),
        ex("/yoink peepohappy happy #forsen", "cyexbot added the 7TV emote happy."),
    ])
    .errs(&[
        "Sign in to use /steal.",
        "/steal doesn’t run in automations.",
        "Say which emote and where from: /steal <name> [new name] #channel.",
        "That’s this chat’s channel. Say where from: /steal <name> #channel.",
        "{word} isn’t a channel name.",
        "Name one channel with #.",
        "Emote names have no spaces.",
        "No Twitch channel called {channel}.",
        "{channel} has no 7TV emotes.",
        "{channel} has no 7TV emote called {name}.",
        "{channel} has more than one: {names}. Type it exactly.",
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
    // V111-03
    Cmd::new(
        "whoadded",
        "/whoadded name [#channel]",
        "Says who added a 7TV emote to this chat’s channel, and when.",
        "V111-03",
    )
    .help("/whoadded")
    .args(&[
        req("name", "word").note("Its name in the channel. Tab completes them."),
        opt("#channel", "channel").note("Another channel. Tab after # completes it."),
    ])
    .ex(&[
        ex("/whoadded peepoHappy", "peepoHappy was added by cyexbot on 2026/11/14 13:05."),
        ex("/whoadded KEKW #forsen", "KEKW was added to forsen by forsen on 2024/03/02 21:40."),
    ])
    .errs(&[
        "Say which emote: /whoadded <name> [#channel].",
        "{word} isn’t a channel name.",
        "Name one channel with #.",
        "Emote names have no spaces.",
        "No Twitch channel called {channel}.",
        "{channel} has no 7TV emotes.",
        "{channel} has no 7TV emote called {name}.",
        "{channel} has more than one: {names}. Type it exactly.",
        "Couldn’t reach 7TV. Try again in a minute.",
        "7TV doesn’t say who added {name}.",
    ])
    .notes("Anyone can use it, signed in or not, also in automations. Reads 7TV’s set; nothing changes."),
];
