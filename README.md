# yatzy-solver

An exact solver for Scandinavian Yatzy and Yahtzee-compatible American rules: the optimal expected final score of
every decision in every situation, with a rules engine, a simulator and a data exporter. Rust core with Python and
WebAssembly bindings.

**Status:** in development (private). The specification is [`SPEC.md`](SPEC.md).

- [Rules](docs/rules.md): the variants, house-rule switches and joker rules.
- [Notation](docs/notation.md): the stable text form of situations and actions.

## Prior work

- Tom Verhoeff (1999) solved solitaire Yahtzee: expected score 254.5896, standard deviation 59.6117, median 248
  ([trivia page](https://www-set.win.tue.nl/~wstomv/misc/yahtzee/trivia.html)).
- James Glenn solved it independently and described symmetry optimizations
  ([Computer Strategies for Solitaire Yahtzee, CIG 2007](http://www.cs.loyola.edu/~jglenn/Papers/yahtzee_cig2007_glenn.pdf)).
- Scandinavian Yatzy was solved by Larsson and Sjöberg (KTH, 2012), expected score 248.63
  ([report](https://www.csc.kth.se/utbildning/kth/kurser/DD143X/dkand12/Group89Michael/report/Larsson+Sjoberg.pdf)),
  and studied by Sederblad and Törnebohm (KTH, 2013,
  [report](https://www.diva-portal.org/smash/get/diva2:676659/FULLTEXT01.pdf)).
- Jakub Pawlewicz studied nearly optimal multiplayer play ("Nearly Optimal Computer Play in Multi-player Yahtzee",
  Computers and Games 2010).

"Yahtzee" is a trademark of Hasbro. This project implements rules compatible with it and is not affiliated with
Hasbro.

## License

MIT, see [`LICENSE`](LICENSE).
