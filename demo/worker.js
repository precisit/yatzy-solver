// Loads the solver off the main thread: the WASM module, then the table for the variant (from the Cache API,
// else the network, checked against the manifest's SHA-256), falling back to solving in the worker.
import init, { Solver, Variant } from "./pkg/yatzy_solver.js";

const CACHE = "yatzy-advisor-tables-v1";
let ready = init();
const solvers = new Map();

async function sha256(bytes) {
  const d = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return [...d].map((b) => b.toString(16).padStart(2, "0")).join("");
}

async function loadTable(id) {
  const manifest = await (await fetch("tables/manifest.json", { cache: "no-cache" })).json();
  const entry = manifest[id];
  if (!entry) return null;
  const url = new URL(`tables/${entry.file}`, self.location).href;
  const cache = await caches.open(CACHE);
  for (const source of ["cache", "network"]) {
    let res = source === "cache" ? await cache.match(url) : await fetch(url).catch(() => null);
    if (!res || !res.ok) continue;
    const bytes = new Uint8Array(await res.clone().arrayBuffer());
    if ((await sha256(bytes)) !== entry.sha256) {
      if (source === "cache") await cache.delete(url);
      continue;
    }
    if (source === "network") {
      await cache.put(url, res);
      // Drop cached tables that the manifest no longer lists.
      const current = new Set(Object.values(manifest).map((m) => new URL(`tables/${m.file}`, self.location).href));
      for (const req of await cache.keys()) if (!current.has(req.url)) await cache.delete(req);
    }
    return { bytes, source };
  }
  return null;
}

async function solver(id, post) {
  if (solvers.has(id)) return solvers.get(id);
  await ready;
  const v = new Variant(id);
  let table = null;
  try {
    table = await loadTable(id);
  } catch {
    table = null;
  }
  let s, source;
  if (table) {
    s = Solver.fromTable(v, table.bytes);
    source = table.source;
  } else {
    post({ type: "solving" });
    s = Solver.build(v);
    source = "solved";
  }
  const entry = { v, s, source };
  solvers.set(id, entry);
  return entry;
}

self.onmessage = async (e) => {
  const m = e.data;
  const post = (x) => self.postMessage({ ...x, id: m.id });
  try {
    const { v, s, source } = await solver(m.variant, post);
    if (m.type === "init") {
      post({ type: "ready", source, categories: v.categories, dice: v.dice });
    } else if (m.type === "query") {
      const options = s.optionValues(m.situation).map((o) => ({ action: o.action, value: o.value }));
      const best = Math.max(...options.map((o) => o.value));
      const bests = new Set(s.bestOptions(m.situation).map((o) => o.action));
      post({ type: "options", options: options.map((o) => ({ ...o, loss: bests.has(o.action) ? 0 : best - o.value, best: bests.has(o.action) })) });
    } else if (m.type === "points") {
      // The points each legal category would earn now (bonuses included), from the rules engine.
      const out = {};
      for (const c of v.categories) {
        try {
          out[c] = v.scoreIn(m.situation, c);
        } catch {
          /* not legal now */
        }
      }
      post({ type: "points", points: out });
    }
  } catch (err) {
    post({ type: "error", message: String(err && err.message ? err.message : err) });
  }
};
