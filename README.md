# yatzy-solver

**Exact optimal play for Scandinavian Yatzy and American rules (Yahtzee-compatible), reproducing the published
values:** American rules 254.5896 (Verhoeff, Glenn) and Scandinavian Yatzy 248.4399894 under its stated rules.
Check it yourself:

```sh
cargo run --release -p yatzy-solver-cli -- verify --golden golden/parity.jsonl
```

That runs an exact brute-force cross-check on reduced games (rational arithmetic, 0 mismatches), solves the full
games against the published values, and checks 300 golden situations bit for bit. It takes a few minutes.

yatzy-solver gives the expected final score of **every decision in every situation**: every keep and every
category, not only the best one. It has a rules engine, a simulator and a data exporter, with the same answers
from Rust, Python and JavaScript (WebAssembly).

## Install

```sh
cargo add yatzy-solver          # Rust library
cargo install yatzy-solver-cli  # the `yatzy-solver` command
pip install yatzy-solver        # Python (numpy)
npm install yatzy-solver        # JavaScript and TypeScript (WebAssembly)
```

Prebuilt CLI binaries for Linux (x86-64), macOS (arm64) and Windows (x86-64) are attached to each release. The
macOS binary is not notarized: use `cargo install yatzy-solver-cli`, or clear the quarantine flag after
downloading (`xattr -d com.apple.quarantine yatzy-solver-aarch64-macos`).

## Examples

Rust:

```rust
use yatzy_solver::{Solver, Variant};

let v = Variant::scandinavian();
let solver = Solver::build(&v);                  // a couple of seconds on all cores
let sit = v.parse_situation("dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -").unwrap();
for o in solver.option_values(&sit).unwrap() {
    println!("{:>10.4}  {}", o.value, v.format_action(&o.action));
}
```

Python:

```python
import yatzy_solver as ys

solver = ys.Solver.build(ys.Variant("yatzy-scandinavian"))
sit = "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -"
for action, value in solver.option_values(sit):
    print(f"{value:10.4f}  {action}")
print(solver.best_options(sit), solver.regret(sit, "keep 6"))
```

JavaScript:

```js
import * as ys from "yatzy-solver";               // in a browser: `await ys.default()` first

const v = new ys.Variant("yatzy-scandinavian");
const solver = ys.Solver.fromTable(v, tableBytes); // a table from `yatzy-solver build`, or ys.Solver.build(v)
for (const o of solver.optionValues("dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -")) {
  console.log(o.value.toFixed(4), o.action);
}
```

Command line:

```sh
yatzy-solver query "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -"   # every option, its value and regret
yatzy-solver simulate --variant american --games 1000000 --seed 2026
yatzy-solver export --source perturbed --perturb 0.2 --rows 1000000 --format parquet --out rows.parquet
```

All values are expected **remaining** scores: add the score so far for the expected final score.

## The rules

- **Scandinavian Yatzy** (`yatzy-scandinavian`): five dice, three rolls, 15 boxes: Ones to Sixes (bonus 50 at
  63), one pair, two pairs, three and four of a kind, small straight 1-2-3-4-5 (15), large straight 2-3-4-5-6
  (20), full house (the sum), chance, and Yatzy (50). House-rule switches: five of a kind as a full house
  (`+fh5`), four of a kind as two pairs (`+tp4`), forced order (`+forced`).
- **American rules** (`american`, Yahtzee-compatible): 13 boxes with the upper bonus 35 at 63, full house 25,
  straights 30 and 40, and the 100-point bonus for extra five of a kinds with the joker rule (free placement as
  in the published solution; `+forced-joker`, `+no-joker` and `+no-bonus` give the alternatives).

Details in [docs/rules.md](docs/rules.md).

## How it works

Backward induction over the 2^15 x 64 states between turns (filled boxes, upper total capped at 63): each final
hand is valued by its best box, each keep by the average over the dice rerolled (computed incrementally, one
die at a time), each earlier hand by its best keep, and the value of a state by the expectation over the first
roll. The table of those state values (8 MB in f32) gives the value of any option in microseconds. It is
solved in f64 with a fixed summation order, so the table is bit-identical on every platform. Details in
[docs/solver.md](docs/solver.md).

## Validation

| variant | this solver | reference |
| --- | --- | --- |
| American rules (Yahtzee-compatible) | 254.5896095 | 254.5896 (Verhoeff, Glenn) |
| American rules without the five-of-a-kind bonus and joker | 245.8707745 | 245.87 (Verhoeff, Glenn) |
| Scandinavian Yatzy | 248.4399894 | 248.4394 (Castux/yahtzee), about 248.44 (Laurii1i/Yatzy) |

**About the published 248.63.** The Scandinavian value usually cited, 248.63 (Larsson and Sjöberg, KTH 2012),
comes from a bug in the authors' code, not from their rules. Their stated rules match ours. Their published Java
code ([ansjob/optimalt-yatzy](https://github.com/ansjob/optimalt-yatzy), `ScoreCard.java`, function
`scorePair`) scores Two pairs as twice the highest pair plus twice the second pair, adding 0 when there is no
second pair, so a hand with a single pair scores in Two pairs. With that scoring this solver gives 248.6328539,
which rounds to 248.63; `verify` checks this too. Details in [docs/rules.md](docs/rules.md).

Castux's 248.4394 differs in the fourth decimal because that solver computes in single precision: its dice
outcome probabilities are built by adding `(float)(1 / 6^n)` per ordered roll, so the five-dice distribution
sums to 0.9999997, and a little value leaks at every expectation. The rules are the same.

Also checked: an exact brute-force reference solver agrees with the fast solver in rational arithmetic on nine
reduced games; the simulated mean of a million optimal games falls within its interval of the exact mean; and
Rust, Python and WebAssembly give bit-identical values on a golden set of 300 situations.

## Tables

Prebuilt tables are attached to each release. Check a download with `shasum -a 256 -c release/tables.sha256`.

| file | SHA-256 |
| --- | --- |
| `yatzy-scandinavian.f32.yzt` | `91b0f362e2e7cd260f4b70a16cfbe6d67ee0ea66a71ee49d2cda5ab2d2323108` |
| `yatzy-scandinavian.f64.yzt` | `76a60b171f9cd59d6a114b95dee60cc7d63d23f418b6e63a76c930f236799a3e` |
| `american.f32.yzt` | `045e5eaca9f284b43615f513780fac6b46421e7c1b5745e5ef4a6ccb89227f6d` |
| `american.f64.yzt` | `21c1031f82f63eeee91d5aaa836f221db7f59eb216d57329a2c6d5b8f9c11b13` |

The file hashes include the header, which records the solver version (1.0.0). The hashes of the value arrays
alone, the same on every platform and across versions while the values do not change, are in
`release/values.sha256` and printed by `yatzy-solver build` as `values hash`.

## Documentation

- [Rules](docs/rules.md): the variants, house-rule switches and joker rules.
- [Notation](docs/notation.md): the stable text form of situations and actions, and the action codes.
- [Solver](docs/solver.md): the method, verification, table format and performance.
- [Queries and simulation](docs/queries.md): option values, best options, regret, batch queries, the seeded
  simulator and its generator.
- [Export](docs/export.md): sampled situations with every option's value, JSON Lines or Parquet, and the schema.
- [Python](docs/python.md): the `yatzy_solver` package.
- [WebAssembly and the web advisor](docs/wasm.md): the npm package, table loading, and the offline advisor.
- [CHANGELOG](CHANGELOG.md).

## Prior work

- Tom Verhoeff (1999) solved solitaire Yahtzee: expected score 254.5896, standard deviation 59.6117, median 248
  ([trivia page](https://www-set.win.tue.nl/~wstomv/misc/yahtzee/trivia.html)).
- James Glenn solved it independently and described symmetry optimizations
  ([Computer Strategies for Solitaire Yahtzee, CIG 2007](http://www.cs.loyola.edu/~jglenn/Papers/yahtzee_cig2007_glenn.pdf)).
- Scandinavian Yatzy was solved by Larsson and Sjöberg (KTH, 2012), published expected score 248.63 (see above)
  ([report](https://www.csc.kth.se/utbildning/kth/kurser/DD143X/dkand12/Group89Michael/report/Larsson+Sjoberg.pdf)),
  and studied by Sederblad and Törnebohm (KTH, 2013,
  [report](https://www.diva-portal.org/smash/get/diva2:676659/FULLTEXT01.pdf)).
- Independent open-source solvers of Scandinavian Yatzy, [Castux/yahtzee](https://github.com/Castux/yahtzee)
  and [Laurii1i/Yatzy](https://github.com/Laurii1i/Yatzy), which confirm the value under the stated rules.
- Jakub Pawlewicz studied nearly optimal multiplayer play ("Nearly Optimal Computer Play in Multi-player Yahtzee",
  Computers and Games 2010).

"Yatzy" is used as the name of the game; this project is not affiliated with any trademark holder. "Yahtzee" is a
trademark of Hasbro. This project implements rules compatible with it and is not affiliated with Hasbro.

## License

MIT, see [`LICENSE`](LICENSE). The prebuilt tables are under the same license.
