# Rules

This document describes the rules the engine implements. Rules are data: each variant is a `VariantDef` (dice,
categories, scoring, bonuses, special rules), and the built-in variants below are definitions, not special
cases in the code. Every scoring rule has a table-driven test in `crates/yatzy-solver/tests/scoring.rs`.

## Common to all variants

- Five six-sided dice. A turn has up to three rolls: after the first and the second roll the player keeps any
  subset of the dice and rerolls the rest. The player may score after any roll.
- One category is filled per turn, each exactly once, so a game has as many turns as categories. A category may
  always be filled with 0 points (a scratch), unless a special rule below restricts where a hand may go.
- The upper section is Ones to Sixes. Each scores the sum of the dice showing that face.

### Actions

In a situation (the state, the dice showing, the rerolls left) the legal actions are:

- `score <category>` for every open category that the rules allow (all of them, except under forced order and
  the forced joker);
- when rerolls are left, `keep <dice>` for every distinct proper sub-multiset of the dice, including keeping
  nothing (`keep -`). Keeping all the dice is not an action: not rerolling is the same as scoring now.

Actions are listed in a fixed order: categories in score-card order, then keeps by size and then
lexicographically.

## Scandinavian Yatzy (`yatzy-scandinavian`)

15 categories, 15 turns. The upper bonus is 50 points when Ones to Sixes total at least 63. The maximum score is
374.

| id | category | score |
| --- | --- | --- |
| `ones` ... `sixes` | Ones ... Sixes | the sum of the dice showing that face |
| `one_pair` | One pair | the highest pair: 2 x v |
| `two_pairs` | Two pairs | two pairs of different faces: 2a + 2b |
| `three_of_a_kind` | Three of a kind | 3 x v |
| `four_of_a_kind` | Four of a kind | 4 x v |
| `small_straight` | Small straight | 1-2-3-4-5: 15 |
| `large_straight` | Large straight | 2-3-4-5-6: 20 |
| `full_house` | Full house | three of one face and two of another: the sum of the dice |
| `chance` | Chance | the sum of the dice |
| `yatzy` | Yatzy | five of a kind: 50 |

Five of a kind counts as one pair, three of a kind and four of a kind (for 2v, 3v and 4v), but not as a full
house or two pairs under the default rules.

### House-rule switches

Each switch adds a suffix to the variant id, in this order:

| suffix | switch | default |
| --- | --- | --- |
| `+fh5` | five of a kind counts as a full house (scoring the sum of the dice) | off |
| `+tp4` | four of a kind counts as two pairs (scoring 4 x v) | off |
| `+forced` | forced order ("tvångsyatzy"): categories are filled top to bottom | off |

For example `yatzy-scandinavian+fh5+forced`.

### The validation value, and the published 248.63 (SPEC 10.1)

The expected score under optimal play with the default rules is **248.4399894**; this is the validation value.
The published 248.63 (Larsson and Sjöberg, KTH, 2012) differs because of the authors' code, not their rules:

- Their stated rules (appendix A, based on Alga's rule set) match the defaults above: the two pairs must be
  different ("1, 2, 2, 2, 2" is not two pairs), five of a kind is not a full house, three and four of a kind
  score 3v and 4v, the straights are 1-2-3-4-5 (15) and 2-3-4-5-6 (20), the full house scores the sum, any
  category may be scratched, there is no forced order, and the upper total is capped at 63.
- Their published Java code (`ansjob/optimalt-yatzy`, `ScoreCard.scorePair`) scores Two pairs as twice the
  highest pair plus twice the second-highest pair, **or plus 0 when there is no second pair**. A hand with a
  single pair therefore scores in Two pairs: 1 2 2 4 5 scores 4 and 6 6 6 6 6 scores 12.
- With that scoring (`verify::larsson_sjoberg_2012()`, id `yatzy-scandinavian-ls2012-code`; a check, not a
  playable variant) this solver gives **248.6328539**, which rounds to 248.63.
- Two independent open-source solvers agree with 248.44 for the stated rules: `Castux/yahtzee` reports 248.4394
  and `Laurii1i/Yatzy` about 248.44.
- Sederblad and Törnebohm (KTH, 2013) did not solve the full game; later theses cite 248.63 without recomputing
  it.

For reference, the other switches give: `+fh5` 248.4680909, `+tp4` 248.6440589, `+fh5+tp4` 248.6748827.

## American rules, Yahtzee-compatible (`american`)

"Yahtzee" is a trademark of Hasbro; this variant implements rules compatible with it. Ids, the notation and
table files never use the name: the variant is `american` and its five-of-a-kind box is `five_of_a_kind`. 13 categories, 13 turns.
The upper bonus is 35 points when Ones to Sixes total at least 63.

| id | category | score |
| --- | --- | --- |
| `ones` ... `sixes` | Ones ... Sixes | the sum of the dice showing that face |
| `three_of_a_kind` | Three of a kind | at least three of one face: the sum of all dice |
| `four_of_a_kind` | Four of a kind | at least four of one face: the sum of all dice |
| `full_house` | Full house | three of one face and two of another: 25 |
| `small_straight` | Small straight | any four in sequence: 30 |
| `large_straight` | Large straight | five in sequence: 40 |
| `five_of_a_kind` | Yahtzee | five of a kind: 50 |
| `chance` | Chance | the sum of the dice |

Five of a kind is not a full house, except as a joker.

### Yahtzee bonus and joker

A hand of five of a kind while the Yahtzee box is already filled is an **extra Yahtzee**.

- **Yahtzee bonus:** an extra Yahtzee earns 100 points when the Yahtzee box holds 50, wherever the hand is
  scored. It does not count toward the upper bonus. If the Yahtzee box was scratched (0), there is no bonus.
- **Joker:** under the joker rule, an extra Yahtzee scores full points in full house (25), small straight (30)
  and large straight (40). Other categories score it normally.

Published sources differ on when the joker applies and where the hand may go, so the joker rule is a switch:

| id | joker rule | expected score |
| --- | --- | --- |
| `american` | **Free joker** (Verhoeff's rules, used for the published 254.5896): the joker applies when the Yahtzee box and the upper box of the hand's face are both filled (with any score, zero included). The player may score the hand in any open box. | 254.5896095 |
| `american+forced-joker` | **Forced joker** (the official placement rule): when the Yahtzee box is filled (with any score), the upper box of the hand's face must be used if open; otherwise any open lower box, with the joker applying; otherwise any open upper box, which scores 0. | 254.5877287 |
| `american+no-joker` | The Yahtzee bonus without the joker. | 253.9702412 |
| `american+no-bonus` | Neither the Yahtzee bonus nor the joker. Verhoeff and Glenn report 245.87 for this game. | 245.8707745 |

The default is the free joker because it is what the literature and the published value use. The printed rules
use the forced joker, available as `american+forced-joker`; the two differ by 0.0019 points of expected score.

Verhoeff's trivia page also gives an exact fraction (Liese and Kelly, 2017) that evaluates to 254.58937. It
matches none of the joker rules above; which rules it was computed under is an open question.

The state of the American variant includes the Yahtzee box status (open, 0 or 50), which the notation records
in the `five_of_a_kind` field.

### Unsupported combinations

The forced joker cannot be combined with forced order: the rules do not say which one wins, so such a variant
definition is rejected.

## Reduced and custom variants

A `VariantDef` may use 1 to 6 dice, any number of rolls, and up to 20 categories built from the category kinds
(upper, n of a kind, two pairs, full house, straights given as face patterns, chance, all the same). The solver's
brute-force cross-check uses reduced games built this way.
