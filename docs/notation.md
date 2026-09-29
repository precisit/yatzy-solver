# Notation

A canonical text form for states, situations and actions (SPEC F6). It is used in logs, tests, exports and user
interfaces. The formatted bytes are stable across versions and are pinned by the golden tests in
`crates/yatzy-solver/tests/notation.rs`; changing them is a breaking change.

The notation is always read against a variant, which gives the category ids and the number of dice. Golden
tests pin the bytes for every built-in variant.

Example, American rules:

```text
dice 2 2 2 2 2 | rolls 1 | upper 12 | filled twos,full_house,five_of_a_kind | five_of_a_kind 50
```

## State (between turns)

```text
upper 21 | filled ones,twos,chance
```

- `upper <n>`: the upper-section total so far, not capped (0 to 105).
- `filled <ids>`: the filled categories, comma-separated without spaces, in score-card order; `-` when none.
- In a variant with a five-of-a-kind bonus (American rules), when the five-of-a-kind box is filled, a third field
  `five_of_a_kind 50` (the box holds its full points, so further five of a kinds earn the bonus) or
  `five_of_a_kind 0` (it was scratched). The field is
  absent while the box is open.

The score so far is not part of the state: it does not affect any decision. Exports record it separately.

## Situation (within a turn)

```text
dice 1 3 3 5 6 | rolls 2 | upper 21 | filled ones,twos,chance
```

- `dice <faces>`: the dice showing, in ascending order, separated by single spaces.
- `rolls <n>`: the rerolls left: 2 after the first roll of a turn, 1 after the second, 0 after the third.
- then the state fields.

## Actions

```text
keep 3 3
keep -
score full_house
```

- `keep <faces>`: keep these dice (ascending) and reroll the others; `keep -` rerolls all of them. A keep is a
  proper subset of the dice showing: keeping every die is not an action.
- `score <id>`: score the dice in that category.

## Canonical form and parsing

Formatting always produces exactly the forms above: fields separated by ` | ` (space, bar, space), tokens by
single spaces, no leading or trailing whitespace. The parser also accepts other amounts of whitespace between
tokens and around `|`, but rejects everything else: fields out of order, unsorted dice, categories out of order
or repeated, unknown categories, dice counts that do not match the variant, and inconsistent states (for
example an upper total with no upper box filled).

## Action codes

Batch arrays and learners with a fixed output layout identify actions by integer codes. The code space is fixed
per variant and is part of this contract:

- categories are `0` to `C - 1`, in score-card order (`C` = 15 for Scandinavian Yatzy, 13 for American rules);
- keeps are `C + k`, where `k` indexes the multisets of 0 to 4 dice (keeping all five is never an action) by
  size and then lexicographically: `keep -` is `C`, `keep 1` to `keep 6` are `C + 1` to `C + 6`, `keep 1 1` is
  `C + 7`, and `keep 6 6 6 6` is `C + 209`.

There are `C + 210` codes (225 for Scandinavian Yatzy, 223 for American rules). One situation has at most
`C + 31` legal actions (every category, and the 31 proper sub-multisets of five different dice).

**Flat layout** (`Solver::option_values_flat`, and the numpy batch functions): per situation, the codes and
values of its options in legal-action order, padded to width `C + 31` with code `-1` and value NaN, plus the
number of options. **Dense layout** (`FlatOptions::to_dense`): per situation, a value for each of the `C + 210`
codes, NaN where the action is not legal.
