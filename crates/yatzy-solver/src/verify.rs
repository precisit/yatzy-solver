//! Verification tools (F8): a brute-force reference solver for reduced games, the reduced games themselves,
//! and the published values.
//!
//! The brute-force solver shares nothing with [`crate::solver`] except the rules engine: it enumerates every
//! ordered outcome of every roll (6^m for m dice) and every keep by dice position (2^n per hand), and values
//! states by plain recursive expectimax over the rules engine's own [`State`] (with the upper total not capped).
//! Run in exact rational arithmetic, it must agree exactly with the fast solver run in the same arithmetic.

use std::collections::HashMap;

use crate::dice::Dice;
use crate::rules::State;
use crate::value::Value;
use crate::variant::{
    Category, CategoryKind, FullHouseScoring, JokerRule, OfAKindScoring, UpperBonus, Variant, VariantDef, YahtzeeBonus,
    face_mask,
};

/// A published value to reproduce (SPEC 5.3).
#[derive(Clone, Copy, Debug)]
pub struct PublishedValue {
    pub variant: &'static str,
    /// The expected final score under optimal play.
    pub expected: f64,
    /// The number of decimals published; a reproduction must round to the same digits.
    pub decimals: u32,
    pub source: &'static str,
}

/// The published values the solver must reproduce.
///
/// The published Scandinavian value 248.63 (Larsson and Sjöberg 2012) was computed with code that scores a
/// single pair in Two pairs; it reproduces under `+tp1`. Under the stated rules the value is 248.4399894, which
/// two independent open-source solvers confirm (see `docs/rules.md`).
pub const PUBLISHED: &[PublishedValue] = &[
    PublishedValue { variant: "yahtzee", expected: 254.5896, decimals: 4, source: "Verhoeff 1999; Glenn 2006" },
    PublishedValue { variant: "yahtzee+no-bonus", expected: 245.87, decimals: 2, source: "Verhoeff; Glenn 2006" },
    PublishedValue {
        variant: "yatzy-scandinavian+tp1",
        expected: 248.63,
        decimals: 2,
        source: "Larsson and Sjöberg 2012, as computed by their code",
    },
    PublishedValue {
        variant: "yatzy-scandinavian",
        expected: 248.44,
        decimals: 2,
        source: "Castux/yahtzee 248.4394; Laurii1i/Yatzy 248.44",
    },
];

impl PublishedValue {
    /// True when `value` rounds to the published digits.
    pub fn matches(&self, value: f64) -> bool {
        let scale = 10f64.powi(self.decimals as i32);
        (value * scale).round() == (self.expected * scale).round()
    }
}

/// The brute-force reference solver.
pub struct BruteForce<'a, T> {
    v: &'a Variant,
    states: HashMap<State, T>,
    rolls: HashMap<(State, Dice, u8), T>,
}

impl<'a, T: Value> BruteForce<'a, T> {
    pub fn new(v: &'a Variant) -> Self {
        BruteForce { v, states: HashMap::new(), rolls: HashMap::new() }
    }

    /// The expected remaining score from the start of a turn in state `s`.
    pub fn state_value(&mut self, s: &State) -> T {
        if self.v.is_over(s) {
            return T::zero();
        }
        if let Some(x) = self.states.get(s) {
            return x.clone();
        }
        let n = self.v.dice();
        let rolls_left = self.v.rolls() - 1;
        let x = self.expect_over_rolls(n, |bf, faces| {
            let dice = Dice::from_faces(faces).expect("valid faces");
            bf.roll_value(s, dice, rolls_left)
        });
        self.states.insert(*s, x.clone());
        x
    }

    /// The value of a situation: the best of scoring in every legal category and every keep by position.
    pub fn roll_value(&mut self, s: &State, dice: Dice, rolls_left: u8) -> T {
        if let Some(x) = self.rolls.get(&(*s, dice, rolls_left)) {
            return x.clone();
        }
        let v = self.v;
        let mut best: Option<T> = None;
        let mut consider = |x: T| match &mut best {
            None => best = Some(x),
            Some(b) => b.max_assign(&x),
        };
        for c in 0..v.num_categories() {
            if let Ok((next, scored)) = v.apply_score(s, &dice, c) {
                let mut x = self.state_value(&next);
                x.add_assign(&T::from_points(scored.total()));
                consider(x);
            }
        }
        if rolls_left > 0 {
            let faces = dice.faces();
            let n = faces.len();
            // Every keep by position, except keeping all the dice.
            for mask in 0..(1u32 << n) - 1 {
                let kept: Vec<u8> = (0..n).filter(|i| mask & (1 << i) != 0).map(|i| faces[i]).collect();
                let x = self.expect_over_rolls(n - kept.len(), |bf, rolled| {
                    let mut all = kept.clone();
                    all.extend_from_slice(rolled);
                    bf.roll_value(s, Dice::from_faces(&all).expect("valid faces"), rolls_left - 1)
                });
                consider(x);
            }
        }
        let x = best.expect("a legal category exists");
        self.rolls.insert((*s, dice, rolls_left), x.clone());
        x
    }

    /// The average of `f` over all 6^m ordered outcomes of rolling `m` dice.
    fn expect_over_rolls(&mut self, m: usize, mut f: impl FnMut(&mut Self, &[u8]) -> T) -> T {
        let total = 6u64.pow(m as u32);
        let mut sum = T::zero();
        let mut faces = vec![0u8; m];
        for code in 0..total {
            let mut c = code;
            for x in faces.iter_mut() {
                *x = (c % 6) as u8 + 1;
                c /= 6;
            }
            sum.add_assign(&f(self, &faces));
        }
        sum.div_int(total);
        sum
    }

    /// Every state valued so far (the states reachable from those asked for), with its value.
    pub fn states(&self) -> impl Iterator<Item = (&State, &T)> {
        self.states.iter()
    }
}

fn cat(id: &str, kind: CategoryKind) -> Category {
    Category { id: id.into(), name: id.into(), kind }
}

fn upper(id: &str, face: u8) -> Category {
    cat(id, CategoryKind::Upper { face })
}

/// Reduced games for the brute-force cross-check. Together they exercise every category kind, both upper
/// bonus paths, every joker rule, forced order and both house-rule switches, with 2 to 4 dice.
pub fn reduced_variants() -> Vec<Variant> {
    use CategoryKind::*;
    let def = |id: &str, dice: u8, rolls: u8, categories: Vec<Category>| VariantDef {
        id: id.into(),
        name: id.into(),
        dice,
        rolls,
        categories,
        upper_bonus: None,
        yahtzee_bonus: None,
        forced_order: false,
    };
    let mut out = Vec::new();

    // Two dice: upper section with a reachable bonus, chance, a pair.
    let mut d = def(
        "reduced-upper",
        2,
        3,
        vec![
            upper("ones", 1),
            upper("twos", 2),
            upper("threes", 3),
            cat("one_pair", OfAKind { n: 2, scoring: OfAKindScoring::Matched }),
            cat("chance", Chance),
        ],
    );
    d.upper_bonus = Some(UpperBonus { threshold: 7, points: 10 });
    out.push(d.clone());
    d.id = "reduced-upper+forced".into();
    d.forced_order = true;
    out.push(d);

    // Three dice, Scandinavian-style lower section.
    let mut d = def(
        "reduced-scandinavian",
        3,
        3,
        vec![
            upper("fives", 5),
            upper("sixes", 6),
            cat("three_of_a_kind", OfAKind { n: 3, scoring: OfAKindScoring::Matched }),
            cat("small_straight", Straight { patterns: vec![face_mask(&[1, 2, 3])], points: 15 }),
            cat("full_house", FullHouse { scoring: FullHouseScoring::AllDice, five_of_a_kind_counts: true }),
            cat("yatzy", AllSame { points: 50 }),
        ],
    );
    d.upper_bonus = Some(UpperBonus { threshold: 20, points: 25 });
    out.push(d);

    // Four dice: two pairs (with four of a kind or a single pair counting), four of a kind, American-style
    // straights.
    for (suffix, four, single) in [("tp4", true, false), ("tp1", false, true)] {
        out.push(def(
            &format!("reduced-pairs+{suffix}"),
            4,
            2,
            vec![
                upper("twos", 2),
                cat("two_pairs", TwoPairs { four_of_a_kind_counts: four, single_pair_counts: single }),
                cat("four_of_a_kind", OfAKind { n: 4, scoring: OfAKindScoring::AllDice }),
                cat(
                    "straight",
                    Straight {
                        patterns: vec![face_mask(&[1, 2, 3]), face_mask(&[2, 3, 4]), face_mask(&[4, 5, 6])],
                        points: 20,
                    },
                ),
            ],
        ));
    }

    // Two dice, American rules: Yahtzee bonus with every joker rule.
    for (suffix, joker) in [("free", JokerRule::Free), ("forced", JokerRule::Forced), ("none", JokerRule::None)] {
        let mut d = def(
            &format!("reduced-american+{suffix}"),
            2,
            3,
            vec![
                upper("ones", 1),
                upper("twos", 2),
                upper("sixes", 6),
                cat("three_of_a_kind", OfAKind { n: 2, scoring: OfAKindScoring::AllDice }),
                cat("full_house", FullHouse { scoring: FullHouseScoring::Fixed(25), five_of_a_kind_counts: false }),
                cat("straight", Straight { patterns: vec![face_mask(&[5, 6])], points: 30 }),
                cat("yahtzee", AllSame { points: 50 }),
            ],
        );
        d.upper_bonus = Some(UpperBonus { threshold: 14, points: 35 });
        d.yahtzee_bonus = Some(YahtzeeBonus { points: 100, joker });
        out.push(d);
    }

    out.into_iter().map(|d| Variant::new(d).expect("reduced variant is valid")).collect()
}
