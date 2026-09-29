//! The exact solver: backward induction over the states between turns (SPEC 3).
//!
//! The stored table holds, for every state, V(state): the expected remaining score from the start of a turn
//! under optimal play (maximizing the expected final score). Within a turn, values are derived from V:
//!
//! 1. each final roll is valued by its best category;
//! 2. each keep is valued from the values of the rolls it can lead to, with the incremental recursion
//!    K(k) = (1/6) (K(k + 1) + ... + K(k + 6)) from keeps of one die more, down to the empty keep;
//! 3. each earlier roll is valued by its best keep, and steps 2 and 3 repeat for the earlier rolls;
//! 4. V(state) is the value of the empty keep before the first roll.
//!
//! Keeping all the dice is included in step 3 (it equals the value of the same dice with one roll fewer, which
//! covers scoring now), so the values agree with the rules engine, where keeping all dice is not an action.
//!
//! The summation order is fixed, so an `f64` build gives bit-identical tables on every platform.

use std::collections::HashMap;

use crate::dice::{Dice, all_multisets_up_to};
use crate::rules::{Action, State};
use crate::value::Value;
use crate::variant::Variant;

/// The layout of the table: the states between turns and their order.
///
/// A state is (the filled mask, the upper total capped at the bonus threshold, whether the Yahtzee bonus is
/// armed). Its index is `((mask * upper_values) + min(upper, cap)) * armed_values + armed`, where
/// `upper_values` is the threshold plus one (1 without an upper bonus) and `armed_values` is 2 for variants
/// with a Yahtzee bonus and 1 otherwise. Every combination has an entry, reachable or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateSpace {
    pub categories: usize,
    pub upper_values: usize,
    pub armed_values: usize,
}

impl StateSpace {
    pub fn of(v: &Variant) -> StateSpace {
        StateSpace {
            categories: v.num_categories(),
            upper_values: usize::from(v.upper_cap()) + 1,
            armed_values: if v.all_same_bonus().is_some() { 2 } else { 1 },
        }
    }

    /// The number of states.
    pub fn len(&self) -> usize {
        (1usize << self.categories) * self.upper_values * self.armed_values
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    /// The index of a state (the upper total is capped here).
    #[inline]
    pub fn index(&self, s: &State) -> usize {
        let up = usize::from(s.upper).min(self.upper_values - 1);
        let armed = usize::from(s.bonus_armed && self.armed_values == 2);
        ((s.filled as usize * self.upper_values) + up) * self.armed_values + armed
    }

    /// The state at an index (with the upper total capped).
    pub fn state(&self, index: usize) -> State {
        let armed = index % self.armed_values;
        let rest = index / self.armed_values;
        State {
            filled: (rest / self.upper_values) as u32,
            upper: (rest % self.upper_values) as u16,
            bonus_armed: armed == 1,
        }
    }
}

/// Precomputed tables for the within-turn computation of one variant.
#[derive(Clone, Debug)]
pub struct TurnModel {
    variant: Variant,
    space: StateSpace,
    /// All multisets of at most `n` dice, by size and then lexicographically. The last `rolls` entries are the
    /// full hands.
    keeps: Vec<Dice>,
    first_roll: usize,
    /// For each keep smaller than a full hand, the keep with one more die of each face.
    child: Vec<[u16; 6]>,
    /// For each full hand, the indices of all its sub-multisets (itself included), flattened.
    subs: Vec<u16>,
    subs_off: Vec<u32>,
    /// Arrangements of each full hand (its probability times 6^n).
    arrangements: Vec<u64>,
    /// `scores[c * hands + r]`: the normal score of hand `r` in category `c`.
    scores: Vec<u16>,
    /// Keep index by [`Dice::key`].
    index_of: HashMap<u32, u16>,
}

/// Per-thread scratch buffers.
struct Scratch<T> {
    e: Vec<T>,
    k: Vec<T>,
}

impl TurnModel {
    pub fn new(variant: &Variant) -> TurnModel {
        let n = variant.dice();
        let keeps = all_multisets_up_to(n);
        let first_roll = keeps.iter().position(|d| d.len() == n).expect("full hands exist");
        let index_of: HashMap<u32, u16> = keeps.iter().enumerate().map(|(i, d)| (d.key(), i as u16)).collect();
        let child =
            keeps[..first_roll].iter().map(|k| std::array::from_fn(|f| index_of[&k.with(f as u8 + 1).key()])).collect();
        let mut subs = Vec::new();
        let mut subs_off = vec![0u32];
        for r in &keeps[first_roll..] {
            subs.extend(r.sub_multisets().iter().map(|s| index_of[&s.key()]));
            subs_off.push(subs.len() as u32);
        }
        let hands = keeps.len() - first_roll;
        let mut scores = Vec::with_capacity(variant.num_categories() * hands);
        for c in 0..variant.num_categories() {
            scores.extend(keeps[first_roll..].iter().map(|r| variant.score(c, r)));
        }
        let arrangements = keeps[first_roll..].iter().map(Dice::arrangements).collect();
        TurnModel {
            variant: variant.clone(),
            space: StateSpace::of(variant),
            keeps,
            first_roll,
            child,
            subs,
            subs_off,
            arrangements,
            scores,
            index_of,
        }
    }

    pub fn variant(&self) -> &Variant {
        &self.variant
    }

    pub fn space(&self) -> StateSpace {
        self.space
    }

    /// The full hands, in the order used by [`TurnValues`].
    pub fn hands(&self) -> &[Dice] {
        &self.keeps[self.first_roll..]
    }

    /// All keeps (multisets of at most `n` dice), in the order used by [`TurnValues`].
    pub fn keeps(&self) -> &[Dice] {
        &self.keeps
    }

    /// The index of a keep in [`TurnModel::keeps`].
    pub fn keep_index(&self, d: &Dice) -> Option<usize> {
        self.index_of.get(&d.key()).map(|&i| usize::from(i))
    }

    /// The index of a full hand in [`TurnModel::hands`].
    pub fn hand_index(&self, d: &Dice) -> Option<usize> {
        self.keep_index(d).and_then(|i| i.checked_sub(self.first_roll))
    }

    fn scratch<T: Value>(&self) -> Scratch<T> {
        Scratch { e: vec![T::zero(); self.keeps.len() - self.first_roll], k: vec![T::zero(); self.keeps.len()] }
    }

    /// The value of scoring hand `r` in category `c` in state `s` under the normal rules (no extra Yahtzee):
    /// the points earned plus the value of the next state.
    #[inline]
    fn fast_score_value<T: Value>(&self, s: &State, c: usize, r: usize, table: &[T]) -> T {
        let hands = self.keeps.len() - self.first_roll;
        let p = self.scores[c * hands + r];
        let v = &self.variant;
        let mut gain = p;
        let mut next = State { filled: s.filled | 1 << c, ..*s };
        if v.upper_mask() & (1 << c) != 0 {
            if let Some(b) = v.upper_bonus()
                && s.upper < b.threshold
                && s.upper + p >= b.threshold
            {
                gain += b.points;
            }
            next.upper = (s.upper + p).min(v.upper_cap());
        }
        if Some(c) == v.all_same_box() {
            next.bonus_armed = p > 0;
        }
        let mut val = table[self.space.index(&next)].clone();
        val.add_assign(&T::from_points(gain));
        val
    }

    /// The value of the best category for each full hand, after the last roll (step 1).
    fn final_values<T: Value>(&self, s: &State, table: &[T], e: &mut [T]) {
        let v = &self.variant;
        let open = v.all_mask() & !s.filled;
        let legal = if v.forced_order() { open & open.wrapping_neg() } else { open };
        let mut first = true;
        let mut bits = legal;
        while bits != 0 {
            let c = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            for (r, slot) in e.iter_mut().enumerate() {
                let val = self.fast_score_value(s, c, r, table);
                if first {
                    *slot = val;
                } else {
                    slot.max_assign(&val);
                }
            }
            first = false;
        }
        // Extra Yahtzees (bonus, joker): take the rules engine's word for legality and points.
        if v.all_same_box().is_some_and(|y| s.is_filled(y)) {
            for (r, hand) in self.hands().iter().enumerate() {
                if hand.all_same() {
                    e[r] = self.score_value_by_rules(s, hand, table);
                }
            }
        }
    }

    /// The best value of scoring `hand` in state `s`, using the rules engine for legality and points.
    fn score_value_by_rules<T: Value>(&self, s: &State, hand: &Dice, table: &[T]) -> T {
        let v = &self.variant;
        let mut best: Option<T> = None;
        for sc in v.score_choices(s, hand).expect("state is not final") {
            let (next, _) = v.apply_score(s, hand, sc.category).expect("legal choice");
            let mut val = table[self.space.index(&next)].clone();
            val.add_assign(&T::from_points(sc.total()));
            match &mut best {
                None => best = Some(val),
                Some(b) => b.max_assign(&val),
            }
        }
        best.expect("a legal category exists")
    }

    /// Keep values from hand values (step 2): `k` gets the value of every keep given the values `e` of the
    /// hands after the reroll.
    fn keep_values<T: Value>(&self, e: &[T], k: &mut [T]) {
        k[self.first_roll..].clone_from_slice(e);
        for i in (0..self.first_roll).rev() {
            let ch = &self.child[i];
            let mut sum = k[usize::from(ch[0])].clone();
            for &c in &ch[1..] {
                sum.add_assign(&k[usize::from(c)]);
            }
            sum.div_faces();
            k[i] = sum;
        }
    }

    /// Hand values from keep values (step 3): the best keep of each hand.
    fn hand_values<T: Value>(&self, k: &[T], e: &mut [T]) {
        for (r, slot) in e.iter_mut().enumerate() {
            let subs = &self.subs[self.subs_off[r] as usize..self.subs_off[r + 1] as usize];
            let mut best = k[usize::from(subs[0])].clone();
            for &i in &subs[1..] {
                best.max_assign(&k[usize::from(i)]);
            }
            *slot = best;
        }
    }

    /// V(s) from the table entries of the states after `s`.
    fn state_value<T: Value>(&self, s: &State, table: &[T], scr: &mut Scratch<T>) -> T {
        let Scratch { e, k } = scr;
        self.final_values(s, table, e);
        for level in 0..self.variant.rolls() {
            self.keep_values(e, k);
            if level + 1 < self.variant.rolls() {
                self.hand_values(k, e);
            }
        }
        k[0].clone()
    }

    /// The table entries for every state with filled mask `mask`, in index order.
    fn mask_values<T: Value>(&self, mask: u32, table: &[T], scr: &mut Scratch<T>) -> Vec<T> {
        let sp = self.space;
        let mut out = Vec::with_capacity(sp.upper_values * sp.armed_values);
        if mask == self.variant.all_mask() {
            out.resize(sp.upper_values * sp.armed_values, T::zero());
            return out;
        }
        let y_filled = self.variant.all_same_box().is_some_and(|y| mask & (1 << y) != 0);
        for up in 0..sp.upper_values {
            for armed in 0..sp.armed_values {
                if armed == 1 && !y_filled {
                    // The flag only matters once the Yahtzee box is filled; filling it sets the flag afresh.
                    let v0 = out.last().cloned().expect("armed 0 comes first");
                    out.push(v0);
                    continue;
                }
                let s = State { filled: mask, upper: up as u16, bonus_armed: armed == 1 };
                out.push(self.state_value(&s, table, scr));
            }
        }
        out
    }

    /// Solves the variant: the table of V for every state, in [`StateSpace`] order.
    pub fn solve<T: Value>(&self) -> Vec<T> {
        let sp = self.space;
        let per_mask = sp.upper_values * sp.armed_values;
        let mut table = vec![T::zero(); sp.len()];
        let c = sp.categories as u32;
        for filled in (0..=c).rev() {
            let masks: Vec<u32> = (0..1u32 << c).filter(|m| m.count_ones() == filled).collect();
            let layer = self.solve_layer(&masks, &table);
            for (m, vals) in masks.iter().zip(layer) {
                let base = *m as usize * per_mask;
                table[base..base + per_mask].clone_from_slice(&vals);
            }
        }
        table
    }

    #[cfg(feature = "parallel")]
    fn solve_layer<T: Value>(&self, masks: &[u32], table: &[T]) -> Vec<Vec<T>> {
        use rayon::prelude::*;
        masks.par_iter().map_init(|| self.scratch(), |scr, &m| self.mask_values(m, table, scr)).collect()
    }

    #[cfg(not(feature = "parallel"))]
    fn solve_layer<T: Value>(&self, masks: &[u32], table: &[T]) -> Vec<Vec<T>> {
        let mut scr = self.scratch();
        masks.iter().map(|&m| self.mask_values(m, table, &mut scr)).collect()
    }

    /// The expected value of the first roll, from V: sum over hands of P(hand) times the value `e` of the
    /// hand. Used by tests to check that probabilities sum to one.
    pub fn roll_expectation<T: Value>(&self, e: &[T]) -> T {
        let mut sum = T::zero();
        for (x, &a) in e.iter().zip(&self.arrangements) {
            let mut t = x.clone();
            t.mul_int(a);
            sum.add_assign(&t);
        }
        sum.div_int(6u64.pow(self.variant.dice() as u32));
        sum
    }

    /// All the within-turn values of state `s`, derived from the table.
    pub fn turn_values<T: Value>(&self, s: &State, table: &[T]) -> TurnValues<T> {
        let mut scr = self.scratch();
        let rolls = usize::from(self.variant.rolls());
        let mut hands = Vec::with_capacity(rolls);
        let mut keeps = Vec::with_capacity(rolls);
        self.final_values(s, table, &mut scr.e);
        for level in 0..rolls {
            hands.push(scr.e.clone());
            self.keep_values(&scr.e, &mut scr.k);
            keeps.push(scr.k.clone());
            if level + 1 < rolls {
                self.hand_values(&scr.k, &mut scr.e);
            }
        }
        TurnValues { hands, keeps }
    }

    /// The value of every legal action in a situation, in [`Variant::legal_actions`] order: for a category,
    /// the points earned plus V of the next state; for a keep, the expected value after rerolling.
    pub fn action_values<T: Value>(&self, sit: &crate::Situation, table: &[T]) -> Vec<(Action, T)> {
        let v = &self.variant;
        let mut out = Vec::new();
        for sc in v.score_choices(&sit.state, &sit.dice).expect("situation is legal") {
            let (next, _) = v.apply_score(&sit.state, &sit.dice, sc.category).expect("legal choice");
            let mut val = table[self.space.index(&next)].clone();
            val.add_assign(&T::from_points(sc.total()));
            out.push((Action::Score(sc.category), val));
        }
        if sit.rolls_left > 0 {
            let j = usize::from(sit.rolls_left);
            let mut scr = self.scratch();
            self.final_values(&sit.state, table, &mut scr.e);
            for level in 0..j {
                self.keep_values(&scr.e, &mut scr.k);
                if level + 1 < j {
                    self.hand_values(&scr.k, &mut scr.e);
                }
            }
            let subs = sit.dice.sub_multisets();
            for keep in &subs[..subs.len() - 1] {
                let i = self.keep_index(keep).expect("keep is a multiset of at most n dice");
                out.push((Action::Keep(*keep), scr.k[i].clone()));
            }
        }
        out
    }
}

/// The within-turn values of one state.
#[derive(Clone, Debug)]
pub struct TurnValues<T> {
    /// `hands[j][r]`: the value of full hand `r` (in [`TurnModel::hands`] order) with `j` rerolls left.
    pub hands: Vec<Vec<T>>,
    /// `keeps[j][k]`: the value of keeping `k` (in [`TurnModel::keeps`] order) and rerolling the rest, with
    /// `j` rerolls left after that reroll. `keeps[rolls - 1][0]` (the empty keep before the first roll) is V.
    pub keeps: Vec<Vec<T>>,
}
