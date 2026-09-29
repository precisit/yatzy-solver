# Notation

A canonical text form for states, situations and actions (SPEC F6). It is used in logs, tests, exports and user
interfaces. The formatted bytes are stable across versions and are pinned by the golden tests in
`crates/yatzy-solver/tests/notation.rs`; changing them is a breaking change.

The notation is always read against a variant, which gives the category ids and the number of dice.

## State (between turns)

```text
upper 21 | filled ones,twos,chance
```

- `upper <n>`: the upper-section total so far, not capped (0 to 105).
- `filled <ids>`: the filled categories, comma-separated without spaces, in score-card order; `-` when none.
- In a variant with a Yahtzee bonus, when the Yahtzee box is filled, a third field `yahtzee_box 50` (the box
  holds its full points, so further Yahtzees earn the bonus) or `yahtzee_box 0` (it was scratched). The field is
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
