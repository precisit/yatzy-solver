# Queries and simulation

## Values

Every value is an **expected remaining score**: the points still to come from that point on, bonuses included,
when play is optimal from then on. The score already on the card is not included; add it to get the expected
final score. Values are exact up to f64 rounding (or f32, for a solver loaded from an f32 table).

| query | meaning |
| --- | --- |
| `state_value(state)` | the value at the start of a turn, before the first roll; 0 when the game is over |
| `option_values(situation)` | every legal option with its value, in legal-action order (categories in score-card order, then keeps by size and lexicographically) |
| `situation_value(situation)` | the value of the best option |
| `best_options(situation)` | every option within `TIE_EPSILON` (1e-9) of the best, in legal-action order |
| `best_action(situation)` | the first of the best options |
| `regret(situation, action)` | the expected points lost against a best option: 0 for any best option, an error for an illegal action |

- **Score so far and points to come.** The score so far (`Game::total`, and `score_so_far` in exports)
  includes every bonus already earned; values include every bonus not yet earned. So at every decision, the
  score so far plus the points still to come is the final score, exactly; a test checks this at every decision
  of logged games, including games where the upper bonus is earned. Expected final score = score so far +
  value.
- The value of **scoring** in a category is the points it earns (with any bonus) plus `state_value` of the next
  state.
- The value of a **keep** is the expectation, over every outcome of the reroll, of the value of the resulting
  situation.
- **Ties** are real: for example, on the last turn with only Yatzy open and 1 1 2 2 3 showing, keeping 1 1 and
  keeping 2 2 are equally good. Options within `Solver::tie_epsilon()` of each other are tied; see below.
- Queries on situations that cannot occur (a finished game, the wrong number of dice, too many rerolls left, an
  inconsistent state) return an error, never a panic.

The tests check these definitions independently of the solver's turn tables: each keep's value is recomputed by
enumerating every reroll outcome through the rules engine, each category's by `apply_score` and
`state_value`, and V by the expectation of the first roll (`tests/queries.rs`).

### Tie tolerance

Measured with `cargo run --release --example ties` on 100 000 situations per variant (half reached in optimal
play, half from uniformly random states), comparing an f64 solver with one loaded from an f32 table:

| | Scandinavian | American |
| --- | --- | --- |
| largest gap between exactly tied options, f64 | 2.8e-14 | 2.8e-14 |
| largest gap between the same options, f32 table | 1.8e-15 | 2.8e-14 |
| smallest gap between different options (best to next), f64 | 5.7e-7 | 3.1e-5 |
| largest error of an option value, f32 table against f64 | 7.6e-6 | 1.5e-5 |
| situations whose tie set differs from f64, f32 table with tolerance 1e-9 | 0 | 0 |
| the same with tolerance 6.1e-5 (4 x the f32 error) | 5 | 1 |

- **f64:** the tolerance is 1e-9, about five orders of magnitude above the tie noise and two below the
  smallest real gap.
- **f32:** exactly tied options are computed from the same rounded table entries, so they stay tied to about
  1e-14, and the same 1e-9 gives the same tie sets as f64 on every sampled situation. A tolerance scaled to the
  f32 value error would merge genuinely different options. **But no tolerance can separate everything for f32
  tables:** option values carry up to 1.5e-5 of error, which is more than the smallest real gaps (5.7e-7), so
  from an f32 table the order of two options closer than about 3e-5 is not reliable (in the sample, the best
  action never changed). Use an f64 table when those distinctions matter.
- The test suite checks that tie sets from f32 and f64 tables agree on about 22 000 situations (and requires at
  least 50 of them to have ties).

### Batch

`state_values(&[State])` and `option_values_batch(&[Situation])` return results in input order;
`option_values_batch` runs on all cores with the `parallel` feature. Numpy and typed-array interfaces come with
the Python and WebAssembly bindings.

## Simulation

`Solver::simulate_optimal(games, seed, logs)` plays games under the optimal policy (ties broken by the first
best action); `simulate(variant, policy, games, seed, logs)` plays them under any `Policy`, for example a
learner's, or `RandomPolicy`. A policy that chooses an illegal action stops the run with an `IllegalChoice`
error. Results hold every final score, summary statistics (mean, standard deviation and error, minimum,
median, maximum), percentiles and a histogram, and optionally a log of every decision per game.

### The generator (stable contract, version 1)

The same seed gives the same games in every version and every language binding (the bindings call the Rust
code). Logs and exports record the generator version as `rng 1`, so a future change cannot silently mix
datasets.

- The generator is xoshiro256** (Blackman and Vigna). Its state is filled by four consecutive outputs of
  SplitMix64 started from the stream's seed.
- `mix(x)` is one SplitMix64 step from state `x`: `z = x + 0x9E3779B97F4A7C15`, then
  `z = (z ^ z >> 30) * 0xBF58476D1CE4E5B9`, `z = (z ^ z >> 27) * 0x94D049BB133111EB`, `z ^ z >> 31`
  (64-bit wrapping). It is a bijection.
- **Dice:** turn `t` (0-based) of game `g` in a run with seed `s` has its own stream, seeded with
  `mix(mix(s ^ 0x6469636500000001) ^ (g << 8 | t))`. Every roll in the turn draws a full block of `n` dice (`n`
  = 5), in order. The first roll uses the whole block; a reroll of `m` dice uses the first `m` of its block.
- **Policy:** game `g` gives the policy a separate stream, seeded with `mix(mix(s ^ 0x706f6c6963790001) ^ g)`.
- **A die** is `1 + x % 6` for the next output `x`, drawn again while `x >= 2^64 - (2^64 mod 6)`.
- Game indices are below 2^56 and turns below 256. For a fixed seed, `key -> mix(c ^ key)` is a bijection, so
  distinct (game, turn) pairs never share a stream.

**Common random numbers.** The dice do not depend on the policy: under the same seed, roll `r` of turn `t` is
the same for every policy, and a randomizing policy never shifts the dice. Comparing a player with optimal
play on the same seed therefore compares them on identical dice, which needs far fewer games for the same
confidence than independent dice. Each game has its own streams, so results do not depend on how the games are
spread over threads. The test suite pins the streams against an independent implementation, and checks that
the optimal and the random policy see the same first roll of every turn.

### Tie-breaking

`best_action` and the optimal simulator take the first best option in legal-action order. This is a
reproducibility rule, not a preference among equally good options: training data should never use
`best_action` as a label. The export carries every option's value, which represents ties as they are.

### Game logs

`GameLog::to_lines` gives a header `rng 1 | seed <s> | game <g>`, one line per decision in the stable notation,
`<situation> => <action>`, then `final <score>`. The CLI writes JSON Lines, one object per game:

```json
{"variant":"yatzy-scandinavian","rng":1,"seed":1,"game":0,"score":189,"decisions":[{"situation":"dice 1 2 6 6 6 | rolls 2 | upper 0 | filled -","action":"keep 6 6 6","points":null}, ...]}
```

`points` is the total a category earned (bonuses included), or `null` for a keep.

## Command line

```sh
yatzy-solver query "dice 1 1 2 2 3 | rolls 2 | upper 0 | filled -"
yatzy-solver simulate --variant american --games 1000000 --seed 2026
yatzy-solver simulate --games 100 --seed 1 --policy random --log games.jsonl
```

## Results (M3)

One million games under the optimal policy, seed 2026, generator version 1:

| variant | exact mean | simulated mean (95% interval) | std dev | median |
| --- | --- | --- | --- | --- |
| `american` | 254.5896 | 254.5405 (254.4238 to 254.6572) | 59.5420 | 248 |
| `yatzy-scandinavian` | 248.4400 | 248.4459 (248.3705 to 248.5212) | 38.4378 | 249 |

For American rules the published standard deviation is 59.6117 and the median 248 (Verhoeff); the exact
distribution is milestone M7.
