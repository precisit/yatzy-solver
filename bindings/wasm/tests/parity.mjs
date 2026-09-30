// Cross-language parity (SPEC 5.3): the golden set through the WASM single and batch APIs, bit for bit.
// Run with `node tests/parity.mjs` after building the package (pkg/).
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import * as ys from "../pkg/node.js";

const golden = readFileSync(new URL("../../../golden/parity.jsonl", import.meta.url), "utf8")
  .trim()
  .split("\n")
  .map((l) => JSON.parse(l));
assert.equal(golden.length, 300);

for (const id of ["yatzy-scandinavian", "american"]) {
  const v = new ys.Variant(id);
  const t0 = performance.now();
  const s = ys.Solver.build(v);
  console.log(`${id}: solved in ${((performance.now() - t0) / 1000).toFixed(1)} s`);
  const rows = golden.filter((r) => r.variant === id);

  // Single queries.
  for (const r of rows) {
    const got = s.optionValues(r.situation);
    assert.deepEqual(got.map((o) => o.code), r.options.map((o) => o[0]), r.situation);
    // Object.is: the same f64 bits, not merely close.
    got.forEach((o, i) => assert.ok(Object.is(o.value, r.options[i][1]), `${r.situation} option ${i}`));
    assert.ok(Object.is(s.stateValue(r.situation.split(" | ").slice(2).join(" | ")), r.state_value));
  }

  // Batch queries from typed arrays.
  const n = rows.length;
  const filled = new Uint32Array(n), upper = new Uint16Array(n), armed = new Uint8Array(n);
  const dice = new Uint8Array(n * 5), rollsLeft = new Uint8Array(n);
  rows.forEach((r, i) => {
    const f = Object.fromEntries(r.situation.split(" | ").map((x) => [x.split(" ")[0], x.slice(x.indexOf(" ") + 1)]));
    const cats = v.categories;
    filled[i] = f.filled === "-" ? 0 : f.filled.split(",").reduce((m, c) => m | (1 << cats.indexOf(c)), 0);
    upper[i] = Number(f.upper);
    armed[i] = f.five_of_a_kind === "50" ? 1 : 0;
    f.dice.split(" ").forEach((d, j) => (dice[i * 5 + j] = Number(d)));
    rollsLeft[i] = Number(f.rolls);
  });
  const flat = s.optionValuesBatch(filled, upper, armed, dice, rollsLeft);
  const [codes, values, counts, w] = [flat.codes, flat.values, flat.counts, flat.width];
  assert.equal(w, v.maxOptions);
  rows.forEach((r, i) => {
    assert.equal(counts[i], r.options.length);
    r.options.forEach(([c, x], j) => {
      assert.equal(codes[i * w + j], c);
      assert.ok(Object.is(values[i * w + j], x));
    });
    for (let j = r.options.length; j < w; j++) assert.ok(codes[i * w + j] === -1 && Number.isNaN(values[i * w + j]));
  });
  const states = s.stateValues(filled, upper, armed);
  rows.forEach((r, i) => assert.ok(Object.is(states[i], r.state_value)));

  // A table round trip: the f32 table loads and agrees with the f64 values to f32 precision.
  const table = ys.Solver.fromTable(v, ys.Solver.buildTable(v, "f32"));
  assert.equal(table.precision, "f32");
  assert.ok(Math.abs(table.stateValue("upper 0 | filled -") - s.stateValue("upper 0 | filled -")) < 1e-4);
}

// Rules engine smoke test.
const v = new ys.Variant("yatzy-scandinavian");
assert.equal(v.score("full_house", [3, 3, 3, 5, 5]), 19);
const sit = v.startTurn("upper 0 | filled -", [6, 5, 3, 3, 1]);
assert.equal(sit, "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -");
assert.ok(v.legalActions(sit).includes("keep 3 3"));
assert.equal(v.applyKeep(sit, [3, 3], [3, 4, 4]), "dice 3 3 3 4 4 | rolls 1 | upper 0 | filled -");
assert.deepEqual(v.applyScore("upper 60 | filled ones,twos,fours,fives", [3, 3, 1, 2, 4], "threes"), [
  "upper 66 | filled ones,twos,threes,fours,fives",
  56,
]);
assert.throws(() => new ys.Variant("yahtzee"));
assert.equal(v.actionCode("keep 6 6 6 6"), 15 + 209);
const g = new ys.Game(new ys.Variant("american"));
assert.equal(g.score([5, 5, 5, 5, 5], "five_of_a_kind"), 50);
assert.equal(g.score([5, 5, 5, 5, 5], "chance"), 125);
assert.equal(g.bonus, 100);
assert.equal(g.points("ones"), undefined);
console.log("parity: 300 golden situations match bit for bit (single and batch); rules engine ok");
