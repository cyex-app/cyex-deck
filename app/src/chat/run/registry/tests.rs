//! The registry against the code, both ways, and the file against the registry.

use std::collections::BTreeSet;

use cyex_chat::commands::{FUNCTIONS, vars::FORMS};
use cyex_moderation::command::NAMES;
use cyex_settings::schema::Account;
use cyex_twitch::gql::{ChannelInfo, StreamInfo};

use super::language::{self, VARIABLES};
use super::render::{render, roles};
use super::{all, help_line};
use crate::chat::run::builtin::{MOD_COMMANDS, OWN};
use crate::chat::run::{AUTOMATION_VALUES, MENU_VALUES, MSG_TIME, values};

fn set<'a>(it: impl IntoIterator<Item = &'a str>) -> BTreeSet<&'a str> {
    it.into_iter().collect()
}

fn show(s: &BTreeSet<&str>) -> String {
    s.iter().copied().collect::<Vec<_>>().join(", ")
}

#[test]
fn every_command_in_code_is_in_the_registry_and_back() {
    let code = set(OWN.iter().chain(MOD_COMMANDS).chain(NAMES).copied());
    let table = set(all().flat_map(|c| std::iter::once(c.name).chain(c.aliases.iter().copied())));
    let (missing, extra) = (&code - &table, &table - &code);
    assert!(
        missing.is_empty(),
        "in the code, not in the registry: {}",
        show(&missing)
    );
    assert!(
        extra.is_empty(),
        "in the registry, not in the code: {}",
        show(&extra)
    );
    // A name once, as a command or an alias.
    let listed = all().flat_map(|c| std::iter::once(c.name).chain(c.aliases.iter().copied()));
    assert_eq!(listed.count(), table.len(), "a name is in the registry twice");
}

#[test]
fn functions_match_the_parser() {
    let table = set(language::FUNCTIONS.iter().map(|f| f.name));
    let code = set(FUNCTIONS.iter().copied());
    assert_eq!(
        table,
        code,
        "missing {}; extra {}",
        show(&(&code - &table)),
        show(&(&table - &code))
    );
}

/// The names of the plain `$(name)` variables for one scope, or the `{…}` forms.
fn names(scope: &str, brace: bool) -> BTreeSet<&'static str> {
    VARIABLES
        .iter()
        .filter(|v| v.scope.contains(&scope) && v.syntax.starts_with('{') == brace)
        .filter(|v| brace || v.syntax == format!("$({})", v.name))
        .map(|v| v.name)
        .collect()
}

#[test]
fn variables_match_the_code() {
    let live = ChannelInfo {
        id: "1".into(),
        login: "c".into(),
        name: "C".into(),
        avatar: None,
        offline_image: None,
        stream: Some(StreamInfo {
            rerun: false,
            title: "t".into(),
            game: Some("g".into()),
            viewers: 1,
            started_at: String::new(),
        }),
    };
    let me = Account {
        user_id: "7".into(),
        login: "me".into(),
        display_name: "Me".into(),
        ..Default::default()
    };
    let p = values("c", &["c".into()], Some(&live), Some(&me));
    assert_eq!(set(p.keys()), names("everywhere", false), "everywhere");
    assert_eq!(
        set(MENU_VALUES.into_iter().chain([MSG_TIME])),
        names("menus", false),
        "menus"
    );
    assert_eq!(names("usercard", false), names("menus", false), "usercard");
    assert_eq!(set(AUTOMATION_VALUES), names("automations", false), "automations");
    let seven_tv: BTreeSet<&str> = VARIABLES
        .iter()
        .filter(|v| v.syntax.starts_with('{'))
        .map(|v| v.syntax)
        .collect();
    assert_eq!(seven_tv, set(FORMS), "7TV forms");
}

#[test]
fn help_line_reads_as_before() {
    assert_eq!(
        help_line(),
        "/me, /echo, /set var, /chain, /foreach, /runin, /timer, /user, /logs, /block, /unblock, /hideuser, /unhideuser, /mutepings, /tl, /translate, /vote, /search, /popout, /color, /cosmetics, /combo, /paint, /badge, /w, /r. Mods: /ban, /timeout, /unban, /warn and Twitch’s other mod commands. Your own commands are in Settings → Commands."
    );
}

const KINDS: [&str; 11] = [
    "text", "word", "name", "login", "channel", "number", "duration", "combo", "paint", "badge", "command",
];

#[test]
fn every_entry_is_filled_in() {
    for c in all() {
        let n = c.name;
        assert!(
            !c.summary.is_empty() && !c.syntax.is_empty() && !c.spec.is_empty(),
            "{n}"
        );
        assert!(!c.examples.is_empty(), "{n} has no example");
        assert!(c.syntax.starts_with(&format!("/{n}")), "{n} syntax");
        for a in c.args {
            assert!(KINDS.contains(&a.kind), "{n}: kind {}", a.kind);
        }
    }
    for v in VARIABLES {
        assert!(
            !v.value.is_empty() && !v.empty.is_empty() && !v.examples.is_empty() && !v.spec.is_empty(),
            "{}",
            v.name
        );
    }
    assert!(
        language::FUNCTIONS
            .iter()
            .all(|f| !f.summary.is_empty() && !f.syntax.is_empty())
    );
}

#[test]
fn roles_come_from_the_code() {
    assert_eq!(roles("echo"), "anyone");
    assert_eq!(roles("ban"), "mod");
    assert_eq!(roles("nuke"), "mod");
    assert_eq!(roles("vip"), "lead");
    assert_eq!(roles("raid"), "broadcaster");
}

#[test]
fn registry_file_is_current() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/commands/registry.json");
    let now = render();
    if std::env::var("CYEX_REGISTRY_WRITE").is_ok_and(|v| v == "1") {
        std::fs::write(path, &now).expect("write docs/commands/registry.json");
        return;
    }
    // tools/commands/sections.mjs adds these after the write; `npm run test:ui` checks them.
    const NODE_KEYS: [&str; 7] = [
        "current",
        "statusSince",
        "sections",
        "features",
        "changelog",
        "settings",
        "hotkeys",
    ];
    let mut file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap_or_default()).unwrap_or_default();
    if let Some(m) = file.as_object_mut() {
        m.retain(|k, _| !NODE_KEYS.contains(&k.as_str()));
    }
    let now: serde_json::Value = serde_json::from_str(&now).expect("the registry parses");
    assert!(
        file == now,
        "docs/commands/registry.json is stale: run tools/commands/registry.ps1"
    );
}
