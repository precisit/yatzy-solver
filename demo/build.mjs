// Builds the advisor into demo/dist: the static files, the WASM package (from bindings/wasm/pkg, built with
// wasm-pack) and the f32 tables (built with the CLI), with a manifest of their SHA-256 hashes.
//   node demo/build.mjs            (after wasm-pack and the release CLI are built)
//   PRECISION=f64 node demo/build.mjs   (f64 tables, for example to test table updates)
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
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
const precision = process.env.PRECISION || "f32";
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const manifest = {};
for (const id of ["yatzy-scandinavian", "american"]) {
  // Named by the hash prefix, so a URL never changes its content.
  const tmp = join(dist, "tables", `${id}.tmp`);
  execFileSync(cli, ["build", "--variant", id, "--precision", precision, "--out", tmp], { stdio: "inherit" });
  const bytes = readFileSync(tmp);
  const hash = sha(bytes);
  const file = `${id}.${precision}.${hash.slice(0, 8)}.yzt`;
  renameSync(tmp, join(dist, "tables", file));
  manifest[id] = { file, bytes: bytes.length, sha256: hash };
}
const manifestText = JSON.stringify(manifest, null, 2) + "\n";
writeFileSync(join(dist, "tables/manifest.json"), manifestText);

// The build id: a hash of the shell, the WASM package and the manifest, written into the service worker.
const shell = ["index.html", "style.css", "app.js", "i18n.js", "worker.js", "pkg/yatzy_solver.js", "pkg/yatzy_solver_bg.wasm"];
const h = createHash("sha256");
for (const f of shell) h.update(f).update(readFileSync(join(dist, f)));
h.update(manifestText);
const build = h.digest("hex").slice(0, 16);
const sw = readFileSync(join(dist, "sw.js"), "utf8");
if (!sw.includes("__BUILD_ID__")) throw new Error("sw.js has no build id placeholder");
writeFileSync(join(dist, "sw.js"), sw.replace("__BUILD_ID__", build));
console.log("built demo/dist, build", build, manifest);
