//! Twitch’s mod and broadcaster commands, read by `cyex_moderation::command` (1.49, 1.50, M3).
//! Syntax and errors follow `crates/moderation/src/command.rs`.

use super::Changes::Twitch;
use super::{Arg, Cmd, Example, ex, opt, req};

const NAME: Arg = req("name", "login").note("@Name, name or #name.");
const USAGE_NAME: &str = "Usage: /{name} name";

/// A Twitch command: the account must be signed in and have the right role (see `works.roles`).
const fn tw(
    name: &'static str,
    syntax: &'static str,
    summary: &'static str,
    spec: &'static str,
    e: &'static [Example],
) -> Cmd {
    Cmd::new(name, syntax, summary, spec)
        .changes(Twitch, "Runs on Twitch for the chat’s account.")
        .ex(e)
}

pub(super) const CMDS: &[Cmd] = &[
    tw(
        "ban",
        "/ban name [reason]",
        "Bans someone from the channel.",
        "1.49",
        &[ex("/ban spammer buying followers", "")],
    )
    .help("/ban")
    .args(&[NAME, opt("reason", "text")])
    .errs(&["Usage: /ban name"]),
    tw(
        "timeout",
        "/timeout name [10m] [reason]",
        "Times someone out. Default 10 minutes.",
        "1.49",
        &[ex("/timeout someone 1h30m caps", "")],
    )
    .help("/timeout")
    .args(&[
        NAME,
        opt("length", "duration").note("1 second to 2 weeks."),
        opt("reason", "text"),
    ])
    .errs(&[
        "Usage: /timeout name [10m] [reason]",
        "Timeouts go from 1 second to 2 weeks.",
    ]),
    tw(
        "unban",
        "/unban name",
        "Unbans a user, or ends their timeout.",
        "1.49",
        &[ex("/unban someone", "")],
    )
    .help("/unban")
    .args(&[NAME])
    .errs(&["Usage: /unban name"]),
    tw(
        "untimeout",
        "/untimeout name",
        "Ends a timeout early. Twitch treats it like /unban, so it also lifts a ban.",
        "1.49",
        &[ex("/untimeout someone", "")],
    )
    .args(&[NAME])
    .errs(&["Usage: /untimeout name"]),
    tw(
        "delete",
        "/delete message-id",
        "Deletes one message.",
        "1.49",
        &[ex("/delete bddd8905-c104-4f7a-9c2e-0a1b2c3d4e5f", "")],
    )
    .args(&[req("message-id", "word")])
    .errs(&["Usage: /delete message-id"]),
    tw(
        "clear",
        "/clear",
        "Clears the chat.",
        "1.49",
        &[ex("/clear", "Chat cleared.")],
    )
    .errs(&["Clearing the whole chat needs a Helix sign-in with mod scopes."]),
    tw(
        "slow",
        "/slow [30]",
        "Slow mode with that many seconds between messages. Default 30.",
        "1.49",
        &[ex("/slow 10", "Slow mode on.")],
    )
    .args(&[opt("seconds", "duration")])
    .errs(&["Usage: /slow [30]"]),
    tw(
        "slowoff",
        "/slowoff",
        "Turns slow mode off.",
        "1.49",
        &[ex("/slowoff", "Slow mode off.")],
    ),
    tw(
        "followers",
        "/followers [10m]",
        "Followers-only chat; the time is how long someone must have followed. No time means all followers.",
        "1.49",
        &[ex("/followers 1h", "Followers-only on.")],
    )
    .args(&[opt("time", "duration").note("Up to 3 months.")])
    .errs(&["Usage: /followers [10m]", "Followers-only goes up to 3 months."]),
    tw(
        "followersoff",
        "/followersoff",
        "Turns followers-only off.",
        "1.49",
        &[ex("/followersoff", "Followers-only off.")],
    ),
    tw(
        "subscribers",
        "/subscribers",
        "Subscribers-only chat on.",
        "1.49",
        &[ex("/subscribers", "Subscribers-only on.")],
    ),
    tw(
        "subscribersoff",
        "/subscribersoff",
        "Subscribers-only chat off.",
        "1.49",
        &[ex("/subscribersoff", "Subscribers-only off.")],
    ),
    tw(
        "emoteonly",
        "/emoteonly",
        "Emote-only chat on.",
        "1.49",
        &[ex("/emoteonly", "Emote-only on.")],
    ),
    tw(
        "emoteonlyoff",
        "/emoteonlyoff",
        "Emote-only chat off.",
        "1.49",
        &[ex("/emoteonlyoff", "Emote-only off.")],
    ),
    tw(
        "uniquechat",
        "/uniquechat",
        "Unique-chat mode on (no repeated messages).",
        "1.49",
        &[ex("/uniquechat", "Unique-chat on.")],
    )
    .alias(&["r9kbeta"]),
    tw(
        "uniquechatoff",
        "/uniquechatoff",
        "Unique-chat mode off.",
        "1.49",
        &[ex("/uniquechatoff", "Unique-chat off.")],
    )
    .alias(&["r9kbetaoff"]),
    tw(
        "announce",
        "/announce message",
        "Posts an announcement in the channel’s color.",
        "1.49",
        &[ex("/announce Stream starts at 8", "")],
    )
    .args(&[req("message", "text")])
    .errs(&["Usage: /announce message"]),
    tw(
        "announceblue",
        "/announceblue message",
        "Posts a blue announcement.",
        "1.49",
        &[ex("/announceblue Be nice", "")],
    )
    .args(&[req("message", "text")])
    .errs(&["Usage: /announceblue message"]),
    tw(
        "announcegreen",
        "/announcegreen message",
        "Posts a green announcement.",
        "1.49",
        &[ex("/announcegreen Giveaway soon", "")],
    )
    .args(&[req("message", "text")])
    .errs(&["Usage: /announcegreen message"]),
    tw(
        "announceorange",
        "/announceorange message",
        "Posts an orange announcement.",
        "1.49",
        &[ex("/announceorange Rules changed", "")],
    )
    .args(&[req("message", "text")])
    .errs(&["Usage: /announceorange message"]),
    tw(
        "announcepurple",
        "/announcepurple message",
        "Posts a purple announcement.",
        "1.49",
        &[ex("/announcepurple Welcome", "")],
    )
    .args(&[req("message", "text")])
    .errs(&["Usage: /announcepurple message"]),
    tw(
        "shoutout",
        "/shoutout name",
        "Gives someone a shoutout.",
        "1.49",
        &[ex("/shoutout friend", "Shouted out friend.")],
    )
    .alias(&["so"])
    .args(&[NAME])
    .errs(&[USAGE_NAME, "No user called {login}."]),
    tw(
        "raid",
        "/raid name",
        "Starts a raid to another channel.",
        "1.49",
        &[ex("/raid friend", "Raiding friend…")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME, "No user called {login}."]),
    tw(
        "unraid",
        "/unraid",
        "Cancels the raid that is starting.",
        "1.49",
        &[ex("/unraid", "Raid canceled.")],
    ),
    tw(
        "warn",
        "/warn name reason",
        "Warns someone; they have to accept it before they can chat again.",
        "1.49",
        &[ex("/warn someone stop it", "")],
    )
    .help("/warn")
    .args(&[NAME, req("reason", "text")])
    .errs(&[
        "Usage: /warn name reason",
        "A warning needs a reason: /warn name reason",
    ])
    .notes("Warn on a chat line opens a small reason box that sends this; Enter sends, Escape cancels. Twitch needs a reason of 1 to 500 characters."),
    tw(
        "shield",
        "/shield",
        "Shield mode on.",
        "1.49",
        &[ex("/shield", "Shield mode on.")],
    ),
    tw(
        "shieldoff",
        "/shieldoff",
        "Shield mode off.",
        "1.49",
        &[ex("/shieldoff", "Shield mode off.")],
    ),
    tw(
        "monitor",
        "/monitor name",
        "Monitors a suspicious user: their messages show to mods with a marker.",
        "M3",
        &[ex("/monitor someone", "")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME]),
    tw(
        "unmonitor",
        "/unmonitor name",
        "Stops monitoring someone.",
        "M3",
        &[ex("/unmonitor someone", "")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME]),
    tw(
        "restrict",
        "/restrict name",
        "Restricts a suspicious user: their messages show to mods only.",
        "M3",
        &[ex("/restrict someone", "")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME]),
    tw(
        "unrestrict",
        "/unrestrict name",
        "Lifts the restriction on someone.",
        "M3",
        &[ex("/unrestrict someone", "")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME]),
    tw(
        "mod",
        "/mod name",
        "Makes someone a moderator.",
        "1.50",
        &[ex("/mod friend", "Made friend a mod.")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME]),
    tw(
        "unmod",
        "/unmod name",
        "Removes someone’s moderator role.",
        "1.50",
        &[ex("/unmod friend", "friend is no longer a mod.")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME]),
    tw(
        "vip",
        "/vip name",
        "Gives someone VIP.",
        "1.50",
        &[ex("/vip friend", "Made friend a VIP.")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME]),
    tw(
        "unvip",
        "/unvip name",
        "Removes someone’s VIP.",
        "1.50",
        &[ex("/unvip friend", "friend is no longer a VIP.")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME]),
    tw(
        "leadmod",
        "/leadmod name",
        "Makes someone a lead moderator.",
        "V107-R28",
        &[ex("/leadmod friend", "Made friend a lead mod.")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME, cyex_moderation::roles::NEEDS_GQL]),
    tw(
        "unleadmod",
        "/unleadmod name",
        "Removes someone’s lead moderator role.",
        "V107-R28",
        &[ex("/unleadmod friend", "friend is no longer a lead mod.")],
    )
    .args(&[NAME])
    .errs(&[USAGE_NAME, cyex_moderation::roles::NEEDS_GQL]),
    tw(
        "mods",
        "/mods",
        "Lists the channel’s moderators.",
        "1.50",
        &[ex("/mods", "Mods: one, two.")],
    )
    .errs(&["Listing needs a Helix sign-in."]),
    tw(
        "vips",
        "/vips",
        "Lists the channel’s VIPs.",
        "1.50",
        &[ex("/vips", "VIPs: one, two.")],
    )
    .errs(&["Listing needs a Helix sign-in."]),
    tw(
        "commercial",
        "/commercial [30|60|90|120|150|180]",
        "Runs an ad break of that many seconds. Default 30.",
        "1.49",
        &[ex("/commercial 60", "Ad started (60s).")],
    )
    .args(&[opt("seconds", "number").values(&["30", "60", "90", "120", "150", "180"])])
    .errs(&[
        "Usage: /commercial [30|60|90|120|150|180]",
        "You need a Helix sign-in with mod scopes to run an ad.",
    ]),
    tw(
        "marker",
        "/marker [text]",
        "Adds a stream marker.",
        "1.49",
        &[ex("/marker big play", "Marker added.")],
    )
    .args(&[opt("text", "text")])
    .errs(&["You need a Helix sign-in with mod scopes to add a marker."]),
    tw(
        "pin",
        "/pin message | /pin message-id [5m]",
        "Pins a message by its id, or sends your text and pins it.",
        "1.9",
        &[ex("/pin Giveaway at 8", "")],
    )
    .args(&[
        req("message", "text")
            .note("A message id pins that message; any other text is sent and pinned."),
        opt("time", "duration").note("Only with an id. 30 seconds to 30 minutes."),
    ])
    .errs(&[
        "Usage: /pin message, or /pin message-id [5m]",
        "Usage: /pin message-id [5m]",
        "That’s over 500 characters.",
    ])
    .notes("With an id the line “Pinned.” shows."),
    tw(
        "unpin",
        "/unpin",
        "Takes the current pin down.",
        "1.9",
        &[ex("/unpin", "Unpinned.")],
    ),
];
