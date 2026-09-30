# Releasing

The release workflow (`.github/workflows/release.yml`) builds and checks everything on every pull request that
touches it, and on a manual run; it **publishes only on a version tag**, and only after an approval in the
`release` environment. Tagging, publishing and making the repository public are the owner's decisions; this is
the owner's checklist.

## Once, before the first release

- [ ] Decide the names: `yatzy-solver` on crates.io, PyPI and npm, and `yatzy-solver-cli` on crates.io (all
      free as of 2026-09-29, see SPEC 9.3). Check the trademark status of "Yatzy" in the Nordic countries first;
      `optimal-yatzy` is the fallback.
- [ ] Decide where the web advisor is hosted (for example GitHub Pages, which needs the repository public).
- [ ] Create the GitHub environment `release` with required reviewers (Settings, Environments).
- [ ] Add the publishing credentials to that environment:
  - crates.io: an API token as `CARGO_REGISTRY_TOKEN`;
  - PyPI: configure trusted publishing for this repository and the `release` environment (no token needed);
  - npm: an automation token as `NPM_TOKEN`.
- [ ] Pass the Safari check below once.

## For each release

1. [ ] Update `CHANGELOG.md` (move "unreleased" to the version and date) and the version in `Cargo.toml`
       (the workspace version is used by the crates, the Python wheel and the npm package).
2. [ ] Rebuild the tables and update the published hashes, in the same pull request:
       `for v in yatzy-scandinavian american; do for p in f32 f64; do yatzy-solver build --variant $v --precision $p --out tables/$v.$p.yzt; done; done`,
       then `(cd tables && shasum -a 256 *.yzt) > release/tables.sha256`, and the table in `README.md`.
       The values hashes (`release/values.sha256`) change only if the values do; the tests pin them.
3. [ ] Merge with CI green, including the release workflow's dry run (artifacts built, installed on Linux, macOS
       and Windows, golden set checked through Rust, Python and WebAssembly from the artifacts).
4. [ ] Run the manual checks on the release candidate:
   - [ ] **Safari** (and ideally Firefox): open the advisor from a normal window. Check the first load (table
         downloaded), a reload (table from cache), offline (reload with the network off), both languages, a
         keep and a score, and switching to American rules while offline (the in-browser solve fallback).
   - [ ] On a quiet machine: `yatzy-solver verify --golden golden/parity.jsonl` and the timings in
         `docs/solver.md` and `docs/wasm.md`; record the machine and its load.
5. [ ] Tag: `git tag -s v1.0.0 -m "yatzy-solver 1.0.0" && git push origin v1.0.0`.
6. [ ] Approve the `release` environment in the workflow run. It publishes the crates, the wheels and sdist, and
       the npm package, and creates the GitHub release with the tables, `release/tables.sha256` and the CLI
       binaries.
7. [ ] After publishing: check `cargo add yatzy-solver`, `pip install yatzy-solver` and `npm install
       yatzy-solver` from a clean environment, and that docs.rs built.
8. [ ] Make the repository public, if decided, and publish the advisor.
