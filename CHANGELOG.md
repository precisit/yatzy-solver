# Changelog

All notable changes to this project are recorded here. The project follows semantic versioning. The stable
contracts (the notation, the action codes, the table format, the export schema, the generator, and the Python
and npm APIs) change only with a major version; fields and functions may be added in minor versions.

## 1.0.0 (2026-10-01)

The first release.

- **Solver.** Exact backward induction for Scandinavian Yatzy and American rules (Yahtzee-compatible), with the
  house-rule switches `+fh5`, `+tp4` and `+forced`, and the joker rules `+forced-joker`, `+no-joker` and
  `+no-bonus`. Deterministic f64 arithmetic: the value arrays are bit-identical on every platform.
- **Validation.** American rules 254.5896 and 245.87 (without bonus and joker) as published; Scandinavian Yatzy
  248.4399894 under the stated rules, and the published 248.63 reproduced under the scoring of its authors'
  code (a bug, documented). An exact brute-force solver agrees in rational arithmetic on nine reduced games.
- **Rules engine and notation.** Legal actions, scoring, bonuses and jokers; a stable text notation and a fixed
  action code space.
- **Queries.** Option values, best options with explicit ties, regret, and batch queries (flat and dense
  layouts).
- **Simulator.** Seeded games under the optimal policy or any policy, with a documented generator (version 1)
  whose dice do not depend on the policy.
- **Export.** Optimal, perturbed and uniform samples with every option's value, as JSON Lines or Parquet.
- **Table files.** A versioned, checksummed, memory-mappable format; prebuilt f32 and f64 tables.
- **Bindings.** Python (`yatzy-solver` on PyPI, numpy batch functions; abi3 wheels for Python 3.9 and later on
  Linux x86-64 and arm64, macOS arm64 and x86-64, and Windows x86-64, plus the sdist) and WebAssembly
  (`yatzy-solver` on npm, with provenance), bit-identical to Rust on a golden set of 300 situations.
- **Web advisor.** An offline page in Swedish and English that shows every option with its expected final score
  and its loss against the best.
- **CLI.** `yatzy-solver build | query | simulate | export | verify`.
