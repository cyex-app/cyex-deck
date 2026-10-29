//! V108-C01 One line budget for every chat no view shows: hidden tabs keep their whole
//! scrollback and closed chats stay parked (Chatterino keeps a channel's lines while the
//! app runs), until all of them together pass `LINE_BUDGET`. Then the oldest parked chats
//! go first, then the hidden ones shown longest ago shrink to `BACKGROUND`.

use super::keep::BACKGROUND;

/// Lines kept over all hidden and parked chats.
pub const LINE_BUDGET: usize = 150_000;
/// Closed chats kept at most, however few lines they have.
pub const PARKED_MAX: usize = 32;

/// A chat no view shows.
#[derive(Clone, Copy, Debug)]
pub struct Held {
    pub lines: usize,
    /// When a view last showed it (ms); for a parked chat, when it was closed.
    pub last_shown: i64,
    pub parked: bool,
    /// Already shrunk to `BACKGROUND`.
    pub small: bool,
}

/// What to do with `rooms[i]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cut {
    /// A hidden chat drops to `BACKGROUND` lines.
    Shrink(usize),
    /// A parked chat is forgotten.
    Drop(usize),
}

/// The cuts that bring `rooms` under `budget` lines and `parked_max` parked chats, oldest
/// first. Under both: none.
pub fn cuts(rooms: &[Held], budget: usize, parked_max: usize) -> Vec<Cut> {
    let mut total: usize = rooms.iter().map(|r| r.lines).sum();
    let mut parked: Vec<usize> = (0..rooms.len()).filter(|&i| rooms[i].parked).collect();
    parked.sort_by_key(|&i| rooms[i].last_shown);
    let mut hidden: Vec<usize> = (0..rooms.len())
        .filter(|&i| !rooms[i].parked && !rooms[i].small && rooms[i].lines > BACKGROUND)
        .collect();
    hidden.sort_by_key(|&i| rooms[i].last_shown);
    let mut out = Vec::new();
    let mut left = parked.len();
    for i in parked {
        if left <= parked_max && total <= budget {
            break;
        }
        out.push(Cut::Drop(i));
        total -= rooms[i].lines;
        left -= 1;
    }
    for i in hidden {
        if total <= budget {
            break;
        }
        out.push(Cut::Shrink(i));
        total -= rooms[i].lines - BACKGROUND;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hidden(lines: usize, last_shown: i64) -> Held {
        Held {
            lines,
            last_shown,
            parked: false,
            small: false,
        }
    }

    fn parked(lines: usize, last_shown: i64) -> Held {
        Held {
            lines,
            last_shown,
            parked: true,
            small: false,
        }
    }

    #[test]
    fn under_the_budget_nothing_is_cut() {
        let rooms = [hidden(1000, 5), parked(1000, 1)];
        assert!(cuts(&rooms, 2000, 4).is_empty());
        assert!(cuts(&[], 0, 0).is_empty());
    }

    #[test]
    fn parked_chats_go_first_oldest_first() {
        let rooms = [hidden(1000, 1), parked(1000, 9), parked(1000, 3)];
        assert_eq!(cuts(&rooms, 2000, 4), [Cut::Drop(2)]);
        assert_eq!(cuts(&rooms, 1000, 4), [Cut::Drop(2), Cut::Drop(1)]);
    }

    #[test]
    fn then_hidden_chats_shown_longest_ago_shrink() {
        let rooms = [hidden(1000, 7), hidden(1000, 2), parked(500, 9), hidden(1000, 5)];
        // 3500 → drop the parked (3000) → shrink the one shown at 2 (2150).
        assert_eq!(cuts(&rooms, 2500, 4), [Cut::Drop(2), Cut::Shrink(1)]);
        let all = cuts(&rooms, 0, 4);
        assert_eq!(
            all,
            [Cut::Drop(2), Cut::Shrink(1), Cut::Shrink(3), Cut::Shrink(0)]
        );
    }

    #[test]
    fn small_chats_and_ones_under_the_floor_are_left_alone() {
        let small = Held {
            small: true,
            ..hidden(BACKGROUND, 1)
        };
        let rooms = [small, hidden(100, 2), hidden(1000, 3)];
        assert_eq!(cuts(&rooms, 0, 4), [Cut::Shrink(2)]);
    }

    #[test]
    fn too_many_parked_chats_drop_the_oldest() {
        let rooms = [parked(10, 4), parked(10, 1), parked(10, 3), hidden(10, 0)];
        assert_eq!(cuts(&rooms, LINE_BUDGET, 2), [Cut::Drop(1)]);
        assert_eq!(
            cuts(&rooms, LINE_BUDGET, 0),
            [Cut::Drop(1), Cut::Drop(2), Cut::Drop(0)]
        );
    }
}
