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

Measured on an Apple M1 Max (10 cores, 32 GB), release build, 2026-09-29. **Provisional**: the machine was
running other heavy work, with load averages of 30 to 60 during the measurements.

| target | measured | load average |
| --- | --- | --- |
| build Scandinavian, one core: under 5 min | 28.5 s | 48 |
| build Scandinavian, all cores: under 1 min | 4.1 s | 30 |
| build American rules, one core | 11.3 s | 48 |
| `state_value`: under 1 µs | 3.5 ns | 54 |
| option values of one situation: under 50 µs | 0.7 µs (0 rerolls left), 7.9 µs (1), 13.8 µs (2) | 54 |
| batch labelling: 100 000 situations per second per core | about 144 000 (mixed rerolls left) | 54 |
| batch labelling through `option_values_batch`, one thread / all threads | 144 800 / 630 600 per second | 70 to 120 |
| table size, f32: 8 MB | 8 388 768 bytes | |

Reproduce with `RAYON_NUM_THREADS=1 yatzy-solver build`, `yatzy-solver build`, and
`cargo run --release --example bench`.
