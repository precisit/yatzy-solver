# yatzy-solver

Exact solver, rules engine, simulator and data exporter for Scandinavian Yatzy and American rules
(Yahtzee-compatible): the optimal expected final score of every decision in every situation.

It reproduces the published optimal values (American rules 254.5896; Scandinavian Yatzy 248.4399894 under its
stated rules) and checks itself against an exact brute-force solver.

```rust
use yatzy_solver::{Solver, Variant};

let v = Variant::scandinavian();                 // or Variant::american()
let solver = Solver::build(&v);                  // a couple of seconds on all cores
let sit = v.parse_situation("dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -").unwrap();
for o in solver.option_values(&sit).unwrap() {
    println!("{:>10.4}  {}", o.value, v.format_action(&o.action));
}
```

Values are expected remaining scores. See the [repository](https://github.com/precisit/yatzy-solver) for the
rules, the notation, the table format, the Python and npm packages, and the validation.

Features: `parallel` (default, solves on all cores), `verify` (exact rational arithmetic and the brute-force
reference solver), `parquet` (Parquet output for the export).

License: MIT.
