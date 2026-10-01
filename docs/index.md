# yatzy-solver

Exact optimal play for Scandinavian Yatzy and American rules (Yahtzee-compatible): the expected final score of
every decision in every situation, from Rust, Python and JavaScript.

**[Open the Yatzy advisor](https://precisit.github.io/yatzy-solver/advisor/)**: enter your score card and dice,
and see every option with its expected final score and what it loses against the best. It works offline after
the first visit, in Swedish and English.

## Install

```sh
cargo add yatzy-solver          # Rust library
cargo install yatzy-solver-cli  # the yatzy-solver command
pip install yatzy-solver        # Python
npm install yatzy-solver        # JavaScript (WebAssembly)
```

## Documentation

- [Rules](rules.md): the variants, house-rule switches and joker rules.
- [Notation](notation.md): the stable text form of situations and actions, and the action codes.
- [Solver](solver.md): the method, verification, table format and performance.
- [Queries and simulation](queries.md): option values, best options, regret, batch queries, the seeded simulator.
- [Export](export.md): sampled situations with every option's value, JSON Lines or Parquet.
- [Python](python.md): the `yatzy_solver` package.
- [WebAssembly and the web advisor](wasm.md): the npm package, table loading, and the offline advisor.

The source, the validation against the published values, and the changelog are in the
[repository](https://github.com/precisit/yatzy-solver).

"Yatzy" is used as the name of the game; this project is not affiliated with any trademark holder. "Yahtzee" is a
trademark of Hasbro; this project is not affiliated with Hasbro.
