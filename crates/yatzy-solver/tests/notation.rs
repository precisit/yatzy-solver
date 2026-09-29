//! Golden tests for the stable notation (F6). These bytes are part of the public contract: a change here is a
//! breaking change to logs, exports and datasets.

use yatzy_solver::{Action, Dice, Situation, State, Variant};

/// (variant id, canonical situation notation).
const SITUATIONS: &[(&str, &str)] = &[
    ("yatzy-scandinavian", "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -"),
    ("yatzy-scandinavian", "dice 1 3 3 5 6 | rolls 2 | upper 21 | filled ones,twos,chance"),
    ("yatzy-scandinavian", "dice 6 6 6 6 6 | rolls 0 | upper 63 | filled ones,twos,threes,fours,fives,sixes"),
    (
        "yatzy-scandinavian",
        "dice 1 1 1 1 2 | rolls 1 | upper 0 | filled one_pair,two_pairs,three_of_a_kind,four_of_a_kind,small_straight,large_straight,full_house,chance,yatzy",
    ),
    ("yatzy-scandinavian+forced", "dice 2 2 3 4 5 | rolls 1 | upper 3 | filled ones"),
    ("yahtzee", "dice 2 3 4 5 6 | rolls 0 | upper 0 | filled -"),
    ("yahtzee", "dice 5 5 5 5 5 | rolls 2 | upper 20 | filled fives,yahtzee | yahtzee_box 50"),
    ("yahtzee", "dice 1 1 5 5 5 | rolls 1 | upper 0 | filled full_house,yahtzee | yahtzee_box 0"),
    ("yahtzee+no-bonus", "dice 1 1 5 5 5 | rolls 1 | upper 0 | filled full_house,yahtzee"),
];

#[test]
fn situations_round_trip_byte_for_byte() {
    for &(vid, text) in SITUATIONS {
        let v = Variant::by_id(vid).unwrap();
        let sit = v.parse_situation(text).unwrap_or_else(|e| panic!("{vid}: {text:?}: {e}"));
        assert_eq!(v.format_situation(&sit), text);
    }
}

#[test]
fn golden_formatting() {
    let v = Variant::scandinavian();
    let state = State { filled: 0b10_0000_0000_0011, upper: 21, yahtzee_armed: false };
    let sit = Situation { state, dice: Dice::from_faces(&[6, 5, 3, 3, 1]).unwrap(), rolls_left: 2 };
    assert_eq!(v.format_situation(&sit), "dice 1 3 3 5 6 | rolls 2 | upper 21 | filled ones,twos,chance");
    assert_eq!(v.format_state(&state), "upper 21 | filled ones,twos,chance");
    assert_eq!(v.format_action(&Action::Keep(Dice::from_faces(&[3, 3]).unwrap())), "keep 3 3");
    assert_eq!(v.format_action(&Action::Keep(Dice::EMPTY)), "keep -");
    assert_eq!(v.format_action(&Action::Score(12)), "score full_house");

    let a = Variant::american();
    let y = a.category_index("yahtzee").unwrap();
    let s = State { filled: 1 << y, upper: 0, yahtzee_armed: true };
    assert_eq!(a.format_state(&s), "upper 0 | filled yahtzee | yahtzee_box 50");
    assert_eq!(a.format_action(&Action::Score(y)), "score yahtzee");
}

#[test]
fn actions_round_trip() {
    let v = Variant::scandinavian();
    for text in ["keep 3 3", "keep -", "keep 1 2 3 4", "keep 6", "score yatzy", "score ones"] {
        assert_eq!(v.format_action(&v.parse_action(text).unwrap()), text);
    }
}

#[test]
fn whitespace_is_tolerated_but_not_emitted() {
    let v = Variant::scandinavian();
    let sit = v.parse_situation("  dice 1 3  3 5 6|rolls 2 |upper   21|  filled ones,twos,chance ").unwrap();
    assert_eq!(v.format_situation(&sit), "dice 1 3 3 5 6 | rolls 2 | upper 21 | filled ones,twos,chance");
    assert_eq!(v.parse_action("  keep   3 3 ").unwrap(), v.parse_action("keep 3 3").unwrap());
}

#[test]
fn rejects_non_canonical_or_invalid_text() {
    let v = Variant::scandinavian();
    for bad in [
        "",
        "dice 3 1 3 5 6 | rolls 2 | upper 0 | filled -", // unsorted dice
        "dice 1 3 3 5 | rolls 2 | upper 0 | filled -",   // four dice
        "dice 1 3 3 5 7 | rolls 2 | upper 0 | filled -", // bad face
        "dice 1 3 3 5 6 | rolls 3 | upper 0 | filled -", // too many rolls left
        "dice 1 3 3 5 6 | upper 0 | rolls 2 | filled -", // field order
        "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled chance,ones", // category order
        "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled ones,ones", // duplicate
        "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled yahtzee", // wrong variant's category
        "dice 1 3 3 5 6 | rolls 2 | upper 5 | filled -", // upper total with no upper box
        "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled ones, twos", // space in the list
        "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled - | yahtzee_box 0", // no Yahtzee box here
    ] {
        assert!(v.parse_situation(bad).is_err(), "accepted {bad:?}");
    }
    for bad in ["keep 3 1", "keep", "keep 1 2 3 4 5", "score", "score nope", "hold 3 3", "score ones twos"] {
        assert!(v.parse_action(bad).is_err(), "accepted {bad:?}");
    }
    let a = Variant::american();
    for bad in [
        "upper 0 | filled yahtzee",                  // Yahtzee box status missing
        "upper 0 | filled - | yahtzee_box 50",       // status for an open box
        "upper 0 | filled yahtzee | yahtzee_box 25", // not 0 or 50
    ] {
        assert!(a.parse_state(bad).is_err(), "accepted {bad:?}");
    }
}
