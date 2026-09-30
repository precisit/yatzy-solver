# WebAssembly and the web advisor

The npm package `yatzy-solver` is the Rust library compiled to WebAssembly with wasm-bindgen: the rules
engine, queries, batch queries on typed arrays, and table loading, on one thread. Values are bit-identical to
the Rust and Python packages: CI runs the golden set (`golden/parity.jsonl`, 300 situations) through the WASM
single and batch APIs in Node, and compares with `Object.is`.

The package is not published yet.

## Build

```sh
wasm-pack build bindings/wasm --release --target web --out-dir pkg --out-name yatzy_solver
node bindings/wasm/scripts/package.mjs     # package.json for `yatzy-solver`, the Node entry, license
node bindings/wasm/tests/parity.mjs        # golden parity
```

The package works in browsers (`import init, * as ys from "yatzy-solver"; await init();`) and in Node, where
the `node` export condition loads the module synchronously (`import * as ys from "yatzy-solver"`).

## API

The same shape as the Python package: notation strings for single queries, typed arrays for batches.

| | |
| --- | --- |
| `builtinVariants()`, `solverVersion()`, `rngVersion()` | |
| `new Variant(id)` | `id`, `name`, `dice`, `rolls`, `categories`, `categoryNames`, `numActionCodes`, `maxOptions`, `actionCode(a)`, `actionNotation(code)` |
| rules engine | `score(category, dice)`, `scoreIn(situation, category)` (points now, bonuses and jokers included), `startTurn(state, dice)`, `legalActions(situation)`, `applyKeep(situation, keep, rolled)`, `applyScore(state, dice, category)` -> `[nextState, points]`, `isOver(state)` |
| `new Game(variant)` | `score(dice, category)`, `state`, `total`, `upperBonus`, `bonus`, `isOver`, `points(category)` |
| `Solver.fromTable(variant, bytes)`, `Solver.build(variant)`, `Solver.buildTable(variant, precision)` | |
| queries | `stateValue(state)`, `optionValues(situation)` -> `[{action, value, code}]`, `bestOptions`, `bestAction`, `regret(situation, action)`, `precision`, `tieEpsilon` |
| batch | `stateValues(filled: Uint32Array, upper: Uint16Array, armed: Uint8Array)`; `optionValuesBatch(filled, upper, armed, dice: Uint8Array (n x 5), rollsLeft: Uint8Array)` -> `{rows, width, codes: Int16Array, values: Float64Array, counts: Uint16Array}` (the flat layout, [notation](notation.md#action-codes)) |
| simulation | `simulateOptimal(games, seed)` -> final scores (generator version 1, [queries](queries.md)) |

## Loading a table: download or solve in the browser

Measured on a MacBook Air (Apple M5, 10 cores, 32 GB, macOS 27, on mains power) at load average 1.6 to 4,
2026-09-30, with Node 26.7 (V8, the engine in Chrome). The in-browser figure was checked in Chrome on another
machine (an M1 Max under load average 34: 10.2 s for American rules).

| | Scandinavian Yatzy | American rules |
| --- | --- | --- |
| `.wasm` | 174 631 bytes (about 70 KB gzipped) | same module |
| f32 table (gzip -9) | 8 388 768 bytes (4 670 491) | 4 194 464 bytes (1 753 847) |
| load a table (`Solver.fromTable`, checksum and all) | 0.018 s | 0.011 s |
| solve in WASM, one thread | 10.9 s | 5.0 s |
| solve natively, one core / all cores | 10.8 s / 1.7 s | 4.6 s / 0.76 s |
| option values of one situation, 2 rerolls left, WASM | 20 µs | 26 µs |

Decision, as recorded in the advisor:

- **Download the f32 table** (8 MB; 4.7 MB if the server compresses it) by default: it is ready in well under a second
  once fetched, on any device. The worker checks its SHA-256 against `tables/manifest.json`, keeps it in the
  Cache API, and loads it in a Web Worker, so the page never blocks.
- **Solve in the worker** as the fallback when no table can be fetched (and as an independent check): about
  11 s for Scandinavian Yatzy on a current laptop, longer on phones, with the page responsive throughout.

**An engine detail that mattered.** V8 compiles WebAssembly first with a fast baseline compiler and replaces a
function with optimized code only between calls. When the per-state work was inlined into the single long
call that solves the table, the first solve in a process ran in unoptimized code: 26 s instead of 5 s for
American rules, 71 s instead of 11 s for Scandinavian Yatzy. Keeping the per-state function out of line
(`#[inline(never)]`, called about two million times) fixed it. **Do not remove that attribute to "optimize"
the native build:** it exists for the WebAssembly engines, and the comment on `TurnModel::state_value` says so. Natively it costs about 4% on one core (10.4 s
to 10.8 s); the table hashes are unchanged.

## The advisor

`demo/` is a static page (no framework, no bundler), in Swedish and English:

- enter the score card and the dice (or roll), and see **every option** with its exact expected final score
  (score so far plus the expected remaining score) and **its loss against the best**, ties marked;
- click an option to play it: a keep rerolls the other dice, a category fills its box;
- Scandinavian Yatzy and American rules (Yahtzee-compatible); no logos or trade dress.

Loading follows the decision above. A service worker caches the page and the WASM package, and the worker
caches the tables, so after the first visit the advisor works offline.

**Updates.** `demo/build.mjs` writes a build id (a hash of the shell, the WASM package and the table manifest)
into the service worker, which names its cache after it and deletes older shell caches when it activates, so
a new release reaches returning visitors on their next load or the one after. The table manifest is fetched
network-first (the cached copy only offline), and table files are named by their hash prefix
(`yatzy-scandinavian.f32.e933212b.yzt`), so a URL never changes content; the worker drops cached tables the
manifest no longer lists. Checked by hand in Chrome (2026-09-30): after changing a shell string and rebuilding,
the first reload installed the new shell and deleted the old cache, and the second showed the new text; after
rebuilding the tables (as f64, new hashes), the first reload downloaded the new table and dropped the old one.

Build it with `node demo/build.mjs` (after the WASM package and the release CLI), which writes `demo/dist` with
the tables and their manifest. Verified in Chrome: first load from the network, reload from the cache, a
reload with the server stopped (offline), both languages, keeps and scoring, and the fallback solve for a
variant whose table was not cached.
