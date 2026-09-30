// Checks that the advisor's table manifest (demo/dist/tables/manifest.json) lists the published hashes
// (release/tables.sha256), so the demo and the release serve the same tables. Run after `node demo/build.mjs`.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..");
const published = new Map(
  readFileSync(join(repo, "release/tables.sha256"), "utf8")
    .trim()
    .split("\n")
    .map((l) => l.split(/\s+/))
    .map(([hash, file]) => [file, hash]),
);
const manifest = JSON.parse(readFileSync(join(repo, "demo/dist/tables/manifest.json"), "utf8"));
let ok = true;
for (const [id, entry] of Object.entries(manifest)) {
  const precision = entry.file.split(".")[1];
  const want = published.get(`${id}.${precision}.yzt`);
  const match = want === entry.sha256;
  ok &&= match;
  console.log(`${id} ${precision}: ${match ? "matches" : `MISMATCH (manifest ${entry.sha256}, published ${want})`}`);
}
process.exit(ok ? 0 : 1);
