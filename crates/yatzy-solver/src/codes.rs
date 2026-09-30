//! The action code space: a fixed integer code for every action that can be legal in a variant, for batch
//! arrays and learners with a fixed output layout. Part of the stable notation contract (`docs/notation.md`).
//!
//! - categories are `0 .. C-1`, in score-card order;
//! - keeps are `C + k`, where `k` indexes the multisets of 0 to `n - 1` dice (keeping all `n` is never legal) by
//!   size and then lexicographically: `keep -` is `C`, `keep 1` is `C + 1`, ..., `keep 6 6 6 6` is `C + 209` for
//!   five dice.
//!
//! There are `C + 210` codes for five dice. A situation has at most `C + 2^n - 1` legal actions (every category,
//! and the 31 proper sub-multisets of five distinct dice): the width of the flat batch layout.

use crate::dice::Dice;
use crate::rules::Action;
use crate::variant::Variant;

impl Variant {
    /// The number of action codes: categories plus keeps.
    pub fn num_action_codes(&self) -> usize {
        self.num_categories() + self.keep_codes().len()
    }

    /// The largest number of legal actions in one situation: `C + 2^n - 1`.
    pub fn max_options(&self) -> usize {
        self.num_categories() + (1 << self.dice()) - 1
    }

    /// The code of an action, or `None` for a keep that can never be legal (all the dice, or too many).
    pub fn action_code(&self, a: &Action) -> Option<u16> {
        match a {
            Action::Score(c) => (*c < self.num_categories()).then_some(*c as u16),
            Action::Keep(k) => self.keep_code_index(k).map(|i| (self.num_categories() + i) as u16),
        }
    }

    /// The action with a code.
    pub fn action_from_code(&self, code: u16) -> Option<Action> {
        let code = usize::from(code);
        let c = self.num_categories();
        if code < c { Some(Action::Score(code)) } else { self.keep_codes().get(code - c).map(|&k| Action::Keep(k)) }
    }

    fn keep_code_index(&self, k: &Dice) -> Option<usize> {
        if k.len() >= self.dice() {
            return None;
        }
        // Keeps are ordered by size, then lexicographically; binary search within the size block.
        let codes = self.keep_codes();
        let start = codes.partition_point(|d| d.len() < k.len());
        let end = codes.partition_point(|d| d.len() <= k.len());
        codes[start..end].binary_search_by(|d| crate::dice::cmp_faces(d, k)).ok().map(|i| start + i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_space() {
        for v in [Variant::scandinavian(), Variant::american()] {
            let c = v.num_categories();
            assert_eq!(v.num_action_codes(), c + 210);
            assert_eq!(v.max_options(), c + 31);
            for code in 0..v.num_action_codes() as u16 {
                let a = v.action_from_code(code).unwrap();
                assert_eq!(v.action_code(&a), Some(code));
            }
            assert_eq!(v.action_from_code(v.num_action_codes() as u16), None);
            assert_eq!(v.action_code(&Action::Keep(Dice::EMPTY)), Some(c as u16));
            assert_eq!(v.action_code(&Action::Keep(Dice::from_faces(&[1]).unwrap())), Some(c as u16 + 1));
            assert_eq!(v.action_code(&Action::Keep(Dice::from_faces(&[6, 6, 6, 6]).unwrap())), Some(c as u16 + 209));
            assert_eq!(v.action_code(&Action::Keep(Dice::from_faces(&[6; 5]).unwrap())), None);
        }
    }
}
