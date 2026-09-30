// Turns wasm-pack's output (pkg/) into the npm package `yatzy-solver`: the package metadata, a Node entry that
// loads the module synchronously, the license and a README. Run after `wasm-pack build --target web`.
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pkg = join(root, "pkg");
const generated = JSON.parse(readFileSync(join(pkg, "package.json"), "utf8"));

const manifest = {
  name: "yatzy-solver",
  version: generated.version,
  description:
    "Exact solver, rules engine and advisor for Scandinavian Yatzy and American rules (Yahtzee-compatible), in WebAssembly",
  license: "MIT",
  repository: { type: "git", url: "https://github.com/precisit/yatzy-solver" },
  type: "module",
  files: ["yatzy_solver.js", "yatzy_solver.d.ts", "yatzy_solver_bg.wasm", "yatzy_solver_bg.wasm.d.ts", "node.js", "node.d.ts", "LICENSE", "README.md"],
  main: "./node.js",
  module: "./yatzy_solver.js",
  types: "./yatzy_solver.d.ts",
  exports: {
    ".": {
      node: { types: "./node.d.ts", default: "./node.js" },
      default: { types: "./yatzy_solver.d.ts", default: "./yatzy_solver.js" },
    },
    "./yatzy_solver_bg.wasm": "./yatzy_solver_bg.wasm",
  },
  sideEffects: ["./node.js"],
  keywords: ["yatzy", "yahtzee-compatible", "dice", "solver", "wasm"],
};
writeFileSync(join(pkg, "package.json"), JSON.stringify(manifest, null, 2) + "\n");

// Node: initialize from the file next to this module, synchronously, then re-export everything.
writeFileSync(
  join(pkg, "node.js"),
  `import { readFileSync } from "node:fs";
import { initSync } from "./yatzy_solver.js";
initSync({ module: readFileSync(new URL("./yatzy_solver_bg.wasm", import.meta.url)) });
export * from "./yatzy_solver.js";
`,
);
writeFileSync(join(pkg, "node.d.ts"), `export * from "./yatzy_solver.js";\n`);
copyFileSync(join(root, "..", "..", "LICENSE"), join(pkg, "LICENSE"));
copyFileSync(join(root, "README.md"), join(pkg, "README.md"));
