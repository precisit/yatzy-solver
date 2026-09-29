//! Measures what the tie tolerance must separate (docs/queries.md): `cargo run --release --example ties`.
//!
//! On 100 000 situations per variant (half reached in optimal play, half from uniformly random states):
//! - the largest |f32 - f64| error of an option value;
//! - in f64, the largest gap between the best option and another option within 1e-6 of it ("tie noise"), and
//!   the smallest gap to the second-best option above that ("smallest real gap");
//! - in f32, the largest gap between options tied in f64.

use yatzy_solver::rules::Action;
use yatzy_solver::{Precision, Rng, Situation, Solver, State, Table, Variant};

fn main() {
    for id in ["yatzy-scandinavian", "american"] {
        let v = Variant::by_id(id).unwrap();
        let f64s = Solver::build(&v);
        let f32s = Solver::from_table(&Table::from_values(&v, Precision::F32, f64s.values().to_vec()));
        let mut sits: Vec<Situation> = Vec::new();
        // Situations reached in optimal play.
        for log in f64s.simulate_optimal(2_000, 5, true).logs {
            sits.extend(log.decisions.iter().map(|d| d.situation));
        }
        sits.truncate(50_000);
        // Uniformly random states and dice.
        let mut rng = Rng::new(17);
        while sits.len() < 100_000 {
            let filled = rng.next_u64() as u32 & v.all_mask();
            if filled == v.all_mask() {
                continue;
            }
            let upper = if filled & v.upper_mask() != 0 { rng.below(64) as u16 } else { 0 };
            let yfilled = v.all_same_box().is_some_and(|y| filled & (1 << y) != 0);
            let state = State { filled, upper, bonus_armed: yfilled && rng.below(2) == 1 };
            sits.push(Situation { state, dice: rng.roll(5), rolls_left: rng.below(3) as u8 });
        }
        let (mut max_err, mut tie_noise, mut f32_tie_gap, mut min_gap) = (0f64, 0f64, 0f64, f64::INFINITY);
        let (mut ties, mut min_gap_at) = (0usize, String::new());
        // Gaps below 1e-4 by decade: index d counts gaps in [10^-(d+1), 10^-d), the last bucket below 1e-16.
        let mut decades = [0usize; 17];
        let mut smallest_above_noise = f64::INFINITY;
        // Tie-set disagreements between f32 (several tolerances) and f64 (1e-9), and best-action changes.
        let cands = [1e-9, 1e-7, 1e-6, 6.1e-5];
        let mut disagree = [0usize; 4];
        let mut best_changed = 0usize;
        let set = |vals: &[f64], eps: f64| -> Vec<usize> {
            let b = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (0..vals.len()).filter(|&i| vals[i] >= b - eps).collect()
        };
        for sit in &sits {
            let a = f64s.option_values(sit).unwrap();
            let b = f32s.option_values(sit).unwrap();
            for (x, y) in a.iter().zip(&b) {
                max_err = max_err.max((x.value - y.value).abs());
            }
            let va: Vec<f64> = a.iter().map(|o| o.value).collect();
            let vb: Vec<f64> = b.iter().map(|o| o.value).collect();
            let s64 = set(&va, 1e-9);
            for (k, &e) in cands.iter().enumerate() {
                if set(&vb, e) != s64 {
                    disagree[k] += 1;
                }
            }
            if set(&vb, 1e-9)[0] != s64[0] {
                best_changed += 1;
            }
            let best = a.iter().map(|o| o.value).fold(f64::NEG_INFINITY, f64::max);
            let ib = a.iter().position(|o| o.value == best).unwrap();
            for (i, o) in a.iter().enumerate() {
                if i == ib {
                    continue;
                }
                let gap = best - o.value;
                if gap < 1e-4 {
                    let d = if gap <= 0.0 { 16 } else { ((-gap.log10()).floor() as usize).min(16) };
                    decades[d] += 1;
                    if gap > 1e-12 {
                        smallest_above_noise = smallest_above_noise.min(gap);
                    }
                }
                if gap < 1e-12 {
                    ties += 1;
                    tie_noise = tie_noise.max(gap);
                    f32_tie_gap = f32_tie_gap.max((b[ib].value - b[i].value).abs());
                } else if gap < min_gap {
                    min_gap = gap;
                    let fmt = |a: &Action| v.format_action(a);
                    min_gap_at = format!("{} : {} vs {}", v.format_situation(sit), fmt(&a[ib].action), fmt(&o.action));
                }
            }
        }
        println!("{id}: {} situations, {ties} tied pairs", sits.len());
        println!("  largest |f32 - f64| option error   {max_err:.3e}");
        println!("  f64 tie noise (largest)            {tie_noise:.3e}");
        println!("  f32 gap between f64-tied options   {f32_tie_gap:.3e}");
        println!("  smallest real gap (f64)            {min_gap:.3e}");
        println!("  smallest gap above 1e-12 (f64)     {smallest_above_noise:.3e}");
        for (k, e) in cands.iter().enumerate() {
            println!("  f32 tolerance {e:.1e}: tie sets differ from f64 in {} situations", disagree[k]);
        }
        println!("  f32 best action differs from f64 in {best_changed} situations");
        let hist: Vec<String> =
            decades.iter().enumerate().filter(|&(_, &n)| n > 0).map(|(d, n)| format!("1e-{}: {n}", d + 1)).collect();
        println!("  gaps below 1e-4 by decade (upper bound 1e-d+1): {}", hist.join(", "));
        println!("    at {min_gap_at}");
    }
}
