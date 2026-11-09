//! V110-U68 Who gets the watch slots now. A live channel that can earn a badge beats
//! "Mine first" and the rules: while one is up, only that channel is watched (the badge is
//! worth it, and every minute it gets counts). Otherwise the usual picks (`slots::pick`).

use cyex_settings::schema::Rule;

use crate::slots::{self, Candidate};

/// What to watch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// Channel ids to send minutes for.
    pub slots: Vec<String>,
    /// The badge channel among them, when a badge is being earned.
    pub focus: Option<String>,
}

/// `targets` are the channels that can earn a badge, best first; `free` is how many
/// slots the miner may use (the player's own streams take the rest, W1). With no free
/// slot nothing is watched and nothing is focused: the badge waits.
pub fn plan(cands: &[Candidate], targets: &[String], first: &[String], rules: &[Rule], free: usize) -> Plan {
    if free == 0 {
        return Plan::default();
    }
    let focus = targets
        .iter()
        .find(|t| cands.iter().any(|c| c.ready && &c.id == *t))
        .cloned();
    match focus {
        Some(id) => Plan {
            slots: vec![id.clone()],
            focus: Some(id),
        },
        None => Plan {
            slots: slots::pick(cands, first, rules, free),
            focus: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: &str, ready: bool) -> Candidate {
        Candidate {
            id: id.into(),
            ready,
            ..Candidate::default()
        }
    }

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_badge_channel_beats_first_and_rules() {
        let list = [c("a", true), c("b", true), c("badge", true)];
        let p = plan(&list, &ids(&["badge"]), &ids(&["a"]), &[Rule::Streak], 2);
        assert_eq!(p.slots, ["badge"]);
        assert_eq!(p.focus.as_deref(), Some("badge"));
        // One slot free: still the badge.
        assert_eq!(plan(&list, &ids(&["badge"]), &[], &[], 1).slots, ["badge"]);
    }

    #[test]
    fn the_first_ready_target_in_target_order_wins() {
        let list = [c("a", true), c("b", false), c("c", true)];
        let p = plan(&list, &ids(&["b", "c", "a"]), &[], &[], 2);
        assert_eq!(p.focus.as_deref(), Some("c"));
    }

    #[test]
    fn no_free_slot_means_no_focus_and_nothing_watched() {
        let list = [c("a", true), c("badge", true)];
        assert_eq!(
            plan(&list, &ids(&["badge"]), &ids(&["a"]), &[], 0),
            Plan::default()
        );
    }

    #[test]
    fn without_a_live_target_the_usual_picks_run() {
        let list = [c("a", true), c("b", true), c("badge", false)];
        let p = plan(&list, &ids(&["badge", "not-listed"]), &ids(&["b"]), &[], 2);
        assert_eq!(p.slots, slots::pick(&list, &ids(&["b"]), &[], 2));
        assert_eq!(
            (p.slots.as_slice(), p.focus),
            (["b".to_owned(), "a".to_owned()].as_slice(), None)
        );
        let p = plan(&list, &[], &[], &[], 1);
        assert_eq!(p.slots, ["a"]);
    }
}
