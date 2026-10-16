//! The 7TV commands: `/cosmetics` (V1051-03) and `/combo`, `/paint`, `/badge` (V1051-04).

use super::Changes::{SevenTv, Window};
use super::{Cmd, ex, opt, req};

#[cfg(test)]
const SEVEN_TV: &[&str] = &[
    "Allow 7TV first: /cosmetics.",
    "Sign in to 7TV first: /cosmetics.",
    "Your 7TV sign-in ran out. /cosmetics signs you in again.",
    "7TV didn’t allow that one.",
    "Couldn’t reach 7TV. Nothing changed.",
    "7TV needs a short break. Try again in {seconds} s.",
    "7TV needs a short break. Try again in {minutes} min.",
    "This account has no 7TV account yet. Sign in once on 7tv.app.",
    "Couldn’t reach 7TV. Try again in a minute.",
];

pub(super) const CMDS: &[Cmd] = &[
    Cmd::new("cosmetics", "/cosmetics", "Opens the 7TV paint and badge pop-up for this chat’s account.", "V1051-03")
        .help("/cosmetics")
        .changes(Window, "Opens the pop-up; picking there changes the account’s 7TV cosmetics.")
        .ex(&[
            ex("/cosmetics", "(the pop-up opens)"),
            ex("/cosmetics", "Sign in to pick a 7TV paint and badge."),
        ])
        .notes("Without an account the second line shows instead of the pop-up."),
    Cmd::new(
        "combo",
        "/combo name | list | save name | delete name | rename old new",
        "Wears a saved paint and badge pair, or lists, saves, deletes and renames your combos.",
        "V1051-04",
    )
    .help("/combo")
    .args(&[
        req("name", "combo").note("The combo to wear. Quote names with spaces: \"night owl\" or “night owl”."),
        opt("action", "word")
            .values(&["list", "save", "delete", "rename"])
            .note("A combo can’t use these words as its name."),
    ])
    .changes(SevenTv, "Applying changes the account’s 7TV paint and badge; save, delete and rename change your combos list only.")
    .ex(&[
        ex("/combo night owl", "Wearing night owl"),
        ex("/combo save gold set", "Saved “gold set”: Gold and Founder."),
        ex("/combo list", "Your combos: gold set, “night owl”."),
        ex("/combo rename \"night owl\" late", "Renamed “night owl” to “late”."),
        ex("/combo delete late", "Deleted “late”."),
    ])
    .errs(&[
        "Sign in to use /combo.",
        "/combo doesn’t run in automations.",
        "Say which combo: /combo <name>. /combo list shows yours.",
        "Say a name: /combo {save|delete} <name>.",
        "Put names with spaces in quotes: /combo rename \"old name\" \"new name\".",
        "A quote isn’t closed.",
        "No combo called “{name}”. /combo list shows yours.",
        "Give it a name.",
        "Names can be up to 32 characters.",
        "Names can have letters, numbers, spaces, ’, - and _.",
        "“{word}” is part of /combo. Pick another name.",
        "You have a combo called “{name}” already.",
        "You have 50 combos. Delete one first.",
        "That combo is gone.",
        "Couldn’t save your combos. Try again.",
    ])
    .notes("Results are local lines; nothing goes to the channel. Needs a signed-in chat; wearing and saving also need 7TV allowed. A ⚠ after a name in /combo list means a part isn’t usable. Automations can only use /combo list. Tab completes combo names."),
    Cmd::new("paint", "/paint name | none", "Wears one of your 7TV paints, or none.", "V1051-04")
        .help("/paint")
        .args(&[req("name", "paint").values(&["none"]).note("The rest of the line; no quotes needed. \"None\" in quotes means a paint called None.")])
        .changes(SevenTv, "Changes the account’s 7TV paint; the badge stays.")
        .ex(&[ex("/paint Forest Spring", "Paint: Forest Spring."), ex("/paint none", "No paint now.")])
        .errs(&[
            "Sign in to use /paint.",
            "/paint doesn’t run in automations.",
            "Say which paint, or none: /paint <name>.",
            "A quote isn’t closed.",
            "You don’t have a paint called “{name}”. Tab shows yours.",
            "You don’t own that paint. Tab shows yours.",
            "That one’s for 7TV subscribers.",
            "7TV says that one isn’t yours.",
        ])
        .notes("Not in automations. Tab completes your paints."),
    Cmd::new("badge", "/badge name | none", "Wears one of your 7TV badges, or none.", "V1051-04")
        .help("/badge")
        .args(&[req("name", "badge").values(&["none"]).note("The rest of the line; no quotes needed. \"None\" in quotes means a badge called None.")])
        .changes(SevenTv, "Changes the account’s 7TV badge; the paint stays.")
        .ex(&[ex("/badge Founder", "Badge: Founder."), ex("/badge none", "No badge now.")])
        .errs(&[
            "Sign in to use /badge.",
            "/badge doesn’t run in automations.",
            "Say which badge, or none: /badge <name>.",
            "A quote isn’t closed.",
            "You don’t have a badge called “{name}”. Tab shows yours.",
            "You don’t own that badge. Tab shows yours.",
            "That one’s for 7TV subscribers.",
            "7TV says that one isn’t yours.",
        ])
        .notes("Not in automations. Tab completes your badges."),
];

/// The lines every 7TV command can end in; the render adds them to `/combo`, `/paint` and `/badge`.
#[cfg(test)]
pub(super) const SHARED: &[&str] = SEVEN_TV;
