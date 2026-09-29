//! Property tests: random games under every built-in variant, with invariants checked at every step.

use proptest::prelude::*;
use yatzy_solver::{Action, Dice, Game, Variant};

const VARIANTS: &[&str] = &[
    "yatzy-scandinavian",
    "yatzy-scandinavian+fh5",
    "yatzy-scandinavian+tp4",
    "yatzy-scandinavian+forced",
    "yatzy-scandinavian+fh5+tp4+forced",
    "yahtzee",
    "yahtzee+forced-joker",
    "yahtzee+no-joker",
    "yahtzee+no-bonus",
];

/// Plays one game driven by `seq` (dice faces and choices, used cyclically) and checks invariants.
fn play(v: &Variant, seq: &[u8]) -> Result<Game, TestCaseError> {
    let mut i = 0;
    let mut next = || {
        i += 1;
        seq[i % seq.len()]
    };
    let roll = |n: usize, next: &mut dyn FnMut() -> u8| {
        let faces: Vec<u8> = (0..n).map(|_| next() % 6 + 1).collect();
        Dice::from_faces(&faces).unwrap()
    };
    let mut game = Game::new();
    let mut earned = 0u16;
    while !v.is_over(game.state()) {
        let turn = game.state().turns_played();
        let mut sit = v.start_turn(game.state(), roll(v.dice(), &mut next)).unwrap();
        loop {
            // Notation round trip.
            let text = v.format_situation(&sit);
            prop_assert_eq!(v.parse_situation(&text).unwrap(), sit, "{}", text);
            let actions = v.legal_actions(&sit).unwrap();
            prop_assert!(actions.iter().any(|a| matches!(a, Action::Score(_))), "no category for {}", text);
            for a in &actions {
                prop_assert_eq!(v.parse_action(&v.format_action(a)).unwrap(), *a);
            }
            let a = actions[usize::from(next()) % actions.len()];
            match a {
                Action::Keep(k) => {
                    let rolled = roll(v.dice() - k.len(), &mut next);
                    sit = v.apply_keep(&sit, &k, &rolled).unwrap();
                }
                Action::Score(c) => {
                    let before = *game.state();
                    let scored = game.score(v, &sit.dice, c).unwrap();
                    earned += scored.total();
                    prop_assert_eq!(game.state().turns_played(), turn + 1);
                    prop_assert!(game.state().upper >= before.upper);
                    prop_assert_eq!(game.points(c), Some(scored.points));
                    break;
                }
            }
        }
    }
    prop_assert_eq!(game.total(), earned);
    Ok(game)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn random_games_keep_invariants(vi in 0..VARIANTS.len(), seq in prop::collection::vec(any::<u8>(), 1..200)) {
        let v = Variant::by_id(VARIANTS[vi]).unwrap();
        let g = play(&v, &seq)?;
        if v.id().starts_with("yatzy-scandinavian") {
            prop_assert!(g.total() <= 374);
        }
        prop_assert_eq!(g.upper_bonus() > 0, g.state().upper >= 63);
    }

    #[test]
    fn notation_of_any_dice_round_trips(faces in prop::collection::vec(1u8..=6, 5)) {
        let v = Variant::scandinavian();
        let d = Dice::from_faces(&faces).unwrap();
        let sit = v.start_turn(&yatzy_solver::State::new(), d).unwrap();
        prop_assert_eq!(v.parse_situation(&v.format_situation(&sit)).unwrap(), sit);
    }

    #[test]
    fn every_legal_score_is_accepted_and_every_other_rejected(
        vi in 0..VARIANTS.len(),
        filled in any::<u32>(),
        armed in any::<bool>(),
        faces in prop::collection::vec(1u8..=6, 5),
    ) {
        let v = Variant::by_id(VARIANTS[vi]).unwrap();
        let filled = filled & v.all_mask();
        prop_assume!(filled != v.all_mask());
        let yfilled = v.yahtzee_box().is_some_and(|y| filled & (1 << y) != 0);
        let s = yatzy_solver::State { filled, upper: 0, yahtzee_armed: armed && yfilled };
        let d = Dice::from_faces(&faces).unwrap();
        let legal: Vec<usize> = v.score_choices(&s, &d).unwrap().iter().map(|x| x.category).collect();
        prop_assert!(!legal.is_empty());
        for c in 0..v.num_categories() {
            prop_assert_eq!(v.apply_score(&s, &d, c).is_ok(), legal.contains(&c));
        }
    }
}
