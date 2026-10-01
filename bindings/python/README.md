# yatzy-solver (Python)

Exact solver, rules engine, simulator and data exporter for Scandinavian Yatzy and American rules
(Yahtzee-compatible): the optimal expected final score of every decision in every situation. The Python package
wraps the Rust library; values are bit-identical to the Rust and WebAssembly packages.

```sh
pip install yatzy-solver
```

```python
import yatzy_solver as ys

solver = ys.Solver.build(ys.Variant("yatzy-scandinavian"))   # or "american"; a couple of seconds
sit = "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -"
for action, value in solver.option_values(sit):
    print(f"{value:10.4f}  {action}")
print(solver.best_options(sit), solver.regret(sit, "keep 6"))
```

Values are expected remaining scores; add the score so far for the expected final score. Batch queries take and
return numpy arrays, simulations can use a policy written in Python, and `export` writes labelled situations as
JSON Lines or Parquet.

See the [repository](https://github.com/precisit/yatzy-solver) for the rules, the notation, the validation
against the published values, and the full documentation. License: MIT.
