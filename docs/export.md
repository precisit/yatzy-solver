# Export

`yatzy-solver export` (and `export` in the library and the Python package) writes sampled situations with the
exact value of every legal option (SPEC F5, 5.5), as JSON Lines or Parquet. Exports are deterministic: the same
variant, table precision, source, seed and row count give the same bytes.

```sh
yatzy-solver export --source optimal   --rows 1000000 --seed 1 --out optimal.parquet --format parquet
yatzy-solver export --source perturbed --perturb 0.2 --rows 1000000 --seed 2 --out perturbed.jsonl
yatzy-solver export --source uniform   --rows 1000000 --seed 3 --out uniform.parquet --format parquet
```

## Sources

- **`optimal`**: every decision of games played optimally, from game 0 on, whole games in order (the last one
  is cut to the row count). Game `g` is the same game as game `g` of the simulator with the same seed.
- **`perturbed`**: the same, except that at each decision, with probability `perturb`, the action is chosen
  uniformly at random among the legal ones. These games reach states a good player rarely sees. The dice come
  from the same streams as the optimal games with the same seed (common random numbers): every turn's first
  roll is the same, and rerolls draw from the same blocks. When `perturb` > 0, each decision uses the game's
  policy stream: one draw `u` (the top 53 bits of the next output, divided by 2^53) compared with `perturb`,
  then, if random, one uniform index.
- **`uniform`**: sample `i` draws, from its own stream `mix(mix(seed ^ 0x73616d706c650001) ^ i)` (see the
  generator in [queries](queries.md)):
  - a state uniformly from the **reachable non-final states**: every filled mask except the full card (under
    forced order, only prefixes), every upper total that the filled upper boxes can add up to (face x 0 to 5
    per box), and, in American rules with the five-of-a-kind box filled, both box values (0 and 50). There are
    1 596 822 such states for Scandinavian Yatzy and 598 636 for American rules;
  - the dice from a fair roll, and the rerolls left uniformly from 0, 1 and 2.

  The score so far is not determined by the state, so it is null.

  Uniform over states is not uniform over the stage of the game: there are more reachable states in mid-game,
  when many masks and upper totals combine. The share of samples by the number of filled boxes (turn - 1),
  exact from the reachable set and from a sample of 1 000 000 rows (seed 2026), lets you reweight if you need
  to:

  | filled | Scandinavian, exact | Scandinavian, 1M sample | American, exact | American, 1M sample |
  | --- | --- | --- | --- | --- |
  | 0 | < 0.01% | < 0.01% | < 0.01% | < 0.01% |
  | 1 | < 0.01% | < 0.01% | 0.01% | 0.01% |
  | 2 | 0.05% | 0.05% | 0.12% | 0.12% |
  | 3 | 0.37% | 0.37% | 0.87% | 0.86% |
  | 4 | 1.72% | 1.71% | 3.61% | 3.60% |
  | 5 | 5.26% | 5.27% | 9.65% | 9.68% |
  | 6 | 11.35% | 11.39% | 17.67% | 17.66% |
  | 7 | 17.99% | 17.97% | 22.95% | 23.00% |
  | 8 | 21.37% | 21.41% | 21.44% | 21.39% |
  | 9 | 19.20% | 19.15% | 14.37% | 14.32% |
  | 10 | 13.02% | 13.00% | 6.77% | 6.81% |
  | 11 | 6.58% | 6.59% | 2.14% | 2.15% |
  | 12 | 2.41% | 2.39% | 0.41% | 0.40% |
  | 13 | 0.60% | 0.61% | | |
  | 14 | 0.09% | 0.09% | | |

## Schema

One row per situation. JSON Lines has one object per line with these keys; Parquet has these columns (zstd
compression, row groups of 100 000).

| field | JSON | Parquet | meaning |
| --- | --- | --- | --- |
| `variant` | string | utf8 | variant id, e.g. `yatzy-scandinavian` |
| `solver_version` | string | utf8 | the version of the solver that computed the values |
| `rng` | integer | uint32 | generator version (1) |
| `precision` | string | utf8 | `f32` or `f64`: the table the values came from |
| `source` | string | utf8 | `optimal`, `perturbed` or `uniform` |
| `perturb` | number or null | float64, nullable | the share of random decisions (perturbed only) |
| `seed` | integer | uint64 | the run's seed |
| `game` | integer | uint64 | game index (trajectories) or sample index (uniform) |
| `decision` | integer | uint32 | the decision's index within its game, from 0 (0 for uniform) |
| `turn` | integer | uint8 | the turn, from 1: filled categories + 1 |
| `filled` | integer | uint32 | filled-category mask, bit `c` for category `c` in score-card order |
| `upper` | integer | uint16 | upper-section total, not capped |
| `five_of_a_kind` | 0, 50 or null | uint16, nullable | American rules: the five-of-a-kind box when filled; null otherwise |
| `score_so_far` | integer or null | uint16, nullable | points on the card, every bonus already earned included; null for uniform samples |
| `dice` | array of 5 integers | list of uint8 | the dice, ascending |
| `rolls_left` | integer | uint8 | rerolls left: 2, 1 or 0 |
| `notation` | string | utf8 | the situation in the stable notation ([notation](notation.md)) |
| `options` | array of objects | list of struct | every legal option, in legal-action order |
| `options[].code` | integer | int16 | the action code ([notation](notation.md#action-codes)) |
| `options[].type` | `score` or `keep` | utf8 | |
| `options[].notation` | string | utf8 | the action in the stable notation, e.g. `keep 6 6` |
| `options[].value` | number | float64 | the exact expected remaining score from this decision on, choosing this option (its own points included) |
| `best_value` | number | float64 | the largest option value |
| `chosen` | integer or null | int16, nullable | the code of the action taken (trajectories); null for uniform samples |
| `random` | boolean or null | boolean, nullable | true when the perturbation drew the action at random (it may still be a best action); false for deliberate choices, including all optimal play; null for uniform samples |

- **Values** are expected *remaining* scores ([queries](queries.md)); the expected final score of an option is
  `score_so_far + value`. At every decision, `score_so_far` plus the points still to come is the final score.
- **Perturbed data:** `random` tells a random pick that happened to be a best action from a deliberate one, so
  learners can filter or weight the perturbed decisions.
- **Ties** are carried by the values themselves: several options can share the best value. Do not use `chosen`
  as a label for the best option; it is the action the sampler took (optimal play breaks ties by legal order).
- **Floats** in JSON are written in the shortest form that reads back to the same f64; a correctly rounding
  parser (Python's `json`, serde_json with `float_roundtrip`) recovers them exactly.
- The schema, the notation and the action codes are part of the stable contract from the first release: fields
  may be added in a later version, never renamed or removed without a major version.

## Size and speed

Each format has one schema; there is no compact JSON variant. For more than about 100 000 rows use Parquet.

100 000 optimal rows of Scandinavian Yatzy: about 207 MB as JSON Lines, 14 MB as Parquet (142 bytes per row),
written in under 2 s on an M1 Max under heavy load (provisional).
