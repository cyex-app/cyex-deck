//! V1051-04 The 7TV variables in custom-command bodies (`{paint}`, `{combo:Night}`…), as
//! one run sees them. Nothing here calls 7TV without the account's consent.

use std::collections::HashSet;
use std::time::Duration;

use cyex_chat::commands::Var;
use cyex_emotes::seventv::account::model::Inventory;
use cyex_settings::schema::{Combo, ComboItem};
use tauri::AppHandle;

use super::apply::{Part, Want, apply};
use super::{SelectError, combos, has_consent, lookup, read, words};

const READ_WAIT: Duration = Duration::from_secs(3);
const APPLY_WAIT: Duration = Duration::from_secs(10);

/// One custom-command run's view of the 7TV variables. Lives in `AppHost`.
pub struct VarScope {
    pub account: Option<String>,
    /// An automation's run (S4): other people's messages start it, so `{combo:…}` puts
    /// nothing on there and expands to "".
    automation: bool,
    applied: HashSet<String>,
    /// Lines for the user only, shown after the run.
    pub notes: Vec<String>,
}

/// The read variables, from an inventory and the saved combos. `ComboApply` isn't read.
pub fn pick(inv: &Inventory, combos: &[Combo], v: &Var) -> String {
    let (paint, badge) = lookup::worn(inv);
    let name = |i: Option<ComboItem>| i.map(|i| i.name);
    let id = |i: Option<ComboItem>| i.map(|i| i.id);
    match v {
        Var::Paint | Var::PaintName => name(paint),
        Var::PaintId => id(paint),
        Var::Badge | Var::BadgeName => name(badge),
        Var::BadgeId => id(badge),
        Var::Combo => lookup::current(combos, inv).map(|c| c.name.clone()),
        Var::ComboApply(_) => None,
    }
    .unwrap_or_default()
}

/// One apply per run and combo name: true the first time.
fn first_time(applied: &mut HashSet<String>, name: &str) -> bool {
    applied.insert(name.to_lowercase())
}

fn clean(p: &Part) -> bool {
    matches!(p, Part::Changed | Part::Same | Part::Overtaken)
}

impl VarScope {
    pub fn new(account: Option<String>, automation: bool) -> Self {
        Self {
            account,
            automation,
            applied: HashSet::new(),
            notes: Vec::new(),
        }
    }

    pub fn resolve(&mut self, app: &AppHandle, rt: &tokio::runtime::Handle, v: &Var) -> String {
        let Some(account) = self.account.clone() else {
            return String::new();
        };
        if !has_consent(app, &account) {
            return String::new();
        }
        if let Var::ComboApply(_) = v {
            return match self.apply_name(v) {
                Some(name) => self.combo(app, rt, &account, &name),
                None => String::new(),
            };
        }
        match rt.block_on(tokio::time::timeout(READ_WAIT, read(app, &account))) {
            Ok(Ok(inv)) => pick(&inv, &combos::list(app, &account), v),
            _ => String::new(),
        }
    }

    /// The combo `{combo:<name>}` may put on: the only way to `apply` from a variable.
    /// None for the read forms, an empty name and inside automations.
    fn apply_name(&self, v: &Var) -> Option<String> {
        match v {
            Var::ComboApply(name) if !self.automation && !name.trim().is_empty() => Some(name.clone()),
            _ => None,
        }
    }

    fn combo(&mut self, app: &AppHandle, rt: &tokio::runtime::Handle, account: &str, name: &str) -> String {
        let Some(combo) = (!name.trim().is_empty())
            .then(|| combos::find(app, account, name))
            .flatten()
        else {
            return String::new();
        };
        if !first_time(&mut self.applied, &combo.name) {
            return combo.name;
        }
        let want = Want {
            paint: combo.paint.clone(),
            badge: combo.badge.clone(),
        };
        let done = rt.block_on(tokio::time::timeout(
            APPLY_WAIT,
            apply(app, account, want, Some(&combo.name)),
        ));
        let err = match done {
            Ok(Ok(a)) => {
                if !(clean(&a.paint) && clean(&a.badge)) {
                    self.notes.push(a.line);
                }
                return combo.name;
            }
            Ok(Err(e)) => e,
            Err(_) => SelectError::Down,
        };
        self.notes
            .push(format!("{{combo:{}}}: {}", combo.name, words::select_error(&err)));
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::lookup::tests::inv;
    use super::*;

    fn combo() -> Combo {
        let it = |id: &str| ComboItem {
            id: id.into(),
            name: id.into(),
        };
        Combo {
            id: "c".into(),
            name: "Night".into(),
            paint: Some(it("P1")),
            badge: Some(it("B1")),
        }
    }

    #[test]
    fn read_variables() {
        let i = inv();
        let list = [combo()];
        let g = |v| pick(&i, &list, &v);
        assert_eq!(g(Var::Paint), "Gold");
        assert_eq!(g(Var::PaintName), "Gold");
        assert_eq!(g(Var::PaintId), "P1");
        assert_eq!(g(Var::Badge), "Star");
        assert_eq!(g(Var::BadgeName), "Star");
        assert_eq!(g(Var::BadgeId), "B1");
        assert_eq!(g(Var::Combo), "Night");
        assert_eq!(g(Var::ComboApply("Night".into())), "");
    }

    #[test]
    fn nothing_worn_is_empty() {
        let mut i = inv();
        i.paint = None;
        i.badge = None;
        for v in [Var::Paint, Var::PaintId, Var::Badge, Var::BadgeId, Var::Combo] {
            assert_eq!(pick(&i, &[], &v), "");
        }
    }

    #[test]
    fn automations_never_apply() {
        let night = Var::ComboApply("Night".into());
        let auto = VarScope::new(Some("1".into()), true);
        assert_eq!(auto.apply_name(&night), None);
        let typed = VarScope::new(Some("1".into()), false);
        assert_eq!(typed.apply_name(&night).as_deref(), Some("Night"));
        assert_eq!(typed.apply_name(&Var::ComboApply(" ".into())), None);
        assert_eq!(typed.apply_name(&Var::Paint), None);
    }

    #[test]
    fn one_apply_per_name_per_run() {
        let mut set = HashSet::new();
        assert!(first_time(&mut set, "Night"));
        assert!(!first_time(&mut set, "NIGHT"));
        assert!(first_time(&mut set, "Day"));
    }
}
