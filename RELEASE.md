# Releasing

The release workflow (`.github/workflows/release.yml`) builds every artifact, installs the artifacts on Linux,
macOS and Windows (and the extra wheels under emulation), and checks the golden set through the CLI, the wheel,
the npm package and a consumer of the packaged crate. On pull requests that touch release inputs it runs as a
dry run, including a test of the skip-if-published logic. It **publishes only on a `v*` tag**, after an approval
in the `release` environment.

Steps are marked **owner** (needs the owner's accounts, keys or decisions) or **agent** (the coding agent does
them, in a pull request). Tagging, publishing and making the repository public are the owner's.

## Repository rules that shape the order

- `main` accepts changes only through pull requests with the required checks green (`test` and `python` on
  three OSes, and `wasm`). If a CI job is renamed, update the required check names in the ruleset.
- Tags `v*` cannot be deleted or moved once pushed.
- The `release` environment deploys only from `v*` tags. **Required reviewers cannot be set while the repository
  is private**, so the repository is made public first, then the reviewer is set, then the tag is pushed. The
  workflow's `guard` job fails a tag run whose environment has no required reviewer, so a tag pushed too early
  stops before publishing.

## How publishing authenticates

| registry | first release (1.0.0) | later releases |
| --- | --- | --- |
| crates.io | a short-lived token scoped to "publish new" for `yatzy-solver` and `yatzy-solver-cli`, as the environment secret `CARGO_REGISTRY_TOKEN` | trusted publishing (`rust-lang/crates-io-auth-action`) |
| PyPI | trusted publishing through a pending publisher (no token) | trusted publishing |
| npm | a short-lived granular token as the environment secret `NPM_TOKEN` | trusted publishing (OIDC), with provenance |

Neither crates.io nor npm can create a new package through trusted publishing, hence the tokens for the first
release. The workflow picks the method by whether the secret exists: **deleting a secret switches that
registry to trusted publishing**, with no workflow change.

**Re-running is safe.** Each publishing step first checks whether the version is already published
(`release/published.sh`) and skips it if so; PyPI skips existing files; the GitHub release is skipped if it
exists. After a failure half-way (for example crates.io done, PyPI failed), fix the cause and re-run the
`publish` job.

## Release 1.0.0, in order

1. **Agent:** version, CHANGELOG, table hashes (`release/tables.sha256`, README), release notes
   (`release/notes.md`), docs; the quiet-machine timing runs and the automated WebKit check; merge with CI green.
   *Done in PR #6.*
2. **Owner:** accounts with 2FA on crates.io, PyPI and npm (personal accounts for now).
3. **Owner:** on PyPI, add a **pending publisher** for the project `yatzy-solver`: owner `precisit`, repository
   `yatzy-solver`, workflow `release.yml`, environment `release`.
4. **Owner:** the manual **Safari** pass (ideally iPhone Safari too) on the advisor: first load (table
   downloaded), reload (table from cache), offline (reload with the network off), both languages, a keep and a
   score, and switching to American rules while offline (the in-browser solve fallback).
5. **Owner:** make the repository **public**.
6. **Owner:** in Settings, Environments, `release`: add yourself as the **required reviewer**, with "prevent
   self-review" **off** (you both tag and approve). Keep the deployment rule "tags matching `v*`".
7. **Owner:** just before tagging, create the tokens with a short expiry and add them as secrets of the
   `release` environment: `CARGO_REGISTRY_TOKEN` (crates.io, "publish new", both crate names) and `NPM_TOKEN`
   (npm granular token, publish, the package `yatzy-solver`).
8. **Owner:** the signed tag, from an up-to-date `main`:
   `git tag -s v1.0.0 -m "yatzy-solver 1.0.0" && git push origin v1.0.0`.
9. **Owner:** in the Release workflow run, check that `guard` passed and the checks are green, then **approve**
   the `release` environment. The `publish` job publishes both crates, the wheels and sdist, the npm package
   with provenance, and the GitHub release with the tables, the hash files and the CLI binaries.
10. **Agent:** after publishing, clean installs from the registries (`cargo add`, `cargo install`, `pip
    install`, `npm install` on three OSes), the golden set through each, and that docs.rs built.
11. **Agent, with the owner's go-ahead:** enable GitHub Pages for the advisor and the `docs/` folder.
12. **Owner:** configure **trusted publishing** on crates.io (both crates) and npm (`yatzy-solver`) for this
    repository, workflow `release.yml` and environment `release`; then **delete both tokens** and the
    environment secrets `CARGO_REGISTRY_TOKEN` and `NPM_TOKEN`.

## Later releases

1. **Agent:** update `CHANGELOG.md` and the workspace version in `Cargo.toml` (it drives the crates, the wheel
   and the npm package). Rebuild the tables and update the published hashes in the same pull request:
   `for v in yatzy-scandinavian american; do for p in f32 f64; do yatzy-solver build --variant $v --precision $p --out tables/$v.$p.yzt; done; done`,
   then `(cd tables && shasum -a 256 *.yzt) > release/tables.sha256`, and the table in `README.md`. The values
   hashes (`release/values.sha256`) change only if the values do; the tests pin them. Update
   `release/notes.md`. Merge with CI green, including the release dry run.
2. **Owner:** the Safari pass if the advisor changed; the signed tag; the approval. No tokens are needed once
   trusted publishing is configured.
3. **Agent:** the clean-install checks.
