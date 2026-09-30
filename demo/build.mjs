// Builds the advisor into demo/dist: the static files, the WASM package (from bindings/wasm/pkg, built with
// wasm-pack) and the f32 tables (built with the CLI), with a manifest of their SHA-256 hashes.
//   node demo/build.mjs            (after wasm-pack and the release CLI are built)
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const demo = dirname(fileURLToPath(import.meta.url));
const repo = join(demo, "..");
const dist = join(demo, "dist");
rmSync(dist, { recursive: true, force: true });
mkdirSync(join(dist, "pkg"), { recursive: true });
mkdirSync(join(dist, "tables"), { recursive: true });
for (const f of ["index.html", "style.css", "app.js", "i18n.js", "worker.js", "sw.js"]) copyFileSync(join(demo, f), join(dist, f));
for (const f of ["yatzy_solver.js", "yatzy_solver_bg.wasm"]) copyFileSync(join(repo, "bindings/wasm/pkg", f), join(dist, "pkg", f));
const cli = join(repo, "target/release/yatzy-solver");
const manifest = {};
for (const id of ["yatzy-scandinavian", "american"]) {
  const file = `${id}.f32.yzt`;
  execFileSync(cli, ["build", "--variant", id, "--precision", "f32", "--out", join(dist, "tables", file)], { stdio: "inherit" });
  const bytes = readFileSync(join(dist, "tables", file));
  manifest[id] = { file, bytes: bytes.length, sha256: createHash("sha256").update(bytes).digest("hex") };
}
writeFileSync(join(dist, "tables/manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log("built demo/dist", manifest);
