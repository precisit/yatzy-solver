//! Rules-engine tests: bonuses, the joker rules, forced order, legal actions and transitions.

use yatzy_solver::{Action, Dice, Game, RulesError, Situation, State, Variant};

fn dice(s: &str) -> Dice {
    let faces: Vec<u8> = s.split_whitespace().map(|t| t.parse().unwrap()).collect();
    Dice::from_faces(&faces).unwrap()
}

fn state(v: &Variant, s: &str) -> State {
    v.parse_state(s).unwrap()
}

fn cat(v: &Variant, id: &str) -> usize {
    v.category_index(id).unwrap()
}

/// (category id, points in the box, upper bonus, Yahtzee bonus) for every legal choice.
fn choices(v: &Variant, s: &State, d: &str) -> Vec<(String, u16, u16, u16)> {
    v.score_choices(s, &dice(d))
        .unwrap()
        .into_iter()
        .map(|x| (v.categories()[x.category].id.clone(), x.points, x.upper_bonus, x.all_same_bonus))
        .collect()
}

fn choice(v: &Variant, s: &State, d: &str, id: &str) -> Option<(u16, u16, u16)> {
    choices(v, s, d).into_iter().find(|c| c.0 == id).map(|c| (c.1, c.2, c.3))
}

#[test]
fn upper_bonus_is_paid_once_when_the_threshold_is_reached() {
    let v = Variant::scandinavian();
    let s = state(&v, "upper 60 | filled ones,twos,fours,fives");
    assert_eq!(choice(&v, &s, "3 3 1 2 4", "threes"), Some((6, 50, 0)));
    assert_eq!(choice(&v, &s, "3 1 1 2 4", "threes"), Some((3, 50, 0)));
    assert_eq!(choice(&v, &s, "1 1 1 2 4", "threes"), Some((0, 0, 0)));
    let s = state(&v, "upper 66 | filled ones,twos,fours,fives");
    assert_eq!(choice(&v, &s, "3 3 1 2 4", "threes"), Some((6, 0, 0)));
    // Lower categories never pay the upper bonus.
    let s = state(&v, "upper 62 | filled ones,twos,fours,fives");
    assert_eq!(choice(&v, &s, "6 6 6 6 6", "chance"), Some((30, 0, 0)));
}

#[test]
fn game_total_includes_bonuses() {
    let v = Variant::scandinavian();
    let mut g = Game::new();
    for (face, d) in [(1, "1 1 1 2 3"), (2, "2 2 2 1 3"), (3, "3 3 3 1 2"), (4, "4 4 4 1 2"), (5, "5 5 5 1 2")] {
        g.score(&v, &dice(d), face - 1).unwrap();
    }
    assert_eq!(g.state().upper, 45);
    assert_eq!(g.upper_bonus(), 0);
    let scored = g.score(&v, &dice("6 6 6 1 2"), 5).unwrap();
    assert_eq!((scored.points, scored.upper_bonus), (18, 50));
    assert_eq!(g.total(), 63 + 50);
    for (id, d) in [
        ("one_pair", "6 6 1 2 3"),
        ("two_pairs", "6 6 5 5 3"),
        ("three_of_a_kind", "6 6 6 5 3"),
        ("four_of_a_kind", "6 6 6 6 3"),
        ("small_straight", "1 2 3 4 5"),
        ("large_straight", "2 3 4 5 6"),
        ("full_house", "6 6 6 5 5"),
        ("chance", "6 6 6 6 6"),
    ] {
        g.score(&v, &dice(d), cat(&v, id)).unwrap();
    }
    assert!(!v.is_over(g.state()));
    assert_eq!(g.score(&v, &dice("6 6 6 6 6"), cat(&v, "chance")), Err(RulesError::CategoryFilled(13)));
    g.score(&v, &dice("6 6 6 6 6"), cat(&v, "yatzy")).unwrap();
    assert!(v.is_over(g.state()));
    assert_eq!(g.total(), 63 + 50 + 12 + 22 + 18 + 24 + 15 + 20 + 28 + 30 + 50);
    assert_eq!(v.score_choices(g.state(), &dice("1 1 1 1 1")), Err(RulesError::GameOver));
}

#[test]
fn free_joker_verhoeff() {
    let v = Variant::american();
    // Yahtzee box holds 50, Fives open: free placement, the bonus everywhere, no joker points.
    let s = state(&v, "upper 0 | filled five_of_a_kind | five_of_a_kind 50");
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "fives"), Some((25, 0, 100)));
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "full_house"), Some((0, 0, 100)));
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "large_straight"), Some((0, 0, 100)));
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "chance"), Some((25, 0, 100)));
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "ones"), Some((0, 0, 100)));
    assert_eq!(choices(&v, &s, "5 5 5 5 5").len(), 12);
    // Fives filled: the joker applies.
    let s = state(&v, "upper 20 | filled fives,five_of_a_kind | five_of_a_kind 50");
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "full_house"), Some((25, 0, 100)));
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "small_straight"), Some((30, 0, 100)));
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "large_straight"), Some((40, 0, 100)));
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "ones"), Some((0, 0, 100)));
    // Yahtzee box scratched: the joker still applies, no bonus.
    let s = state(&v, "upper 20 | filled fives,five_of_a_kind | five_of_a_kind 0");
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "full_house"), Some((25, 0, 0)));
    // Yahtzee box open: no joker, no bonus.
    let s = state(&v, "upper 20 | filled fives");
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "full_house"), Some((0, 0, 0)));
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "five_of_a_kind"), Some((50, 0, 0)));
}

#[test]
fn forced_joker_official() {
    let v = Variant::by_id("american+forced-joker").unwrap();
    let s = state(&v, "upper 0 | filled five_of_a_kind | five_of_a_kind 50");
    assert_eq!(choices(&v, &s, "5 5 5 5 5"), vec![("fives".to_string(), 25, 0, 100)]);
    // Own upper box filled: any open lower box, with joker points.
    let s = state(&v, "upper 20 | filled fives,five_of_a_kind | five_of_a_kind 50");
    let got: Vec<String> = choices(&v, &s, "5 5 5 5 5").into_iter().map(|c| c.0).collect();
    assert_eq!(got, ["three_of_a_kind", "four_of_a_kind", "full_house", "small_straight", "large_straight", "chance"]);
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "large_straight"), Some((40, 0, 100)));
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "three_of_a_kind"), Some((25, 0, 100)));
    // Only upper boxes left: any of them, for 0 (plus the bonus).
    let s = state(
        &v,
        "upper 20 | filled fives,three_of_a_kind,four_of_a_kind,full_house,small_straight,large_straight,five_of_a_kind,chance | five_of_a_kind 0",
    );
    assert_eq!(choices(&v, &s, "5 5 5 5 5").len(), 5);
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "sixes"), Some((0, 0, 0)));
    // The upper bonus can be reached with a forced joker.
    let s = state(&v, "upper 38 | filled ones,twos,threes,fours,five_of_a_kind | five_of_a_kind 50");
    assert_eq!(choices(&v, &s, "5 5 5 5 5"), vec![("fives".to_string(), 25, 35, 100)]);
}

#[test]
fn no_joker_and_no_bonus() {
    let v = Variant::by_id("american+no-joker").unwrap();
    let s = state(&v, "upper 20 | filled fives,five_of_a_kind | five_of_a_kind 50");
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "full_house"), Some((0, 0, 100)));
    let v = Variant::by_id("american+no-bonus").unwrap();
    let s = state(&v, "upper 20 | filled fives,five_of_a_kind");
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "full_house"), Some((0, 0, 0)));
    assert_eq!(choice(&v, &s, "5 5 5 5 5", "chance"), Some((25, 0, 0)));
    assert_eq!(v.format_state(&s), "upper 20 | filled fives,five_of_a_kind");
}

#[test]
fn all_same_box_status_follows_the_score() {
    let v = Variant::american();
    let y = cat(&v, "five_of_a_kind");
    let (s, _) = v.apply_score(&State::new(), &dice("2 2 2 2 2"), y).unwrap();
    assert!(s.bonus_armed);
    let (s, _) = v.apply_score(&State::new(), &dice("2 2 2 2 1"), y).unwrap();
    assert!(!s.bonus_armed && s.is_filled(y));
}

#[test]
fn forced_order_allows_only_the_next_category() {
    let v = Variant::by_id("yatzy-scandinavian+forced").unwrap();
    let s = state(&v, "upper 4 | filled ones,twos");
    assert_eq!(choices(&v, &s, "3 3 3 3 3"), vec![("threes".to_string(), 15, 0, 0)]);
    assert_eq!(
        v.apply_score(&s, &dice("3 3 3 3 3"), cat(&v, "yatzy")),
        Err(RulesError::CategoryNotAllowed(cat(&v, "yatzy")))
    );
}

#[test]
fn legal_actions_and_keeps() {
    let v = Variant::scandinavian();
    let sit = v.start_turn(&State::new(), dice("1 3 3 5 6")).unwrap();
    assert_eq!(sit.rolls_left, 2);
    let actions = v.legal_actions(&sit).unwrap();
    let keeps = actions.iter().filter(|a| matches!(a, Action::Keep(_))).count();
    assert_eq!(keeps, 2 * 3 * 2 * 2 - 1);
    assert_eq!(actions.len(), 15 + keeps);
    assert_eq!(actions[0], Action::Score(0));
    assert_eq!(actions[15], Action::Keep(Dice::EMPTY));
    assert!(!actions.contains(&Action::Keep(sit.dice)));

    let keep = dice("3 3");
    let next = v.apply_keep(&sit, &keep, &dice("3 4 4")).unwrap();
    assert_eq!(next.dice, dice("3 3 3 4 4"));
    assert_eq!(next.rolls_left, 1);
    assert_eq!(v.apply_keep(&sit, &keep, &dice("3 4")), Err(RulesError::BadReroll { expected: 3, got: 2 }));
    assert_eq!(v.apply_keep(&sit, &dice("2"), &dice("1 1 1 1")), Err(RulesError::BadKeep));
    assert_eq!(v.apply_keep(&sit, &sit.dice, &Dice::EMPTY), Err(RulesError::BadKeep));
    let last = Situation { rolls_left: 0, ..sit };
    assert_eq!(v.apply_keep(&last, &keep, &dice("1 1 1")), Err(RulesError::NoRollsLeft));
    assert_eq!(v.legal_actions(&last).unwrap().len(), 15);
    assert_eq!(v.start_turn(&State::new(), dice("1 2 3 4")), Err(RulesError::WrongDiceCount { expected: 5, got: 4 }));
}
