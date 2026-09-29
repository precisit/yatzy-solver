# yatzy-solver

An exact solver for Scandinavian Yatzy and Yahtzee-compatible American rules: the optimal expected final score of
every decision in every situation, with a rules engine, a simulator and a data exporter. Rust core with Python and
WebAssembly bindings.

**Status:** in development (private). The specification is [`SPEC.md`](SPEC.md).

## Validation

| variant | this solver | reference |
| --- | --- | --- |
| American rules (Yahtzee-compatible) | 254.5896095 | 254.5896 (Verhoeff, Glenn) |
| American rules without the five-of-a-kind bonus and joker | 245.8707745 | 245.87 (Verhoeff, Glenn) |
| Scandinavian Yatzy | 248.4399894 | 248.4394 (Castux/yahtzee), about 248.44 (Laurii1i/Yatzy) |

**About the published 248.63.** The Scandinavian value usually cited, 248.63 (Larsson and Sjöberg, KTH 2012),
comes from a bug in the authors' code, not from their rules. Their stated rules match ours. Their published Java
code ([ansjob/optimalt-yatzy](https://github.com/ansjob/optimalt-yatzy), `ScoreCard.java`, function
`scorePair`) scores Two pairs as twice the highest pair plus twice the second pair, adding 0 when there is no
second pair, so a hand with a single pair scores in Two pairs. With that scoring this solver gives 248.6328539,
which rounds to 248.63; `verify` checks this too. Details in [docs/rules.md](docs/rules.md).

A brute-force reference solver checks the fast solver exactly, in rational arithmetic, on reduced games. To
reproduce everything:

```sh
cargo run --release -p yatzy-solver-cli -- verify
```

## Documentation

- [Rules](docs/rules.md): the variants, house-rule switches and joker rules.
- [Notation](docs/notation.md): the stable text form of situations and actions.
- [Solver](docs/solver.md): the method, verification, table format and performance.

## Command line

```sh
yatzy-solver build --variant yatzy-scandinavian          # writes tables/yatzy-scandinavian.f32.yzt
yatzy-solver query "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -"
yatzy-solver verify
```

## Prior work

- Tom Verhoeff (1999) solved solitaire Yahtzee: expected score 254.5896, standard deviation 59.6117, median 248
  ([trivia page](https://www-set.win.tue.nl/~wstomv/misc/yahtzee/trivia.html)).
- James Glenn solved it independently and described symmetry optimizations
  ([Computer Strategies for Solitaire Yahtzee, CIG 2007](http://www.cs.loyola.edu/~jglenn/Papers/yahtzee_cig2007_glenn.pdf)).
- Scandinavian Yatzy was solved by Larsson and Sjöberg (KTH, 2012), published expected score 248.63 (see above)
  ([report](https://www.csc.kth.se/utbildning/kth/kurser/DD143X/dkand12/Group89Michael/report/Larsson+Sjoberg.pdf)),
  and studied by Sederblad and Törnebohm (KTH, 2013,
  [report](https://www.diva-portal.org/smash/get/diva2:676659/FULLTEXT01.pdf)).
- Independent open-source solvers of Scandinavian Yatzy, [Castux/yahtzee](https://github.com/Castux/yahtzee)
  and [Laurii1i/Yatzy](https://github.com/Laurii1i/Yatzy), which confirm the value under the stated rules.
- Jakub Pawlewicz studied nearly optimal multiplayer play ("Nearly Optimal Computer Play in Multi-player Yahtzee",
  Computers and Games 2010).

"Yahtzee" is a trademark of Hasbro. This project implements rules compatible with it and is not affiliated with
Hasbro.

## License

MIT, see [`LICENSE`](LICENSE).
