//! Measures the SPEC 5.2 query targets: `cargo run --release --example bench -- [variant]`.

use std::hint::black_box;
use std::time::Instant;

use yatzy_solver::dice::all_multisets;
use yatzy_solver::export::{Source, export_rows};
use yatzy_solver::{Precision, Situation, Solver, State, Table, TurnModel, Variant};

/// A small deterministic generator (xorshift), so runs are repeatable without a dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn main() {
    let id = std::env::args().nth(1).unwrap_or_else(|| Variant::SCANDINAVIAN.into());
    let v = Variant::by_id(&id).unwrap();
    let table = Table::build(&v, Precision::F32);
    let model = TurnModel::new(&v);
    let space = model.space();
    let hands = all_multisets(v.dice());
    let mut rng = Rng(0x9e3779b97f4a7c15);

    // Random situations over non-final states.
    let n = 20_000;
    let sits: Vec<Situation> = (0..n)
        .map(|_| {
            let filled = loop {
                let m = rng.next() as u32 & v.all_mask();
                if m != v.all_mask() {
                    break m;
                }
            };
            let upper = if filled & v.upper_mask() != 0 { rng.below(64) as u16 } else { 0 };
            let yfilled = v.all_same_box().is_some_and(|y| filled & (1 << y) != 0);
            let state = State { filled, upper, bonus_armed: yfilled && rng.below(2) == 1 };
            Situation { state, dice: hands[rng.below(hands.len() as u64) as usize], rolls_left: rng.below(3) as u8 }
        })
        .collect();

    // state_value: index plus lookup.
    let reps = 50;
    let t = Instant::now();
    let mut acc = 0.0;
    for _ in 0..reps {
        for s in &sits {
            acc += table.values()[space.index(black_box(&s.state))];
        }
    }
    let per = t.elapsed().as_secs_f64() / (reps * n) as f64;
    println!("state_value            {:10.1} ns   ({acc:.0})", per * 1e9);

    // option values, all legal actions of one situation, by rolls left.
    for rl in 0..3u8 {
        let subset: Vec<&Situation> = sits.iter().filter(|s| s.rolls_left == rl).collect();
        let t = Instant::now();
        let mut count = 0usize;
        for s in &subset {
            count += black_box(model.action_values(s, table.values())).len();
        }
        let per = t.elapsed().as_secs_f64() / subset.len() as f64;
        println!("option_values rolls {rl}   {:10.1} us   ({} situations, {count} options)", per * 1e6, subset.len());
    }
    let t = Instant::now();
    for s in &sits {
        black_box(model.action_values(s, table.values()));
    }
    let per = t.elapsed().as_secs_f64() / n as f64;
    println!("option_values mixed    {:10.1} us   = {:.0} situations/s on one core", per * 1e6, 1.0 / per);

    // Batch labelling through the public API, on 100 000 distinct situations from each export source (a
    // cycled small set would stay in cache and overstate the rate).
    let solver = Solver::from_table(&table);
    let threads = std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "all".into());
    for (name, src) in [("uniform", Source::Uniform), ("optimal", Source::Optimal)] {
        let batch: Vec<Situation> = export_rows(&solver, src, 1, 100_000).into_iter().map(|r| r.situation).collect();
        let t = Instant::now();
        let flat = solver.option_values_flat(&batch).unwrap();
        let secs = t.elapsed().as_secs_f64();
        println!(
            "batch {name} ({threads} threads)  {:10.0} situations/s   ({} rows)",
            batch.len() as f64 / secs,
            flat.rows
        );
    }
}
