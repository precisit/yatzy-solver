# yatzy-solver (WebAssembly)

Exact solver, rules engine and advisor for Scandinavian Yatzy and American rules (Yahtzee-compatible), compiled
to WebAssembly from the Rust library. Values are bit-identical to the Rust and Python packages.

```js
import * as ys from "yatzy-solver";          // Node: ready to use. Browser: `await ys.default()` first.

const v = new ys.Variant("yatzy-scandinavian");
const solver = ys.Solver.fromTable(v, tableBytes); // a table file from `yatzy-solver build`, or Solver.build(v)
for (const o of solver.optionValues("dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -")) {
  console.log(o.action, o.value);
}
```

See the repository's `docs/wasm.md` for the API, the table download and the numbers.
