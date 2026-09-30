Exact optimal play for Scandinavian Yatzy and American rules (Yahtzee-compatible): the expected final score of
every decision in every situation, from Rust, Python and JavaScript.

**Validation.** American rules 254.5896 (Verhoeff, Glenn) and 245.87 without bonus and joker, as published;
Scandinavian Yatzy 248.4399894 under its stated rules (the often-cited 248.63 comes from a bug in its authors'
code, reproduced and documented). An exact brute-force solver agrees in rational arithmetic on nine reduced
games, and Rust, Python and WebAssembly agree bit for bit on 300 golden situations. Reproduce:
`yatzy-solver verify --golden golden/parity.jsonl`.

**Install.**

```sh
cargo add yatzy-solver          # Rust library
cargo install yatzy-solver-cli  # the yatzy-solver command
pip install yatzy-solver        # Python
npm install yatzy-solver        # JavaScript (WebAssembly)
```

**Assets.** The prebuilt tables (f32 and f64 for both variants) with `tables.sha256` (file hashes) and
`values.sha256` (value-array hashes, the same on every platform); CLI binaries for Linux x86-64, macOS arm64 (not
notarized: use `cargo install yatzy-solver-cli`, or `xattr -d com.apple.quarantine`) and Windows x86-64.

See `CHANGELOG.md` for everything in this release, and the README for the rules, the method and examples.

"Yatzy" is used as the name of the game; this project is not affiliated with any trademark holder. "Yahtzee" is a
trademark of Hasbro; this project is not affiliated with Hasbro.
