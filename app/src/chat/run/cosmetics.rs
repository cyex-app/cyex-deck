//! V1051-04 `/combo`, `/paint`, `/badge`: 7TV combos and cosmetics from any chat signed in as
//! the account. Results are local lines; nothing goes to the channel.

use cyex_chat::model::NoticeKind;
use cyex_settings::schema::{Combo, ComboDone, ComboItem, ComboOp, WORDS};
use tauri::AppHandle;

use super::{Place, UiAction, local};
use crate::cosmetics::lookup::{self, Have};
use crate::cosmetics::{self, Kind, Part, Want, apply, read, select_error, why};

/// The arm in `builtin::run` and `OWN` must list these (checked in builtin's tests).
#[cfg(test)]
pub(super) const NAMES: [&str; 3] = ["combo", "paint", "badge"];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ComboCmd {
    Apply(String),
    List,
    Save(String),
    Delete(String),
    Rename(String, String),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Pick {
    None,
    Named(String),
}

const QUOTE: &str = "A quote isn’t closed.";

/// The first word of `s` (leading space skipped), whether it was quoted, and the rest.
fn word(s: &str) -> Result<Option<(String, bool, &str)>, String> {
    let s = s.trim_start();
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return Ok(None);
    };
    if first == '"' || first == '“' {
        let rest = chars.as_str();
        let end = rest.find(['"', '”']).ok_or_else(|| QUOTE.to_owned())?;
        let after = &rest[end..];
        let skip = after.chars().next().map_or(0, char::len_utf8);
        return Ok(Some((rest[..end].to_owned(), true, &after[skip..])));
    }
    let end = s.find(char::is_whitespace).unwrap_or(s.len());
    Ok(Some((s[..end].to_owned(), false, &s[end..])))
}

/// Words, with "…" or “…” keeping spaces together (quotes removed). `Err` when a quote isn't closed.
fn words(args: &str) -> Result<Vec<(String, bool /* quoted */)>, String> {
    let mut out = Vec::new();
    let mut rest = args;
    while let Some((w, quoted, after)) = word(rest)? {
        out.push((w, quoted));
        rest = after;
    }
    Ok(out)
}

/// The first word when it is quoted, else all of `s`.
fn name_of(s: &str) -> Result<String, String> {
    match word(s)? {
        Some((w, true, _)) => Ok(w),
        _ => Ok(s.trim().to_owned()),
    }
}

pub(super) fn combo_args(args: &str) -> Result<ComboCmd, String> {
    let args = args.trim();
    let Some((first, quoted, rest)) = word(args)? else {
        return Err("Say which combo: /combo <name>. /combo list shows yours.".into());
    };
    let sub = (!quoted)
        .then(|| WORDS.iter().find(|w| w.eq_ignore_ascii_case(&first)))
        .flatten();
    let Some(sub) = sub else {
        return name_of(args).map(ComboCmd::Apply);
    };
    match *sub {
        "list" => Ok(ComboCmd::List),
        "save" | "delete" => {
            let name = name_of(rest)?;
            if name.is_empty() {
                return Err(format!("Say a name: /combo {sub} <name>."));
            }
            Ok(if *sub == "save" { ComboCmd::Save(name) } else { ComboCmd::Delete(name) })
        }
        _ => match <[_; 2]>::try_from(words(rest)?) {
            Ok([(a, _), (b, _)]) => Ok(ComboCmd::Rename(a, b)),
            Err(_) => Err("Put names with spaces in quotes: /combo rename \"old name\" \"new name\".".into()),
        },
    }
}

pub(super) fn pick_args(args: &str, what: Kind) -> Result<Pick, String> {
    let args = args.trim();
    let kind = if what == Kind::Paint { "paint" } else { "badge" };
    let Some((first, quoted, _)) = word(args)? else {
        return Err(format!("Say which {kind}, or none: /{kind} <name>."));
    };
    if quoted {
        return Ok(Pick::Named(first));
    }
    if args.eq_ignore_ascii_case("none") {
        return Ok(Pick::None);
    }
    Ok(Pick::Named(args.to_owned()))
}

fn quoted_if_spaced(name: &str) -> String {
    if name.contains(' ') { format!("“{name}”") } else { name.to_owned() }
}

fn combo_of(done: ComboDone) -> Combo {
    match done {
        ComboDone::Saved { combo } | ComboDone::Changed { combo } | ComboDone::Deleted { combo, .. } => combo,
    }
}

/// Automations (S4) start on other people's messages, so they can only list combos: wearing,
/// saving, deleting and renaming stay yours (each wear costs a 7TV request), like `{combo:…}`.
fn automation_may(name: &str, args: &str) -> bool {
    name == "combo" && matches!(combo_args(args), Ok(ComboCmd::List))
}

pub(super) async fn run(app: &AppHandle, place: &Place, name: &str, args: &str) -> Result<Option<UiAction>, String> {
    let account = place.account().ok_or_else(|| format!("Sign in to use /{name}."))?;
    if place.automation && !automation_may(name, args) {
        return Err(format!("/{name} doesn’t run in automations."));
    }
    let line = if name == "combo" {
        combo(app, account, combo_args(args)?).await?
    } else {
        let kind = if name == "paint" { Kind::Paint } else { Kind::Badge };
        pick(app, account, kind, pick_args(args, kind)?).await?
    };
    local(app, &place.key, NoticeKind::Info, &line);
    Ok(None)
}

fn find(app: &AppHandle, account: &str, name: &str) -> Result<Combo, String> {
    cosmetics::find_combo(app, account, name)
        .ok_or_else(|| format!("No combo called “{name}”. /combo list shows yours."))
}

/// Names in list order; a ⚠ after one whose part isn't usable, judged only from a cached inventory.
fn list_line(app: &AppHandle, account: &str) -> String {
    let combos = cosmetics::list_combos(app, account);
    if combos.is_empty() {
        return "No combos yet. /combo save <name> saves what you wear now.".into();
    }
    let inv = cosmetics::cached(app, account);
    let missing = |c: &Combo| {
        inv.as_ref().is_some_and(|i| {
            let bad = |k: Kind, it: &Option<ComboItem>| {
                it.as_ref().is_some_and(|it| lookup::have(i, k, &it.id) != Have::Usable)
            };
            bad(Kind::Paint, &c.paint) || bad(Kind::Badge, &c.badge)
        })
    };
    let names: Vec<String> = combos
        .iter()
        .map(|c| {
            let n = quoted_if_spaced(&c.name);
            if missing(c) { format!("{n} ⚠") } else { n }
        })
        .collect();
    format!("Your combos: {}.", names.join(", "))
}

async fn combo(app: &AppHandle, account: &str, cmd: ComboCmd) -> Result<String, String> {
    Ok(match cmd {
        ComboCmd::List => list_line(app, account),
        ComboCmd::Save(n) => {
            let inv = read(app, account).await.map_err(|w| why(&w).to_owned())?;
            let (paint, badge) = lookup::worn(&inv);
            let label = |it: &Option<ComboItem>, none: &str| it.as_ref().map_or(none.to_owned(), |i| i.name.clone());
            let (p, b) = (label(&paint, "no paint"), label(&badge, "no badge"));
            let done = cosmetics::change_combos(app, account, ComboOp::Save { name: n, paint, badge })
                .map_err(|e| e.to_string())?;
            format!("Saved “{}”: {p} and {b}.", combo_of(done).name)
        }
        ComboCmd::Delete(n) => {
            let c = find(app, account, &n)?;
            cosmetics::change_combos(app, account, ComboOp::Delete { id: c.id }).map_err(|e| e.to_string())?;
            format!("Deleted “{}”.", c.name)
        }
        ComboCmd::Rename(a, b) => {
            let c = find(app, account, &a)?;
            let done = cosmetics::change_combos(app, account, ComboOp::Rename { id: c.id, name: b })
                .map_err(|e| e.to_string())?;
            format!("Renamed “{}” to “{}”.", c.name, combo_of(done).name)
        }
        ComboCmd::Apply(n) => {
            let c = find(app, account, &n)?;
            let want = Want { paint: c.paint.clone(), badge: c.badge.clone() };
            apply(app, account, want, Some(&c.name)).await.map_err(|e| select_error(&e))?.line
        }
    })
}

async fn pick(app: &AppHandle, account: &str, kind: Kind, pick: Pick) -> Result<String, String> {
    let (noun, cap, none_line) = match kind {
        Kind::Paint => ("paint", "Paint", "No paint now."),
        Kind::Badge => ("badge", "Badge", "No badge now."),
    };
    let inv = read(app, account).await.map_err(|w| why(&w).to_owned())?;
    let target = match pick {
        Pick::None => None,
        Pick::Named(t) => Some(
            lookup::find(&inv, kind, &t)
                .ok_or_else(|| format!("You don’t have a {noun} called “{t}”. Tab shows yours."))?,
        ),
    };
    // The other part is set to what's worn, so it stays as it is.
    let (wp, wb) = lookup::worn(&inv);
    let want = match kind {
        Kind::Paint => Want { paint: target.clone(), badge: wb },
        Kind::Badge => Want { paint: wp, badge: target.clone() },
    };
    let done = apply(app, account, want, None).await.map_err(|e| select_error(&e))?;
    match if kind == Kind::Paint { done.paint } else { done.badge } {
        Part::NeedsSub => Err("That one’s for 7TV subscribers.".into()),
        Part::NotOwned => Err("7TV says that one isn’t yours.".into()),
        Part::Failed { error } => Err(select_error(&error)),
        // Overtaken: a newer pick took over; the account ends up as that pick says.
        Part::Changed | Part::Same | Part::Overtaken => {
            Ok(target.map_or_else(|| none_line.to_owned(), |t| format!("{cap}: {}.", t.name)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const USAGE: &str = "Say which combo: /combo <name>. /combo list shows yours.";
    const RENAME: &str = "Put names with spaces in quotes: /combo rename \"old name\" \"new name\".";

    fn apply_of(s: &str) -> ComboCmd {
        ComboCmd::Apply(s.into())
    }

    #[test]
    fn combo_apply_forms() {
        assert_eq!(combo_args("night owl"), Ok(apply_of("night owl")));
        assert_eq!(combo_args("\"night owl\""), Ok(apply_of("night owl")));
        assert_eq!(combo_args("  NIGHT   owl "), Ok(apply_of("NIGHT   owl")));
        assert_eq!(combo_args("\"list\""), Ok(apply_of("list")));
    }

    #[test]
    fn combo_subcommands() {
        assert_eq!(combo_args("LIST"), Ok(ComboCmd::List));
        assert_eq!(combo_args("list extra"), Ok(ComboCmd::List));
        assert_eq!(combo_args("save gold set"), Ok(ComboCmd::Save("gold set".into())));
        assert_eq!(combo_args("save \"gold set\""), Ok(ComboCmd::Save("gold set".into())));
        assert_eq!(combo_args("save"), Err("Say a name: /combo save <name>.".into()));
        assert_eq!(combo_args("delete"), Err("Say a name: /combo delete <name>.".into()));
        assert_eq!(combo_args("delete “gold set”"), Ok(ComboCmd::Delete("gold set".into())));
    }

    #[test]
    fn combo_rename_and_errors() {
        assert_eq!(combo_args("rename a b"), Ok(ComboCmd::Rename("a".into(), "b".into())));
        assert_eq!(combo_args("rename \"a b\" c"), Ok(ComboCmd::Rename("a b".into(), "c".into())));
        assert_eq!(combo_args("rename a b c"), Err(RENAME.into()));
        assert_eq!(combo_args("\"open"), Err(QUOTE.into()));
        assert_eq!(combo_args(""), Err(USAGE.into()));
    }

    #[test]
    fn automations_only_list() {
        assert!(automation_may("combo", "list"));
        assert!(!automation_may("combo", "night owl"));
        assert!(!automation_may("combo", "delete night"));
        assert!(!automation_may("paint", "none"));
        assert!(!automation_may("badge", "Founder"));
    }

    #[test]
    fn pick_forms() {
        assert_eq!(pick_args("none", Kind::Paint), Ok(Pick::None));
        assert_eq!(pick_args("NONE", Kind::Badge), Ok(Pick::None));
        assert_eq!(pick_args("\"None\"", Kind::Paint), Ok(Pick::Named("None".into())));
        assert_eq!(pick_args("Forest Spring", Kind::Paint), Ok(Pick::Named("Forest Spring".into())));
        let id = "01JQM5X9N65Z6M8Q3Z6RV8BDT4";
        assert_eq!(pick_args(id, Kind::Badge), Ok(Pick::Named(id.into())));
        assert_eq!(pick_args("", Kind::Paint), Err("Say which paint, or none: /paint <name>.".into()));
        assert_eq!(pick_args("", Kind::Badge), Err("Say which badge, or none: /badge <name>.".into()));
    }
}
