# The solver

## Method

The solver maximizes the expected final score, bonuses included, by backward induction (SPEC 3).

- **States between turns** are (the filled categories, the upper-section total capped at the bonus threshold,
  and for American rules whether the Yahtzee box holds 50). Scandinavian Yatzy has 2^15 x 64 = 2 097 152
  states and American rules 2^13 x 64 x 2 = 1 048 576. Every combination is stored, reachable or not.
- **The table** holds V(state), the expected remaining score from the start of a turn. States are solved in
  layers by the number of filled categories, from the full card (V = 0) down to the empty one. States within a
  layer are independent and are solved in parallel.
- **Within a turn**, for one state:
  1. each of the 252 final hands is valued by its best legal category: the points (with any bonus) plus V of
     the next state;
  2. each of the 462 keeps is valued with the incremental recursion K(k) = (K(k+1) + ... + K(k+6)) / 6, from
     keeps of one die more down to the empty keep, starting from the hand values;
  3. each hand is valued by its best keep (keeping all five covers scoring now), and steps 2 and 3 repeat for
     the second and first roll;
  4. V(state) is the value of the empty keep before the first roll.
- **Per-decision values** (every keep and every category of a situation) are derived from V the same way, on
  demand (`TurnModel::action_values`).
- Hands of five of a kind with the Yahtzee box filled (bonus and joker) are scored through the rules engine
  itself, so the solver and the engine cannot disagree about them.

## Determinism

All arithmetic is in f64 with a fixed summation order (faces 1 to 6, hands and keeps in a fixed order);
parallelism is only across independent states. The table is bit-identical whatever the number of threads, and
across platforms (IEEE 754, no fused multiply-add); CI checks the pinned hashes below on Linux, macOS and
Windows. Two hashes are printed by `yatzy-solver build`:

- the **values hash**, the SHA-256 of the value array: identical on every platform, and published per release;
- the **file hash**, the SHA-256 trailer of the file: it covers the header too, which includes the solver
  version, so it changes with every release even when the values do not.

| variant | precision | values hash (SHA-256) |
| --- | --- | --- |
| `yatzy-scandinavian` | f32 | `3a5bb59a68632c6236028a6753e01ee39c92c6294d669ad55ec26290e651cf7d` |
| `american` | f32 | `588e16790260e43cfce2222a96e517cdc15091600672234a1da92639abde7c9b` |

## Verification

`yatzy-solver verify` runs:

- **the brute-force cross-check**: a separate expectimax solver that enumerates every ordered outcome of every
  roll and every keep by dice position, using only the rules engine, run in exact rational arithmetic on nine
  reduced games (2 to 5 dice, 3 to 7 categories). The fast solver, also run in exact rational arithmetic, must
  agree exactly on every reachable state. The reduced games cover every category kind, the upper bonus, all
  three joker rules, forced order, both two-pairs switches, one pair with two pairs to choose from, the full
  house with and without five of a kind, and both straights in one game;
- **the published and reference values**: American rules 254.5896 (Verhoeff, Glenn), American rules without
  bonus and joker 245.87, Scandinavian Yatzy 248.4399894 under the stated rules (the validation value), and the
  published Scandinavian 248.63 under the scoring code it was computed with (see [rules](rules.md)).

The brute force checks the solver, not the scoring rules: both use the same rules engine. The rules rest on
the table-driven scoring tests (with an independent reference scorer over all 252 hands) and on the published
values.

The test suite adds invariant checks on the full tables: reroll probabilities sum to 1, values are bounded by
the best possible remaining score, V is monotone in the upper total below the threshold and in the open
categories, V equals the expectation of the first roll, and in every sampled situation the best action value
equals the situation's value (no option beats the best).

## Table file

See the `table` module documentation for the byte layout. In short: a 64-byte-aligned header (magic
`YATZYTBL`, format version, state count and layout, precision, objective, SHA-256 of the variant definition,
variant id, solver version), the values little-endian in state order, then a SHA-256 of everything before it.
The state order is `index = ((filled_mask x upper_values) + min(upper, threshold)) x armed_values + armed`.
Loading checks the checksum and that the table was built for exactly the variant definition in use.

## Performance (SPEC 5.2)

Reference figures from a quiet machine: a MacBook Air (Apple M5, 10 cores, 32 GB, air cooled, macOS 27, on
mains power), release build, 2026-09-30, load average 1.6 to 4 throughout.

| target | measured |
| --- | --- |
| build Scandinavian, one core: under 5 min | 10.8 s |
| build Scandinavian, all cores: under 1 min | 1.7 s |
| build American rules, one core / all cores | 4.6 s / 0.76 s |
| `state_value`: under 1 µs | 2.4 ns |
| option values of one situation: under 50 µs | 0.4 µs (0 rerolls left), 2.9 µs (1), 4.6 µs (2) |
| batch labelling, 100 000 distinct situations: 100 000 per second per core | 258 000 (one thread); 504 000 to 519 000 (all threads) |
| table size, f32: 8 MB | 8 388 768 bytes |

Release check, 2026-10-01, on a Mac mini (Apple M5 Pro, 18 cores, 64 GB, macOS 27), reserved for the run; load
average 1.4 to 7 (1-minute; the 5-minute average stayed between 1.5 and 3), with the release-candidate binaries:

| | measured |
| --- | --- |
| `yatzy-solver verify --golden golden/parity.jsonl` | all published values ok; 300 golden situations, 0 mismatches |
| build Scandinavian, one core / all cores | 10.5 s / 0.71 s |
| build American rules, one core / all cores | 4.5 s / 0.32 s |
| `state_value` | 2.5 ns |
| option values of one situation | 0.4 µs (0 rerolls left), 2.8 µs (1), 4.4 µs (2) |
| batch labelling, 100 000 distinct situations | 268 000 to 270 000 (one thread); 587 000 to 618 000 (all threads) |

Earlier figures from an Apple M1 Max under heavy load (load average 50 to 160: 23 s one-core build, 128 000
situations per second per core) are superseded by these. WebAssembly figures are in [wasm](wasm.md).

The batch figures use 100 000 distinct situations from the optimal and uniform export sources. An earlier figure
(M3: 144 800 per second) came from a benchmark that cycled 20 000 situations and so stayed in cache; on distinct
situations the code then ran at about 77 000 per second per core, below the target. Two changes in M4 fixed this
without changing any value (the pinned table hashes and the golden set are unchanged): the value of a category
is computed once per distinct score rather than once per hand, and keeps are enumerated from the turn model's
precomputed lists instead of allocating and sorting per situation.

Reproduce with `RAYON_NUM_THREADS=1 yatzy-solver build`, `yatzy-solver build`, and
`cargo run --release --example bench`.
