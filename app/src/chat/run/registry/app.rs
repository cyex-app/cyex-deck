//! The app's own commands (everything in `OWN` except the 7TV ones in `cosmetics.rs`),
//! split around them so `/help` keeps its order.

use super::Changes::{Chat, Local, Settings, Twitch, Window};
use super::{Cmd, ex, opt, req};

const WHO: &str = "Say who: a name after the command.";
const NOT_A_NAME: &str = "{word} isn’t a Twitch name.";
const BLOCKS: &[&str] = &[
    "Sign in to block people.",
    WHO,
    NOT_A_NAME,
    "No user called {login}.",
    "This account can’t block people. Sign in again in Settings → Accounts.",
];

pub(super) const BEFORE: &[Cmd] = &[
    Cmd::new("me", "/me message", "Sends an action message, shown in the sender’s color.", "1.8")
        .help("/me")
        .args(&[req("message", "text")])
        .changes(Chat, "Goes out as the chat’s account.")
        .ex(&[ex("/me waves", "(in chat) sender waves")])
        .errs(&["Sign in to chat.", "Not connected yet.", "This chat is closed."])
        .notes("Anonymous chats can’t send."),
    Cmd::new("echo", "/echo text", "Shows text in this chat, only to you.", "1.8")
        .help("/echo")
        .args(&[req("text", "text")])
        .changes(Local, "")
        .ex(&[ex("/echo hello", "hello")])
        .notes("Safe in custom command bodies: nothing reaches Twitch."),
    Cmd::new("help", "/help", "Lists the app’s commands.", "1.8")
        .alias(&["commands"])
        .changes(Local, "")
        .ex(&[ex("/help", "/me, /echo, /set var, … Your own commands are in Settings → Commands.")]),
    Cmd::new(
        "set",
        "/set var key value",
        "Saves a value your commands read with $get(var,key). An empty value removes it.",
        "1.8",
    )
    .help("/set var")
    .args(&[
        req("var", "word").values(&["var"]),
        req("key", "word"),
        opt("value", "text").note("Empty removes the key."),
    ])
    .changes(Settings, "Commands settings.")
    .ex(&[ex("/set var lucky 7", ""), ex("/set var lucky", "")])
    .errs(&["Only /set var key value works.", "Usage: /set var key value"]),
    Cmd::new("chain", "/chain /cmd one | /cmd two", "Runs several commands one after another.", "S3")
        .help("/chain")
        .args(&[req("commands", "command").note("Split by |. Two || make one literal |.")])
        .changes(Local, "Whatever the commands do.")
        .ex(&[ex("/chain /echo a | /echo b", "a, then b")])
        .errs(&["Too many commands in a row."])
        .notes("Stops at the first command that fails."),
    Cmd::new("foreach", "/foreach a b c > /cmd $1", "Runs a command once for each word before the >.", "S3")
        .help("/foreach")
        .args(&[
            req("items", "text").note("Words, split by spaces."),
            req("command", "command").note("After the first single >. Two >> make one literal >."),
        ])
        .changes(Local, "Whatever the command does.")
        .ex(&[ex("/foreach a b > /echo $1", "a, then b")])
        .errs(&["Usage: /foreach a b c > /command $1", "Too many commands in a row."]),
    Cmd::new("runin", "/runin channel /cmd", "Runs a command as if typed in another open chat.", "S3")
        .help("/runin")
        .args(&[req("channel", "channel"), req("command", "command")])
        .changes(Local, "Whatever the command does, in that chat.")
        .ex(&[ex("/runin somechannel /echo hi", "hi (in somechannel’s chat)")])
        .errs(&["Usage: /runin channel /command", "Not in {channel}’s chat."]),
    Cmd::new(
        "timer",
        "/timer [-rso] [:id] 5m|17:30|[2026-10-11 17:30] /cmd",
        "Runs a command later in this chat. Also /timer list and /timer stop id.",
        "S3",
    )
    .help("/timer")
    .args(&[
        opt("options", "word")
            .values(&["-r", "-s", "-o"])
            .note("-r keeps it through a restart, -s skips the “started” line, -o replaces a timer with the same id."),
        opt("id", "word").note(":id. An id ending in * gets a number."),
        req("when", "duration").note("5m, 17:30, or [yyyy-mm-dd hh:mm]; list; stop."),
        req("command", "command"),
    ])
    .changes(Local, "Runs the command when the time comes; -r saves the timer in the commands settings.")
    .ex(&[ex("/timer 5m /echo time", "Timer t1 runs in 5m."), ex("/timer list", "No timers running.")])
    .errs(&[
        "Unknown timer option -{c}.",
        "A ‘]’ is missing.",
        "Say which timer: /timer stop <id>",
        "Usage: /timer 5m <command>",
        "Can’t read the time ‘{head}’.",
        "No timer called {id}.",
        "A timer called {id} is running. -o replaces it.",
        "Too many timers running.",
    ]),
    Cmd::new("user", "/user name", "Opens a user’s card for this channel.", "V102-10")
        .help("/user")
        .alias(&["usercard"])
        .args(&[req("name", "login").note("Write it as @Name, name or name followed by a comma.")])
        .changes(Window, "Opens the user card window.")
        .ex(&[ex("/user @SomeOne", "(opens someone’s card)")])
        .errs(&[WHO, NOT_A_NAME]),
    Cmd::new("logs", "/logs name", "Opens a user’s chat logs for this channel in the browser.", "1.8")
        .help("/logs")
        .args(&[req("name", "login")])
        .changes(Window, "Opens the logs page in your browser.")
        .ex(&[ex("/logs someone", "(opens the logs page)")])
        .errs(&[WHO, NOT_A_NAME]),
    Cmd::new("block", "/block name", "Blocks someone on Twitch.", "F2")
        .help("/block")
        .args(&[req("name", "login")])
        .changes(Twitch, "Blocks the user for the chat’s account.")
        .ex(&[ex("/block someone", "You blocked someone.")])
        .errs(BLOCKS),
    Cmd::new("unblock", "/unblock name", "Unblocks someone on Twitch.", "F2")
        .help("/unblock")
        .args(&[req("name", "login")])
        .changes(Twitch, "Unblocks the user for the chat’s account.")
        .ex(&[ex("/unblock someone", "You unblocked someone.")])
        .errs(BLOCKS),
    Cmd::new("hideuser", "/hideuser name", "Hides a user’s messages in every chat, only in this app.", "C28")
        .help("/hideuser")
        .args(&[req("name", "login")])
        .changes(Settings, "Highlights settings. Nothing happens on Twitch.")
        .ex(&[ex("/hideuser someone", "someone is hidden now. /unhideuser brings them back.")])
        .errs(&[WHO, NOT_A_NAME, "No user called {login}."]),
    Cmd::new("unhideuser", "/unhideuser name", "Shows a hidden user again.", "C28")
        .help("/unhideuser")
        .args(&[req("name", "login")])
        .changes(Settings, "Highlights settings.")
        .ex(&[ex("/unhideuser someone", "someone shows again.")])
        .errs(&[WHO, NOT_A_NAME, "No user called {login}."]),
    Cmd::new("mutepings", "/mutepings", "Turns sounds and flashes for this chat off, or on again.", "C27")
        .help("/mutepings")
        .changes(Settings, "Highlights settings.")
        .ex(&[ex("/mutepings", "This chat won’t play sounds or flash now.")])
        .notes("Run it again to undo."),
    Cmd::new("tl", "/tl language message", "Translates a message and sends the translation.", "C17")
        .help("/tl")
        .args(&[req("language", "word").note("A language code such as de."), req("message", "text")])
        .changes(Chat, "Sends the translated text as the chat’s account.")
        .ex(&[ex("/tl de good morning", "(in chat) guten Morgen")])
        .errs(&[
            "Usage: /tl de your message",
            "Translation isn’t working right now.",
            "Nothing came back.",
            "Sign in to chat.",
        ]),
    Cmd::new("translate", "/translate text", "Translates text into your language and shows it only to you.", "C17")
        .help("/translate")
        .args(&[req("text", "text")])
        .changes(Local, "")
        .ex(&[ex("/translate guten Morgen", "good morning")])
        .errs(&["Usage: /translate text", "Translation isn’t working right now.", "Nothing came back."]),
    Cmd::new("vote", "/vote number", "Votes on the running poll or prediction by choice number.", "C4")
        .help("/vote")
        .args(&[req("number", "number")])
        .changes(Twitch, "Casts the vote for the chat’s account.")
        .ex(&[ex("/vote 1", "")])
        .errs(&[
            "Usage: /vote 1",
            "Open the chat to vote.",
            "No such choice.",
            "This chat is closed.",
            "There’s no prediction right now.",
        ]),
    Cmd::new("search", "/search text", "Opens the chat search with that text.", "F1")
        .help("/search")
        .args(&[opt("text", "text")])
        .changes(Window, "Opens the search pop-up in this chat.")
        .ex(&[ex("/search hello", "(search opens)")]),
    Cmd::new("popout", "/popout", "Pops the chat next to the stream out into its own window.", "1.51")
        .help("/popout")
        .changes(Window, "Opens a pop-out window.")
        .ex(&[ex("/popout", "(the chat pops out)")])
        .notes("Only a chat next to a stream pops out; anywhere else it shows “Only a chat next to a stream pops out.”"),
    Cmd::new("color", "/color #RRGGBB", "Changes your name color on Twitch.", "1.40")
        .help("/color")
        .args(&[req("color", "word").note("A hex color. Any color needs Prime or Turbo; named ones don’t.")])
        .changes(Twitch, "Your chat color, for the chat’s account.")
        .ex(&[ex("/color #1E90FF", "Name color changed.")])
        .errs(&[
            "Sign in to set a color.",
            "Use a color like #1E90FF.",
            "Sign in to change your color.",
            "Any color needs Prime or Turbo. Pick one of the named ones.",
        ]),
];

pub(super) const AFTER: &[Cmd] = &[
    Cmd::new("w", "/w name message", "Whispers someone, from your default account.", "1.26")
        .help("/w")
        .alias(&["whisper"])
        .args(&[req("name", "login"), req("message", "text")])
        .changes(Twitch, "Sends a whisper. With inline whispers off, a short note says it went.")
        .ex(&[ex("/w someone hi", "Whisper sent to someone.")])
        .errs(&[
            "Usage: /w name message",
            "Type something first.",
            "You can’t whisper yourself.",
            "Whispers can be {max} characters at most.",
        ]),
    Cmd::new("r", "/r message", "Replies to the last person who whispered you.", "1.26")
        .help("/r")
        .args(&[req("message", "text")])
        .changes(Twitch, "Sends a whisper.")
        .ex(&[ex("/r thanks", "Whisper sent to someone.")])
        .errs(&["Nobody has whispered you yet.", "Type something first."]),
    Cmd::new(
        "nuke",
        "/nuke phrase [60s] [10m|ban]",
        "Bans or times out everyone who recently said a phrase. A phrase with spaces goes in quotes; /regex/ works too.",
        "M1",
    )
    .args(&[
        req("phrase", "text").note("Plain, “quoted” or /regex/."),
        opt("window", "duration").note("How far back to look. Default 60s."),
        opt("punishment", "duration").values(&["ban"]).note("A timeout length, or ban. Default ban."),
    ])
    .changes(Twitch, "Bans or times out the matching users in the recent messages, paced like a mass action. Says “Nuke: nothing matched.” when no one fits.")
    .ex(&[ex("/nuke \"spam link\" 2m 10m", "Nuke: hitting 3 user(s)…")])
    .errs(&[
        "Sign in to use /nuke.",
        "Usage: /nuke phrase [60s] [10m|ban]",
        "That regex doesn’t read.",
        "Didn’t understand “{word}”. Usage: /nuke phrase [60s] [10m|ban]",
    ])
    .notes("Moderators and up."),
];
