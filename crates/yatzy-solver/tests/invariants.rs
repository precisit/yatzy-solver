//! Invariant checks (F8) on full tables: reroll probabilities sum to 1, values are bounded and monotone, no
//! option beats the best, and the within-turn values are consistent with V.

use yatzy_solver::dice::all_multisets;
use yatzy_solver::rules::Action;
use yatzy_solver::{Situation, State, StateSpace, TurnModel, Variant};

struct Rng(u64);
impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

fn solved(id: &str) -> (Variant, TurnModel, Vec<f64>) {
    let v = Variant::by_id(id).unwrap();
    let m = TurnModel::new(&v);
    let t = m.solve();
    (v, m, t)
}

/// An upper bound on the remaining score: the best score of every open box, the upper bonus, and a Yahtzee
/// bonus on every remaining turn.
fn max_remaining(v: &Variant, s: &State) -> f64 {
    let hands = all_multisets(v.dice());
    let mut m = 0.0;
    for c in 0..v.num_categories() {
        if !s.is_filled(c) {
            let best = hands.iter().map(|h| v.score(c, h).max(v.joker_score(c, h))).max().unwrap();
            m += f64::from(best);
        }
    }
    let open = (v.all_mask() & !s.filled).count_ones();
    m + v.upper_bonus().map_or(0.0, |b| f64::from(b.points))
        + v.yahtzee_bonus().map_or(0.0, |b| f64::from(b.points) * f64::from(open))
}

#[test]
fn reroll_probabilities_sum_to_one() {
    let v = Variant::scandinavian();
    let m = TurnModel::new(&v);
    let ones = vec![1.0f64; m.hands().len()];
    assert!((m.roll_expectation(&ones) - 1.0).abs() < 1e-15);
    // Keep values of a constant hand value are that constant.
    let tv = m.turn_values(
        &State { filled: v.all_mask() & !1, upper: 0, yahtzee_armed: false },
        &vec![0.0; StateSpace::of(&v).len()],
    );
    // Only Ones is open: the final hand value is the number of ones, with expectation 5/6 from scratch.
    let ev = m.roll_expectation(&tv.hands[0]);
    assert!((ev - 5.0 / 6.0).abs() < 1e-12, "{ev}");
}

#[test]
fn values_are_bounded_monotone_and_consistent() {
    for id in ["yahtzee", "yatzy-scandinavian"] {
        let (v, m, t) = solved(id);
        let sp = m.space();
        let mut rng = Rng(12345);
        for _ in 0..3000 {
            let filled = rng.below(1 << v.num_categories()) as u32;
            let upper = if filled & v.upper_mask() != 0 { rng.below(64) as u16 } else { 0 };
            let yfilled = v.yahtzee_box().is_some_and(|y| filled & (1 << y) != 0);
            let s = State { filled, upper, yahtzee_armed: yfilled && rng.below(2) == 1 };
            let x = t[sp.index(&s)];
            // Bounded.
            assert!(x >= 0.0 && x <= max_remaining(&v, &s) + 1e-9, "{id}: {} = {x}", v.format_state(&s));
            if v.is_over(&s) {
                assert_eq!(x, 0.0);
                continue;
            }
            // Monotone in the upper total below the threshold (at the threshold the bonus is already paid,
            // so it is no longer part of the remaining value).
            if upper + 1 < 63 && filled & v.upper_mask() != 0 {
                assert!(t[sp.index(&State { upper: upper + 1, ..s })] >= x - 1e-9);
            }
            // An open category is worth something: filling it first cannot help.
            for c in 0..v.num_categories() {
                if !s.is_filled(c) && Some(c) != v.yahtzee_box() {
                    assert!(t[sp.index(&State { filled: filled | 1 << c, ..s })] <= x + 1e-9);
                }
            }
            // V is the value of the turn, and the expectation of the first roll.
            let tv = m.turn_values(&s, &t);
            let rolls = usize::from(v.rolls());
            assert_eq!(tv.keeps[rolls - 1][0], x);
            assert!((m.roll_expectation(&tv.hands[rolls - 1]) - x).abs() < 1e-9);
            // No option beats the best, and the best is attained, in a random situation.
            let r = rng.below(m.hands().len() as u64) as usize;
            let j = rng.below(u64::from(v.rolls())) as u8;
            let sit = Situation { state: s, dice: m.hands()[r], rolls_left: j };
            let vals = m.action_values(&sit, &t);
            let legal = v.legal_actions(&sit).unwrap();
            assert_eq!(vals.iter().map(|a| a.0).collect::<Vec<Action>>(), legal);
            let best = vals.iter().map(|a| a.1).fold(f64::NEG_INFINITY, f64::max);
            assert_eq!(best, tv.hands[usize::from(j)][r], "{id}: {}", v.format_situation(&sit));
        }
    }
}
