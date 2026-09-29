//! Dice as sorted multisets.
//!
//! A [`Dice`] value is a multiset of faces 1 to 6, stored as a count per face. The order of the physical dice
//! never matters to the rules or the solver, so every hand, keep and reroll outcome is a multiset.

use std::fmt;

/// Number of faces on a die.
pub const FACES: usize = 6;

/// Largest number of dice any variant may use.
pub const MAX_DICE: usize = 6;

/// A multiset of dice faces (1 to 6), stored as a count per face.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Dice {
    counts: [u8; FACES],
}

/// Error returned when dice cannot be built from the given faces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiceError {
    /// A face outside 1 to 6.
    BadFace(u8),
    /// More than [`MAX_DICE`] dice.
    TooMany(usize),
}

impl fmt::Display for DiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiceError::BadFace(v) => write!(f, "die face {v} is not in 1 to 6"),
            DiceError::TooMany(n) => write!(f, "{n} dice is more than the maximum of {MAX_DICE}"),
        }
    }
}

impl std::error::Error for DiceError {}

impl Dice {
    /// No dice.
    pub const EMPTY: Dice = Dice { counts: [0; FACES] };

    /// Builds a multiset from faces in any order.
    pub fn from_faces(faces: &[u8]) -> Result<Dice, DiceError> {
        if faces.len() > MAX_DICE {
            return Err(DiceError::TooMany(faces.len()));
        }
        let mut counts = [0u8; FACES];
        for &v in faces {
            if !(1..=6).contains(&v) {
                return Err(DiceError::BadFace(v));
            }
            counts[usize::from(v - 1)] += 1;
        }
        Ok(Dice { counts })
    }

    /// Builds a multiset from a count per face (index 0 is face 1).
    pub fn from_counts(counts: [u8; FACES]) -> Result<Dice, DiceError> {
        let n: usize = counts.iter().map(|&c| usize::from(c)).sum();
        if n > MAX_DICE {
            return Err(DiceError::TooMany(n));
        }
        Ok(Dice { counts })
    }

    /// The count per face; index 0 is face 1.
    pub fn counts(&self) -> [u8; FACES] {
        self.counts
    }

    /// How many dice show `face` (1 to 6).
    pub fn count(&self, face: u8) -> u8 {
        self.counts[usize::from(face - 1)]
    }

    /// The number of dice.
    pub fn len(&self) -> usize {
        self.counts.iter().map(|&c| usize::from(c)).sum()
    }

    /// True when there are no dice.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The faces in ascending order.
    pub fn faces(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.len());
        for (i, &c) in self.counts.iter().enumerate() {
            for _ in 0..c {
                out.push(i as u8 + 1);
            }
        }
        out
    }

    /// The sum of the faces.
    pub fn sum(&self) -> u16 {
        self.counts.iter().enumerate().map(|(i, &c)| (i as u16 + 1) * u16::from(c)).sum()
    }

    /// A bitmask of the faces present: bit `f - 1` is set when face `f` shows.
    pub fn face_mask(&self) -> u8 {
        let mut m = 0;
        for (i, &c) in self.counts.iter().enumerate() {
            if c > 0 {
                m |= 1 << i;
            }
        }
        m
    }

    /// True when every die in `self` is also in `other` (as multisets).
    pub fn is_subset_of(&self, other: &Dice) -> bool {
        self.counts.iter().zip(other.counts.iter()).all(|(a, b)| a <= b)
    }

    /// The multiset union (sum) of two hands.
    pub fn plus(&self, other: &Dice) -> Result<Dice, DiceError> {
        let mut counts = self.counts;
        for (c, o) in counts.iter_mut().zip(other.counts.iter()) {
            *c += o;
        }
        Dice::from_counts(counts)
    }

    /// The multiset difference `self - other`, or `None` when `other` is not a subset of `self`.
    pub fn minus(&self, other: &Dice) -> Option<Dice> {
        if !other.is_subset_of(self) {
            return None;
        }
        let mut counts = self.counts;
        for (c, o) in counts.iter_mut().zip(other.counts.iter()) {
            *c -= o;
        }
        Some(Dice { counts })
    }

    /// Adds one die showing `face`.
    pub fn with(&self, face: u8) -> Dice {
        let mut counts = self.counts;
        counts[usize::from(face - 1)] += 1;
        Dice { counts }
    }

    /// True when there is at least one die and all dice show the same face.
    pub fn all_same(&self) -> bool {
        self.counts.iter().filter(|&&c| c > 0).count() == 1
    }

    /// Every distinct sub-multiset, including the empty one and `self`, in [`all_multisets`] order by size and
    /// then lexicographically.
    pub fn sub_multisets(&self) -> Vec<Dice> {
        let mut out = vec![Dice::EMPTY];
        for face in 0..FACES {
            let base = out.len();
            for c in 1..=self.counts[face] {
                for i in 0..base {
                    let mut d = out[i];
                    d.counts[face] = c;
                    out.push(d);
                }
            }
        }
        out.sort_by(cmp_multisets);
        out
    }

    /// The number of distinct ordered outcomes that give this multiset when `self.len()` dice are rolled: the
    /// multinomial coefficient n! / (c1! ... c6!). The probability of the multiset is this divided by 6^n.
    pub fn arrangements(&self) -> u64 {
        let mut num = factorial(self.len());
        for &c in &self.counts {
            num /= factorial(usize::from(c));
        }
        num
    }

    /// A compact key: the counts packed in three bits each.
    pub fn key(&self) -> u32 {
        self.counts.iter().enumerate().map(|(i, &c)| u32::from(c) << (3 * i)).sum()
    }
}

fn factorial(n: usize) -> u64 {
    (1..=n as u64).product()
}

/// Canonical order: by size, then by the ascending face sequence, lexicographically.
pub fn cmp_multisets(a: &Dice, b: &Dice) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| a.faces().cmp(&b.faces()))
}

/// All multisets of exactly `n` dice, in lexicographic order of their ascending face sequences.
///
/// There are 252 for five dice.
pub fn all_multisets(n: usize) -> Vec<Dice> {
    fn rec(n: usize, min_face: u8, cur: &mut Vec<u8>, out: &mut Vec<Dice>) {
        if cur.len() == n {
            out.push(Dice::from_faces(cur).expect("valid faces"));
            return;
        }
        for v in min_face..=6 {
            cur.push(v);
            rec(n, v, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    rec(n, 1, &mut Vec::with_capacity(n), &mut out);
    out
}

/// All multisets of at most `n` dice, by size and then lexicographically. There are 462 for `n = 5`.
pub fn all_multisets_up_to(n: usize) -> Vec<Dice> {
    (0..=n).flat_map(all_multisets).collect()
}

impl fmt::Display for Dice {
    /// Faces in ascending order separated by spaces, e.g. `1 3 3 5 6`; `-` for no dice.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("-");
        }
        let faces = self.faces();
        for (i, v) in faces.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            write!(f, "{v}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Dice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dice[{self}]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiset_counts() {
        assert_eq!(all_multisets(5).len(), 252);
        assert_eq!(all_multisets_up_to(5).len(), 462);
        assert_eq!(all_multisets(0), vec![Dice::EMPTY]);
    }

    #[test]
    fn probabilities_sum_to_one() {
        for n in 0..=MAX_DICE {
            let total: u64 = all_multisets(n).iter().map(Dice::arrangements).sum();
            assert_eq!(total, 6u64.pow(n as u32));
        }
    }

    #[test]
    fn sub_multisets_are_distinct_subsets() {
        let d = Dice::from_faces(&[3, 3, 5, 1, 6]).unwrap();
        let subs = d.sub_multisets();
        assert_eq!(subs.len(), 2 * 3 * 2 * 2);
        for s in &subs {
            assert!(s.is_subset_of(&d));
        }
        let mut keys: Vec<u32> = subs.iter().map(Dice::key).collect();
        keys.dedup();
        assert_eq!(keys.len(), subs.len());
        assert_eq!(subs[0], Dice::EMPTY);
        assert_eq!(*subs.last().unwrap(), d);
    }

    #[test]
    fn display_is_sorted() {
        let d = Dice::from_faces(&[6, 1, 3, 5, 3]).unwrap();
        assert_eq!(d.to_string(), "1 3 3 5 6");
        assert_eq!(Dice::EMPTY.to_string(), "-");
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(Dice::from_faces(&[0]), Err(DiceError::BadFace(0)));
        assert_eq!(Dice::from_faces(&[7]), Err(DiceError::BadFace(7)));
        assert_eq!(Dice::from_faces(&[1; 7]), Err(DiceError::TooMany(7)));
    }
}
