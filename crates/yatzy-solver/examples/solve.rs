//! Solves a built-in variant and prints the expected score: `cargo run --release --example solve -- american`.

use std::time::Instant;

use yatzy_solver::{State, TurnModel, Variant};

fn main() {
    let id = std::env::args().nth(1).unwrap_or_else(|| "yatzy-scandinavian".into());
    let v = Variant::by_id(&id).expect("unknown variant");
    let t = Instant::now();
    let model = TurnModel::new(&v);
    let table: Vec<f64> = model.solve();
    let ev = table[model.space().index(&State::new())];
    println!("{id}: {ev:.10} ({} states, {:.2?})", table.len(), t.elapsed());
}
