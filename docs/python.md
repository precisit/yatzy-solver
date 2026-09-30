# Python

The `yatzy_solver` package wraps the Rust library with PyO3: the rules engine, queries, numpy batch functions,
the simulator and the export. One wheel (abi3) serves Python 3.9 and later. Values are bit-identical to the Rust
library's; the golden set (`golden/parity.jsonl`) checks this.

## Install

Not yet published. From a clone, with [uv](https://docs.astral.sh/uv/):

```sh
cd bindings/python
uv venv && uv pip install maturin numpy
uv run maturin develop --release
```

## Example

```python
import yatzy_solver as ys

v = ys.Variant("yatzy-scandinavian")          # or "american"; ys.builtin_variants() lists all
solver = ys.Solver.build(v)                   # a few seconds; or ys.Solver.load("tables/...yzt", v)

sit = "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -"
for action, value in solver.option_values(sit):
    print(f"{value:10.4f}  {action}")
print(solver.best_options(sit), solver.regret(sit, "keep 6"))
```

Values are expected **remaining** scores; add the score so far for the expected final score
([queries](queries.md)).

## API

| | |
| --- | --- |
| `Variant(id)` | `id`, `name`, `dice`, `rolls`, `categories`, `num_action_codes`, `max_options` |
| rules engine | `score(category, dice)`, `start_turn(state, dice)`, `legal_actions(situation)`, `apply_keep(situation, keep, rolled)`, `apply_score(state, dice, category)`, `is_over(state)` |
| codes | `action_code(action)`, `action_notation(code)` ([action codes](notation.md#action-codes)) |
| arrays | `situation_arrays(situations)` -> dict of numpy arrays; `dense(codes, values, counts)` -> `n x num_action_codes` |
| `Game(variant)` | a score card: `score(dice, category)`, `state`, `total`, `points(category)`, `upper_bonus`, `bonus`, `is_over` |
| `Solver.build(variant)`, `Solver.load(path, variant)`, `Solver.build_table(variant, path, precision)` | |
| queries | `state_value(state)`, `option_values(situation)`, `best_options(situation)`, `best_action(situation)`, `regret(situation, action)`, `tie_epsilon`, `precision` |
| batch (numpy, GIL released, all cores) | `state_values(filled, upper, armed)`; `option_values_batch(filled, upper, armed, dice, rolls_left)` -> `(codes, values, counts)` |
| simulation | `simulate(games, seed=0, logs=False, policy=None)` -> dict with `scores` (uint16 array), `mean`, `std_dev`, `std_error`, `min`, `median`, `max`, `rng`, `logs` |
| export | `export(path, rows, source="optimal", seed=0, format="jsonl", perturb=0.1)` ([export](export.md)) |

Situations, states and actions are strings in the [stable notation](notation.md). Invalid input raises
`ValueError`.

### Batch

```python
arr = v.situation_arrays(list_of_situations)
codes, values, counts = solver.option_values_batch(arr["filled"], arr["upper"], arr["armed"], arr["dice"], arr["rolls_left"])
dense = v.dense(codes, values, counts)        # n x (C + 210), NaN where illegal
```

`codes` (int16) and `values` (float64) are `n x (C + 31)`, padded with -1 and NaN; `counts` (uint16) gives
the options per row. About 104 000 situations per second on one thread and 256 000 on all threads, on an M1 Max
under heavy load (provisional).

### A policy of your own

```python
def my_policy(situation, legal_actions, score_so_far):
    return legal_actions[0]                   # an action from the list, in notation

result = solver.simulate(10_000, seed=1, policy=my_policy)
optimal = solver.simulate(10_000, seed=1)     # the same dice (common random numbers)
print(optimal["mean"] - result["mean"])
```

The dice depend only on the seed, the game and the turn, so both runs see the same rolls wherever the policies
reach the same point; the difference of the means has far less noise than with independent dice. Exceptions
raised by the policy propagate.
