//! Table-driven scoring tests: every category of every built-in variant and every house-rule switch.

use std::collections::BTreeSet;

use yatzy_solver::{Dice, Variant};

/// (variant id, dice, category id, expected score) under the normal rules (no joker).
const CASES: &[(&str, &str, &str, u16)] = &[
    // Scandinavian Yatzy, default house rules.
    ("yatzy-scandinavian", "1 1 2 3 4", "ones", 2),
    ("yatzy-scandinavian", "1 1 2 3 4", "twos", 2),
    ("yatzy-scandinavian", "1 1 2 3 4", "threes", 3),
    ("yatzy-scandinavian", "1 1 2 3 4", "fours", 4),
    ("yatzy-scandinavian", "1 1 2 3 4", "fives", 0),
    ("yatzy-scandinavian", "5 5 5 6 6", "fives", 15),
    ("yatzy-scandinavian", "5 5 5 6 6", "sixes", 12),
    ("yatzy-scandinavian", "6 6 6 6 6", "sixes", 30),
    ("yatzy-scandinavian", "1 1 2 3 4", "one_pair", 2),
    ("yatzy-scandinavian", "5 5 6 6 1", "one_pair", 12),
    ("yatzy-scandinavian", "4 4 4 4 2", "one_pair", 8),
    ("yatzy-scandinavian", "1 2 3 4 6", "one_pair", 0),
    ("yatzy-scandinavian", "6 6 6 6 6", "one_pair", 12),
    ("yatzy-scandinavian", "3 3 5 5 6", "two_pairs", 16),
    ("yatzy-scandinavian", "5 5 6 6 1", "two_pairs", 22),
    ("yatzy-scandinavian", "2 2 2 5 5", "two_pairs", 14),
    ("yatzy-scandinavian", "1 1 2 3 4", "two_pairs", 0),
    ("yatzy-scandinavian", "4 4 4 4 2", "two_pairs", 0),
    ("yatzy-scandinavian", "6 6 6 6 6", "two_pairs", 0),
    ("yatzy-scandinavian", "2 2 2 5 5", "three_of_a_kind", 6),
    ("yatzy-scandinavian", "4 4 4 4 2", "three_of_a_kind", 12),
    ("yatzy-scandinavian", "6 6 6 6 6", "three_of_a_kind", 18),
    ("yatzy-scandinavian", "3 3 5 5 6", "three_of_a_kind", 0),
    ("yatzy-scandinavian", "4 4 4 4 2", "four_of_a_kind", 16),
    ("yatzy-scandinavian", "6 6 6 6 6", "four_of_a_kind", 24),
    ("yatzy-scandinavian", "2 2 2 5 5", "four_of_a_kind", 0),
    ("yatzy-scandinavian", "1 2 3 4 5", "small_straight", 15),
    ("yatzy-scandinavian", "2 3 4 5 6", "small_straight", 0),
    ("yatzy-scandinavian", "1 2 3 4 4", "small_straight", 0),
    ("yatzy-scandinavian", "2 3 4 5 6", "large_straight", 20),
    ("yatzy-scandinavian", "1 2 3 4 5", "large_straight", 0),
    ("yatzy-scandinavian", "1 2 3 4 6", "large_straight", 0),
    ("yatzy-scandinavian", "2 2 3 3 3", "full_house", 13),
    ("yatzy-scandinavian", "2 2 2 3 3", "full_house", 12),
    ("yatzy-scandinavian", "5 5 5 6 6", "full_house", 27),
    ("yatzy-scandinavian", "4 4 4 4 2", "full_house", 0),
    ("yatzy-scandinavian", "3 3 5 5 6", "full_house", 0),
    ("yatzy-scandinavian", "6 6 6 6 6", "full_house", 0),
    ("yatzy-scandinavian", "1 1 2 3 4", "chance", 11),
    ("yatzy-scandinavian", "6 6 6 6 6", "chance", 30),
    ("yatzy-scandinavian", "6 6 6 6 6", "yatzy", 50),
    ("yatzy-scandinavian", "1 1 1 1 1", "yatzy", 50),
    ("yatzy-scandinavian", "1 1 1 1 2", "yatzy", 0),
    // House rule: five of a kind counts as a full house.
    ("yatzy-scandinavian+fh5", "6 6 6 6 6", "full_house", 30),
    ("yatzy-scandinavian+fh5", "1 1 1 1 1", "full_house", 5),
    ("yatzy-scandinavian+fh5", "2 2 3 3 3", "full_house", 13),
    ("yatzy-scandinavian+fh5", "4 4 4 4 2", "full_house", 0),
    ("yatzy-scandinavian+fh5", "6 6 6 6 6", "two_pairs", 0),
    // House rule: four of a kind counts as two pairs.
    ("yatzy-scandinavian+tp4", "4 4 4 4 2", "two_pairs", 16),
    ("yatzy-scandinavian+tp4", "6 6 6 6 6", "two_pairs", 24),
    ("yatzy-scandinavian+tp4", "3 3 5 5 6", "two_pairs", 16),
    ("yatzy-scandinavian+tp4", "2 2 2 1 6", "two_pairs", 0),
    ("yatzy-scandinavian+tp4", "6 6 6 6 6", "full_house", 0),
    // Forced order changes legality, not scores.
    ("yatzy-scandinavian+forced", "2 2 3 3 3", "full_house", 13),
    // American rules.
    ("yahtzee", "1 1 2 3 4", "ones", 2),
    ("yahtzee", "2 2 2 5 5", "twos", 6),
    ("yahtzee", "3 3 3 3 1", "threes", 12),
    ("yahtzee", "4 4 4 1 2", "fours", 12),
    ("yahtzee", "5 5 5 5 5", "fives", 25),
    ("yahtzee", "6 6 1 2 3", "sixes", 12),
    ("yahtzee", "4 4 4 1 2", "three_of_a_kind", 15),
    ("yahtzee", "4 4 4 4 2", "three_of_a_kind", 18),
    ("yahtzee", "5 5 5 5 5", "three_of_a_kind", 25),
    ("yahtzee", "3 3 5 5 6", "three_of_a_kind", 0),
    ("yahtzee", "4 4 4 4 2", "four_of_a_kind", 18),
    ("yahtzee", "5 5 5 5 5", "four_of_a_kind", 25),
    ("yahtzee", "4 4 4 1 2", "four_of_a_kind", 0),
    ("yahtzee", "2 2 3 3 3", "full_house", 25),
    ("yahtzee", "6 6 6 1 1", "full_house", 25),
    ("yahtzee", "5 5 5 5 5", "full_house", 0),
    ("yahtzee", "4 4 4 4 2", "full_house", 0),
    ("yahtzee", "1 2 3 4 6", "small_straight", 30),
    ("yahtzee", "2 3 4 5 5", "small_straight", 30),
    ("yahtzee", "1 3 4 5 6", "small_straight", 30),
    ("yahtzee", "1 2 3 4 5", "small_straight", 30),
    ("yahtzee", "1 2 3 5 6", "small_straight", 0),
    ("yahtzee", "1 2 3 4 5", "large_straight", 40),
    ("yahtzee", "2 3 4 5 6", "large_straight", 40),
    ("yahtzee", "1 2 3 4 6", "large_straight", 0),
    ("yahtzee", "5 5 5 5 5", "yahtzee", 50),
    ("yahtzee", "5 5 5 5 4", "yahtzee", 0),
    ("yahtzee", "1 1 2 3 4", "chance", 11),
    ("yahtzee", "6 6 6 6 6", "chance", 30),
    // Zero cases for the upper boxes.
    ("yatzy-scandinavian", "2 3 4 5 6", "ones", 0),
    ("yatzy-scandinavian", "1 3 4 5 6", "twos", 0),
    ("yatzy-scandinavian", "1 1 2 2 4", "threes", 0),
    ("yatzy-scandinavian", "1 1 2 3 5", "fours", 0),
    ("yatzy-scandinavian", "1 1 2 3 4", "fives", 0),
    ("yatzy-scandinavian", "1 1 2 3 4", "sixes", 0),
    ("yahtzee", "2 3 4 5 6", "ones", 0),
    ("yahtzee", "1 3 4 5 6", "twos", 0),
    ("yahtzee", "1 1 2 2 4", "threes", 0),
    ("yahtzee", "1 1 2 3 5", "fours", 0),
    ("yahtzee", "1 1 2 3 4", "fives", 0),
    ("yahtzee", "1 1 2 3 4", "sixes", 0),
    ("yahtzee+no-bonus", "2 2 3 3 3", "full_house", 25),
    ("yahtzee+forced-joker", "2 2 3 3 3", "full_house", 25),
    ("yahtzee+no-joker", "5 5 5 5 5", "full_house", 0),
];

fn dice(s: &str) -> Dice {
    let faces: Vec<u8> = s.split_whitespace().map(|t| t.parse().unwrap()).collect();
    Dice::from_faces(&faces).unwrap()
}

#[test]
fn scoring_table() {
    let mut failures = Vec::new();
    for &(vid, d, cid, expected) in CASES {
        let v = Variant::by_id(vid).unwrap_or_else(|| panic!("no variant {vid}"));
        let c = v.category_index(cid).unwrap_or_else(|| panic!("no category {cid} in {vid}"));
        let got = v.score(c, &dice(d));
        if got != expected {
            failures.push(format!("{vid} {cid} [{d}]: expected {expected}, got {got}"));
        }
    }
    assert!(failures.is_empty(), "scoring failures:\n{}", failures.join("\n"));
}

#[test]
fn table_covers_every_category_of_the_main_variants() {
    for vid in ["yatzy-scandinavian", "yahtzee"] {
        let v = Variant::by_id(vid).unwrap();
        let tested: BTreeSet<&str> = CASES.iter().filter(|c| c.0 == vid).map(|c| c.2).collect();
        for cat in v.categories() {
            assert!(tested.contains(cat.id.as_str()), "{vid}: no scoring case for {}", cat.id);
            // A non-zero and a zero case for every category except Chance and the upper boxes' zero sides.
            let values: BTreeSet<u16> = CASES.iter().filter(|c| c.0 == vid && c.2 == cat.id).map(|c| c.3).collect();
            if cat.id != "chance" {
                assert!(values.contains(&0) && values.len() > 1, "{vid} {}: needs zero and non-zero cases", cat.id);
            }
        }
    }
}

#[test]
fn scandinavian_maximum_is_374() {
    // The best possible hand in each box plus the bonus.
    let v = Variant::scandinavian();
    let best: u16 = (0..v.num_categories())
        .map(|c| yatzy_solver::dice::all_multisets(5).iter().map(|d| v.score(c, d)).max().unwrap())
        .sum();
    assert_eq!(best + 50, 374);
}

#[test]
fn every_score_matches_a_reference_implementation() {
    // An independent, deliberately plain implementation of the Scandinavian and American scoring rules,
    // checked on all 252 hands.
    for d in yatzy_solver::dice::all_multisets(5) {
        let f = d.faces();
        let cnt = |v: u8| f.iter().filter(|&&x| x == v).count() as u16;
        let sum: u16 = f.iter().map(|&x| u16::from(x)).sum();
        let pairs: Vec<u16> = (1..=6).rev().filter(|&v| cnt(v) >= 2).map(u16::from).collect();
        let kind = |n: u16| (1..=6).rev().find(|&v| cnt(v) >= n).map(u16::from);
        let is_fh = (1..=6).any(|a| cnt(a) == 3 && (1..=6).any(|b| b != a && cnt(b) == 2));
        let has = |s: &[u8]| s.iter().all(|x| f.contains(x));
        let scand = [
            ("one_pair", pairs.first().map_or(0, |p| 2 * p)),
            ("two_pairs", if pairs.len() >= 2 { 2 * (pairs[0] + pairs[1]) } else { 0 }),
            ("three_of_a_kind", kind(3).map_or(0, |v| 3 * v)),
            ("four_of_a_kind", kind(4).map_or(0, |v| 4 * v)),
            ("small_straight", if f == [1, 2, 3, 4, 5] { 15 } else { 0 }),
            ("large_straight", if f == [2, 3, 4, 5, 6] { 20 } else { 0 }),
            ("full_house", if is_fh { sum } else { 0 }),
            ("chance", sum),
            ("yatzy", if kind(5).is_some() { 50 } else { 0 }),
        ];
        let v = Variant::scandinavian();
        for (id, want) in scand {
            assert_eq!(v.score(v.category_index(id).unwrap(), &d), want, "scandinavian {id} [{d}]");
        }
        let amer = [
            ("three_of_a_kind", if kind(3).is_some() { sum } else { 0 }),
            ("four_of_a_kind", if kind(4).is_some() { sum } else { 0 }),
            ("full_house", if is_fh { 25 } else { 0 }),
            ("small_straight", if has(&[1, 2, 3, 4]) || has(&[2, 3, 4, 5]) || has(&[3, 4, 5, 6]) { 30 } else { 0 }),
            ("large_straight", if has(&[1, 2, 3, 4, 5]) || has(&[2, 3, 4, 5, 6]) { 40 } else { 0 }),
            ("yahtzee", if kind(5).is_some() { 50 } else { 0 }),
            ("chance", sum),
        ];
        let v = Variant::american();
        for (id, want) in amer {
            assert_eq!(v.score(v.category_index(id).unwrap(), &d), want, "american {id} [{d}]");
        }
        for face in 1..=6u8 {
            for v in [Variant::scandinavian(), Variant::american()] {
                assert_eq!(v.score(usize::from(face - 1), &d), u16::from(face) * cnt(face));
            }
        }
    }
}
