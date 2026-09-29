# yatzy-solver: implementation specification v0.1

> **What this is.** The specification for this repository. It is self-contained: build to it milestone by
> milestone (section 8). Nothing in the project depends on, or mentions, AI models; section 6 lists what a client
> that learns from the solver needs, as ordinary library features. Changes to this spec are made in this file,
> in their own commits, with a line in the changelog at the end.

## 0. Summary

Build the reference open-source library for playing Scandinavian Yatzy (and American-rules Yahtzee) optimally: an
exact dynamic-programming solver that gives the expected final score of every decision in every situation, with a
rules engine, a simulator and a data exporter. It should be fast, validated against published results, usable from
Rust, Python and JavaScript (WebAssembly), and released under a permissive license.

**Name: `yatzy-solver`** (repository `precisit/yatzy-solver`). As of 2026-09-29 the name is free on crates.io,
PyPI and npm; so are the alternatives `optimal-yatzy` and `yatzy-oracle`. See 9.3 on trademarks before the first
package is published.

## 1. Background and prior work

- **Yahtzee (American rules)** was solved for solitaire play (maximize the expected final score) by Tom Verhoeff
  (1999) and independently by James Glenn: expected score **254.5896**, standard deviation **59.6117**, median
  **248** ([Verhoeff, trivia page](https://www-set.win.tue.nl/~wstomv/misc/yahtzee/trivia.html);
  [Glenn, "Computer Strategies for Solitaire Yahtzee", CIG 2007](http://www.cs.loyola.edu/~jglenn/Papers/yahtzee_cig2007_glenn.pdf),
  which also covers symmetry optimizations).
- **Scandinavian Yatzy** was solved in two KTH bachelor theses: expected score **248.63**
  ([Larsson and Sjöberg 2012](https://www.csc.kth.se/utbildning/kth/kurser/DD143X/dkand12/Group89Michael/report/Larsson+Sjoberg.pdf);
  [Sederblad and Törnebohm 2013](https://www.diva-portal.org/smash/get/diva2:676659/FULLTEXT01.pdf)).
  The 248.63 comes from a bug in the 2012 authors' code (their Two pairs scorer, `ScoreCard.scorePair` in
  `ansjob/optimalt-yatzy`, lets a single pair score); under their stated rules the value is **248.4399894**,
  confirmed by the independent open-source solvers Castux/yahtzee and Laurii1i/Yatzy (see 10.1).
- **Multiplayer** play (maximize the probability of winning) is harder; nearly optimal play is studied by
  Pawlewicz, "Nearly Optimal Computer Play in Multi-player Yahtzee" (Computers and Games 2010).
- **Existing code** is small and fragmented: a few Python and C++ Yahtzee solvers on GitHub (single-digit to tens
  of stars), a Scandinavian Yatzy trainer (`Laurii1i/Yatzy`), rule katas, and games. There is no maintained,
  tested, fast library that covers Scandinavian Yatzy, publishes its tables, and offers per-decision values from
  several languages. That is the gap.
- **Reinforcement-learning** work on Yahtzee compares against the optimal solitaire player (e.g. arXiv:2601.00007,
  2026), which makes an exact, easy-to-use reference valuable beyond players.

Credit all of these in the README.

## 2. Rules

Rules are data: a **variant** defines dice, categories, scoring, bonuses and special rules. The engine implements
variants from those definitions; every scoring rule has table-driven unit tests.

### 2.1 Scandinavian Yatzy (the reference variant, `yatzy-scandinavian`)

- Five six-sided dice; each turn up to three rolls. After the first and second roll the player keeps any subset
  and rerolls the rest. The player may score after any roll.
- 15 turns, 15 categories, each filled exactly once, in any order. A category may be filled with 0 points (a
  scratch).

| category | score |
| --- | --- |
| Ones ... Sixes | the sum of the dice showing that face |
| **Bonus** | 50 if the upper section (Ones to Sixes) totals at least 63 (not a category) |
| One pair | the sum of the highest pair (2 x v) |
| Two pairs | two pairs of different values (2a + 2b) |
| Three of a kind | 3 x v |
| Four of a kind | 4 x v |
| Small straight | 1-2-3-4-5: 15 |
| Large straight | 2-3-4-5-6: 20 |
| Full house | three of one value and two of another: the sum of the five dice |
| Chance | the sum of the five dice |
| Yatzy | five of a kind: 50 |

The maximum score is 374. **House-rule switches** (defaults in brackets):
- whether five of a kind counts as a full house [no];
- whether four of a kind counts as two pairs [no];
- **forced order** ("tvångsyatzy": categories filled top to bottom) [off].

Settled in M2 (10.1): the defaults above are the rules of the published solution.

### 2.2 American rules, Yahtzee-compatible (`american`, the validation variant)

- 13 categories: Ones to Sixes; three and four of a kind (the sum of all dice); full house 25; small straight
  (any four in sequence) 30; large straight 40; Yahtzee 50; chance.
- Upper bonus 35 at 63.
- **Yahtzee bonus and jokers:** each further Yahtzee scores 100 if the Yahtzee box holds 50. With the forced-joker
  rule, the upper box of that face must be used if open; otherwise any lower box, with full points for full house
  and the straights.
- The state gains a Yahtzee-box status (open, 0, or 50).
- **Ids** (9.3): the variant id is `american`, the Yahtzee category id is `five_of_a_kind` (its display name may
  say Yahtzee), and the notation field for the box status is `five_of_a_kind 0|50`. No id, table header, export,
  notation or package name contains "yahtzee".
- **Joker rule as a switch** (found in M2): the published 254.5896 uses Verhoeff's reading, not the forced
  joker: the joker (full points for full house and the straights) applies when the Yahtzee box and the upper box
  of the hand's face are both filled, and the hand may be scored in any open box. `american` uses that rule;
  `american+forced-joker`, `american+no-joker` and `american+no-bonus` give the alternatives (docs/rules.md).
  The printed rules use the forced joker; the two differ by 0.0019 points of expected score.

This variant exists to validate the solver against the best-known published number.

### 2.3 Later variants (not in v1)

- **Maxi Yatzy:** six dice, 20 categories, saved rolls. The state space grows sharply; do a feasibility study first.
- Other national rule sets, if requested by users.

## 3. The mathematical model

- **Between turns**, the state is (the set of filled categories, the upper-section sum capped at 63), plus any
  variant flags. Yatzy: 2^15 x 64 = 2 097 152 states, fewer of them reachable.
- **Within a turn**, the situation is (the between-turn state, the five dice as a sorted multiset, the rolls left:
  2, 1 or 0). There are 252 distinct multisets of five dice. A keep is a sub-multiset of the current dice: at most
  32 subsets per roll, fewer distinct ones, 462 keep multisets in total.
- **Objective (v1): maximize the expected final score**, bonus included. Solve by backward induction over the
  number of filled categories. Within a turn:
  1. value each final roll by the best category choice;
  2. value keeps from the probabilities of the rerolled dice;
  3. value rolls by the best keep;
  4. repeat for the second and first roll.

  Use the standard incremental keep computation (values of keeps of size k from those of size k + 1) rather than
  enumerating every transition.
- **Stored table:** V(state) = the expected remaining score from the start of a turn in that state (compute in f64;
  store f32 and optionally f64).
- **Derived, on demand:** the value of every legal option in any situation (every keep, every category), computed
  from V in microseconds. These per-decision values are the main product (section 6).
- **Objectives in v1.1:**
  - the exact distribution of the final score under the optimal-expectation policy (for the standard deviation,
    percentiles and the median);
  - the policy that maximizes P(final score >= T) for a target T, a common heuristic for multiplayer endgames.
- **Optional optimizations:** reachability pruning, and symmetries (Glenn).

## 4. Functional requirements

- **F1. Build.** Build the table for a variant deterministically into a versioned file with a header and a
  checksum (format in 5.4). The table can also be downloaded prebuilt.
- **F2. Query.**
  - `state_value(state)`;
  - `option_values(situation)`: all legal options with their exact expected values;
  - `best_options(situation)`: all maxima, with ties made explicit;
  - `regret(situation, option)`: the expected points lost against the best option.
- **F3. Batch.** Vectorized queries for millions of situations: numpy arrays in and out in Python, typed arrays in
  JavaScript.
- **F4. Simulate.** Play N games with a seeded random generator under the optimal policy or a caller-supplied
  policy. Return the score distribution and per-game logs.
  - **The generator is a stable contract, version 1** (docs/queries.md): xoshiro256** seeded by SplitMix64,
    with one dice stream per (seed, game, turn), derived so that distinct (game, turn) pairs cannot collide,
    and every roll drawing a full block of dice (a reroll of m dice uses the first m). The policy has a
    separate stream per game. The dice therefore do not depend on the policy (common random numbers), so two
    players can be compared on identical dice.
  - Logs and exports record the generator version (`rng 1`); changing the generator means a new version.
  - `best_action` and the optimal simulator break ties by taking the first best option in legal order. This is
    a reproducibility rule, not a preference, and not a training label.
- **F5. Export situations.** Sample situations with all their option values, from:
  - optimal-play trajectories;
  - trajectories with a configurable share of random or perturbed decisions, which cover states a good player
    rarely reaches;
  - uniform sampling over reachable states.

  Deterministic with a seed. Formats: Parquet and JSON Lines, with a documented schema (5.5).
- **F6. Stable notation.** A canonical, documented text form for situations and options, e.g. `dice 1 3 3 5 6 |
  rolls 2 | upper 21 | filled ones,twos,chance` and `keep 3 3` / `score full_house`. Used in logs, tests, exports
  and user interfaces, and stable across versions. Its bytes are part of the golden tests.
- **F7. Rules engine.** Legal options, applying an option, scoring, game over, final score, as a small public API.
  Usable on its own as a game's logic from every binding (web and apps).
- **F8. Verification tools.**
  - A brute-force reference solver for reduced games (fewer categories and dice) that checks the fast solver
    exactly.
  - The golden published values.
  - Invariant checks: reroll probabilities sum to 1; values are bounded; no option beats the best.

## 5. Non-functional requirements

### 5.1 Implementation

- **Core in Rust** (stable, no `unsafe` outside the bindings): crates `yatzy-solver` (library) and a CLI
  (`yatzy-solver build | query | simulate | export | verify`).
- **Python bindings** (PyO3 + maturin), on PyPI as `yatzy-solver`, with numpy arrays for the batch functions.
- **WebAssembly** (wasm-bindgen), on npm as `yatzy-solver`, for browsers and Node: the rules engine and queries,
  with the table loaded from a URL or bundled.
- **A C ABI** in v1.1, for Swift and iOS.
- No network access and no telemetry.

### 5.2 Performance targets (to confirm in milestone M2)

- Build the Scandinavian Yatzy table in under 5 minutes on one core of an Apple M1, and under 1 minute on all
  cores.
- `state_value` under 1 µs; `option_values` for one situation under 50 µs.
- Batch labelling of at least 100 000 situations per second per core.
- Table size: 8 MB (f32) for Yatzy.

### 5.3 Correctness

- Deterministic results: a fixed summation order, and the same **values hash** (SHA-256 of the value array) on
  every platform, published per release. The file hash also covers the header, which includes the solver
  version, so it changes with every release.
- Tests:
  - scoring tables for every category and house-rule switch;
  - property tests;
  - the brute-force cross-check;
  - cross-language parity (Rust, Python and WASM give identical answers on a golden set);
  - the published values: **american 254.5896**, standard deviation 59.6117, median 248; **yatzy-scandinavian
    248.4399894** (the stated rules); and, as a check, 248.63 reproduced under the scoring of the published code
    (10.1).

### 5.4 Table file format

- A header (magic, format version, variant id, a hash of the variant definition, solver version, objective,
  precision, state count), then the value array in a documented state order, then a SHA-256 checksum.
- Two hashes per table: the values hash (the value array only; identical on every platform) and the file hash
  (the whole file, header included).
- Little-endian, memory-mappable.
- Prebuilt tables are attached to each GitHub release, with their hashes in the README.

### 5.5 Export schema (F5)

Per row:
- variant id and solver version;
- turn number, filled-category mask, upper-sum, score so far;
- dice (sorted), rolls left;
- the stable notation (F6);
- the list of legal options, each with its type, notation and exact expected value;
- the best value;
- the sampling source (optimal, perturbed, uniform), the seed and the generator version.

### 5.6 Quality of the release

- A README with the rules, a short explanation of the method, the validation numbers, install lines for cargo, pip
  and npm, and examples.
- Documentation on docs.rs and a small docs site.
- A CHANGELOG and semantic versioning.
- CI on Linux, macOS and Windows: tests, wheels, the npm package, and publishing on tags.

## 6. What an AI-training client needs (as ordinary features)

Any client that learns to play from the solver needs exactly these, all
covered above:
1. **Exact values for every legal option in a situation**, not only the best move (F2), so a learner can be
   trained on the full ranking and measured by regret.
2. **Many situations, well spread** (F5): optimal play alone rarely visits bad states that learners and humans
   reach, so the perturbed and uniform samplers matter.
3. **Speed at scale** (F3, 5.2): tens of millions of labelled situations in minutes.
4. **A stable notation and schema** (F6, 5.5), so datasets stay valid across versions.
5. **Evaluation tools**: regret per decision (F2) and full-game simulation with a caller-supplied policy (F4), for
   comparing any player against optimal play.
6. **The same rules everywhere** (F7): the rules engine in Rust, Python and WASM, so the training data and a game
   built on it cannot disagree.

## 7. Release plan: what makes it appreciated

- **Correct first:** reproduce the published numbers and say so on the first line of the README, with the
  commands to verify them.
- **Easy to use:** `pip install yatzy-solver`, `npm install yatzy-solver` and `cargo add yatzy-solver`, with a
  ten-line example each; prebuilt tables so nobody has to build first.
- **A small web demo:** "the optimal Yatzy advisor". Enter or roll dice, see the best keep or category, and see
  how many points each alternative loses. A static page using the WASM package, in Swedish and English.
- **Both Nordic and American rules**, with the house-rule switches explained.
- **Credits** to Verhoeff, Glenn, Pawlewicz and the KTH theses.
- **Launch:** a short write-up (the Precisit blog), crates.io, PyPI and npm, Hacker News ("Show HN"), and board-game
  and Nordic communities.
- **Maintenance promise:** a scope that stays small; issues and discussions open.

## 8. Milestones and acceptance

| milestone | content | accepted when |
| --- | --- | --- |
| M1 | variants, scoring and rules engine (F7), notation (F6) | all scoring tables pass; rules documented |
| M2 | solver and table (F1), brute-force cross-check (F8) | reduced games match brute force exactly; american 254.5896; yatzy-scandinavian 248.4399894, and 248.63 under the published code's scoring; performance targets measured |
| M3 | queries, batch, simulator (F2-F4) | regret and option values consistent with V; the simulated mean within its interval of the exact mean |
| M4 | Python bindings and export (F5) | parity with Rust on the golden set; export schema documented and tested |
| M5 | WASM, npm and the web advisor demo | parity; the demo works offline in a browser |
| M6 | release: docs, CI, packages, v1.0.0 | a clean install on the three OSs; the README's verify commands reproduce the numbers |
| M7 (v1.1) | the score distribution and P(score >= T) objective; C ABI | american standard deviation 59.6117 and median 248 reproduced |

## 9. Decisions

1. **License: MIT** (decided 2026-09-29), for the code and the prebuilt tables.
2. **Visibility:** the repository is private for now; making it public is the owner's decision, expected once M2
   reproduces the published values. Do not publish packages (crates.io, PyPI, npm) before that decision.
3. **Names and trademarks.** "Yahtzee" is a Hasbro trademark: call that variant "American rules
   (Yahtzee-compatible)" in docs and code, and never use "Yahtzee" as a product or package name.

   **"Yatzy" in Sweden (checked 2026-09-29):**
   - **The Swedish national register (PRV)** has no registration of the plain word "Yatzy". There are registered
     compound marks: YATZY CASINO, YATZY LAS VEGAS and YATZY MACAU (all classes 9 and 28, valid until 2033).
     YATZY MONTE CARLO was withdrawn, Jätte Yatzy dismissed, and YATZY LOTTEN lapsed in 2009.
   - **EU trade marks**, which also cover Sweden, are compound marks too: Zoo Yatzy, farm yatzy, Yatzy ultimate
     (figurative), WORD YATZY and Yatzy Blitz (classes 9, 16, 28, 41, 42).
   - **International registrations designating Sweden** (WIPO Global Brand Database) add nothing beyond those EU
     marks.

   So the game name itself is not registered in Sweden; the registered marks protect particular compound names.
   Using "Yatzy" descriptively for a solver of the game, as in `yatzy-solver`, should be low risk. Avoid
   product names that resemble a registered compound (for example "Yatzy" plus a place or a brand word). A US
   filing for "YATZY" exists (serial 86454511, status not verified). This is a register check, not legal advice:
   before a commercial app is named, have an IP lawyer confirm, including Norway, Denmark, Finland and the US.

## 10. Open questions

1. The exact rules behind the published 248.63: full house with five of a kind, two pairs from four of a kind,
   straights. Confirm from the KTH theses and document any difference.
   **Resolved in M2:** the stated rules of Larsson and Sjöberg (2012) match the `yatzy-scandinavian` defaults,
   which give 248.4399894 (confirmed independently by the open-source solvers Castux/yahtzee, 248.4394, and
   Laurii1i/Yatzy, about 248.44). Their published code scores a single pair in Two pairs (the second pair
   counts 0 when absent); with that scoring (a verification variant, not a house rule) the solver gives
   248.6328539, which rounds to 248.63. The house-rule defaults stay as in 2.1. **Decided (owner, after the M2
   review):** the Scandinavian validation value is 248.4399894; 248.63 is kept as a check reproducing the
   published code.
2. Whether Maxi Yatzy (six dice, saved rolls) is feasible exactly, or only approximately.
3. Multiplayer: an exact win-probability solver is out of scope for v1; the P(score >= T) objective is the
   practical step.
4. Verhoeff's trivia page also gives an exact fraction (Liese and Kelly, 2017) that evaluates to 254.58937,
   not 254.5896. It matches none of the free, forced or no-joker rules (254.5896095, 254.5877287,
   253.9702412). Which rules it uses is unknown; it does not affect the validation value. Recorded, not pursued.
5. Castux/yahtzee publishes 248.4394 for Scandinavian Yatzy; this solver gives 248.4399894, a gap of 0.0006.
   Is it a rule difference or rounding or precision in their code? It does not block anything.
   **Resolved in M3:** the rules are identical (their `Yatzy.cs`). Castux computes in single precision and
   builds each outcome probability by adding `(float)(1 / 6^n)` once per ordered roll into a float, so the
   distributions do not sum to 1 (five dice: 1 - 2.98e-7); value leaks at every expectation. A copy of their
   float arithmetic gives 248.4394531, their published tables give the same, and the same solve in f64 with
   exact probabilities gives 248.4399894.

## Changelog

- 2026-09-29: v0.1, initial specification; license MIT; repository private.
- 2026-09-29: 2.2 records the joker rule as a switch (Verhoeff's reading gives 254.5896); 10.1 resolved (248.63
  comes from the published code's Two pairs scoring; 248.44 under the stated rules); 10.4 added.
- 2026-09-29: decisions after the M2 review: Scandinavian validation value 248.4399894 (sections 1, 5.3, M2),
  with 248.63 as a check of the published code; ids `american` and `five_of_a_kind` (2.2); values hash versus
  file hash (5.3, 5.4); 10.1 decided; 10.5 added.
- 2026-09-29: 10.5 resolved (single-precision probabilities in Castux/yahtzee; same rules).
- 2026-09-30: F4 fixes the generator contract (version 1: per-turn dice streams, a separate policy stream) and
  the tie-breaking rule; 5.5 export rows carry the generator version.
- 2026-09-29: 9.3, trademark check for "Yatzy" in Sweden (PRV, EUIPO, WIPO): no registration of the plain
  word; the name `yatzy-solver` is kept.
