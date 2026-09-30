# yatzy-solver-cli

The `yatzy-solver` command: build tables, query situations, simulate games, export labelled data, and verify
the published values.

```sh
cargo install yatzy-solver-cli
yatzy-solver verify                                   # the brute-force cross-check and the published values
yatzy-solver build --variant yatzy-scandinavian       # tables/yatzy-scandinavian.f32.yzt
yatzy-solver query "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -"
yatzy-solver simulate --variant american --games 1000000 --seed 2026
yatzy-solver export --source perturbed --perturb 0.2 --rows 1000000 --format parquet --out rows.parquet
```

See the [repository](https://github.com/precisit/yatzy-solver). License: MIT.
