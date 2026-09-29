//! M3 acceptance: option values and regret are consistent with V (checked by explicit enumeration through the
//! rules engine), batch queries equal single queries, and simulated means fall within their interval of the
//! exact mean.

use yatzy_solver::dice::all_multisets;
use yatzy_solver::rules::{Action, RulesError};
use yatzy_solver::simulate::{OptimalPolicy, Policy, RandomPolicy, play_game, simulate};
use yatzy_solver::{Dice, Game, Rng, Situation, Solver, State, Variant};

fn random_state(v: &Variant, rng: &mut Rng) -> State {
    loop {
        let filled = rng.next_u64() as u32 & v.all_mask();
        if filled == v.all_mask() {
            continue;
        }
        let upper = if filled & v.upper_mask() != 0 { rng.below(64) as u16 } else { 0 };
        let yfilled = v.all_same_box().is_some_and(|y| filled & (1 << y) != 0);
        return State { filled, upper, bonus_armed: yfilled && rng.below(2) == 1 };
    }
}

fn random_situation(v: &Variant, rng: &mut Rng) -> Situation {
    Situation {
        state: random_state(v, rng),
        dice: rng.roll(v.dice()),
        rolls_left: rng.below(u64::from(v.rolls())) as u8,
    }
}

/// The value of a keep by enumerating every outcome of the reroll through the rules engine.
fn keep_by_enumeration(solver: &Solver, sit: &Situation, keep: &Dice) -> f64 {
    let v = solver.variant();
    let m = v.dice() - keep.len();
    let mut sum = 0.0;
    for rolled in all_multisets(m) {
        let next = v.apply_keep(sit, keep, &rolled).unwrap();
        sum += rolled.arrangements() as f64 * solver.situation_value(&next).unwrap();
    }
    sum / 6f64.powi(m as i32)
}

#[test]
fn option_values_and_regret_are_consistent_with_v() {
    for id in ["american", "yatzy-scandinavian"] {
        let solver = Solver::build(&Variant::by_id(id).unwrap());
        let v = solver.variant();
        let mut rng = Rng::new(2026);
        for i in 0..60 {
            let sit = random_situation(v, &mut rng);
            let options = solver.option_values(&sit).unwrap();
            assert_eq!(options.iter().map(|o| o.action).collect::<Vec<_>>(), v.legal_actions(&sit).unwrap());
            let best = options.iter().map(|o| o.value).fold(f64::NEG_INFINITY, f64::max);
            assert_eq!(solver.situation_value(&sit).unwrap(), best);
            // Every option, recomputed independently of the turn tables.
            for (k, o) in options.iter().enumerate() {
                let want = match o.action {
                    Action::Score(c) => {
                        let (next, scored) = v.apply_score(&sit.state, &sit.dice, c).unwrap();
                        f64::from(scored.total()) + solver.state_value(&next)
                    }
                    // Enumerating every keep is slow; check a spread of them.
                    Action::Keep(keep) if (k + i) % 4 == 0 => keep_by_enumeration(&solver, &sit, &keep),
                    Action::Keep(_) => continue,
                };
                assert!(
                    (o.value - want).abs() < 1e-9,
                    "{id}: {} {:?}: {} vs {want}",
                    v.format_situation(&sit),
                    o.action,
                    o.value
                );
            }
            // Regret: zero for the best, the gap otherwise, never negative.
            let bests = solver.best_options(&sit).unwrap();
            assert!(!bests.is_empty());
            assert_eq!(solver.best_action(&sit).unwrap(), bests[0].action);
            for o in &options {
                let r = solver.regret(&sit, &o.action).unwrap();
                assert!(r >= 0.0);
                if bests.iter().any(|b| b.action == o.action) {
                    assert_eq!(r, 0.0);
                } else {
                    assert!((r - (best - o.value)).abs() < 1e-12);
                }
            }
        }
        // V is the expectation of the first roll's situation values.
        for _ in 0..5 {
            let s = random_state(v, &mut rng);
            let mut sum = 0.0;
            for hand in all_multisets(v.dice()) {
                let sit = v.start_turn(&s, hand).unwrap();
                sum += hand.arrangements() as f64 * solver.situation_value(&sit).unwrap();
            }
            let ev = sum / 6f64.powi(v.dice() as i32);
            assert!((ev - solver.state_value(&s)).abs() < 1e-9, "{id}: {}", v.format_state(&s));
        }
    }
}

#[test]
fn ties_are_explicit() {
    let solver = Solver::build(&Variant::scandinavian());
    let v = solver.variant();
    // Last turn, only Yatzy open, 1 1 2 2 3 with two rerolls: keeping 1 1 and keeping 2 2 are symmetric, so
    // both are best (their values are sums in different orders, equal up to rounding).
    let only_yatzy = v.all_mask() & !(1 << v.category_index("yatzy").unwrap());
    let state = State { filled: only_yatzy, upper: 63, bonus_armed: false };
    let sit = Situation { state, dice: Dice::from_faces(&[1, 1, 2, 2, 3]).unwrap(), rolls_left: 2 };
    let bests: Vec<String> = solver.best_options(&sit).unwrap().iter().map(|o| v.format_action(&o.action)).collect();
    assert_eq!(bests, ["keep 1 1", "keep 2 2"]);
    assert_eq!(solver.best_action(&sit).unwrap(), v.parse_action("keep 1 1").unwrap());
    assert_eq!(solver.regret(&sit, &v.parse_action("keep 2 2").unwrap()).unwrap(), 0.0);
    assert!(solver.regret(&sit, &v.parse_action("keep 3").unwrap()).unwrap() > 0.0);
    // No rerolls left: a single option, the only open box.
    let sit = Situation { rolls_left: 0, ..sit };
    let bests = solver.best_options(&sit).unwrap();
    assert_eq!(bests.len(), 1);
    assert_eq!(bests[0].value, 0.0);
}

#[test]
fn errors_instead_of_panics() {
    let solver = Solver::build(&Variant::american());
    let v = solver.variant();
    let s = State::new();
    let d = Dice::from_faces(&[1, 2, 3, 4, 5]).unwrap();
    assert!(matches!(
        solver.option_values(&Situation { state: s, dice: Dice::from_faces(&[1, 2]).unwrap(), rolls_left: 0 }),
        Err(RulesError::WrongDiceCount { .. })
    ));
    assert!(solver.option_values(&Situation { state: s, dice: d, rolls_left: 3 }).is_err());
    let over = State { filled: v.all_mask(), upper: 0, bonus_armed: false };
    assert_eq!(solver.option_values(&Situation { state: over, dice: d, rolls_left: 0 }), Err(RulesError::GameOver));
    let sit = Situation { state: s, dice: d, rolls_left: 0 };
    assert!(solver.regret(&sit, &Action::Keep(Dice::EMPTY)).is_err());
    assert!(solver.regret(&sit, &Action::Score(99)).is_err());
    assert_eq!(solver.state_value(&over), 0.0);
}

#[test]
fn batch_equals_single() {
    let solver = Solver::build(&Variant::scandinavian());
    let mut rng = Rng::new(7);
    let mut sits: Vec<Situation> = (0..500).map(|_| random_situation(solver.variant(), &mut rng)).collect();
    sits.push(Situation { rolls_left: 5, ..sits[0] });
    let batch = solver.option_values_batch(&sits);
    for (s, b) in sits.iter().zip(&batch) {
        assert_eq!(&solver.option_values(s), b);
    }
    let states: Vec<State> = sits.iter().map(|s| s.state).collect();
    let vals = solver.state_values(&states);
    assert!(states.iter().zip(&vals).all(|(s, &x)| solver.state_value(s) == x));
}

#[test]
fn generator_is_the_documented_xoshiro256starstar() {
    use yatzy_solver::simulate::mix;
    // Reference values from an independent implementation of SplitMix64, xoshiro256** and the stream
    // derivation (docs/queries.md). mix(0) is the canonical first SplitMix64 output.
    assert_eq!(mix(0), 0xe220a8397b1dcdaf);
    assert_eq!(mix(12345), 0x22118258a9d111a0);
    let mut r = Rng::new(0);
    assert_eq!(
        [r.next_u64(), r.next_u64(), r.next_u64()],
        [0x99ec5f36cb75f2b4, 0xbf6e1f784956452a, 0x1a5f849d4933e6e0]
    );
    let mut r = Rng::for_dice(42, 7, 3);
    let dice: Vec<u8> = (0..15).map(|_| r.die()).collect();
    assert_eq!(dice, [5, 4, 5, 6, 2, 3, 1, 4, 2, 1, 5, 6, 2, 4, 2]);
    let mut r = Rng::for_policy(42, 7);
    assert_eq!([r.next_u64(), r.next_u64()], [0x56de4991b7cb08eb, 0x292c127d7a2eec8c]);
    // Blocks: a reroll of m dice uses the first m of a full block of five.
    let mut a = Rng::for_dice(42, 7, 3);
    let mut b = Rng::for_dice(42, 7, 3);
    assert_eq!(a.roll_block(5, 2), Dice::from_faces(&[5, 4]).unwrap());
    assert_eq!(b.roll_block(5, 5), Dice::from_faces(&[5, 4, 5, 6, 2]).unwrap());
    assert_eq!(a.roll_block(5, 3), b.roll_block(5, 3));
}

#[test]
fn dice_do_not_depend_on_the_policy() {
    // Common random numbers: under the same seed, every policy sees the same first roll of every turn, and the
    // same dice on each reroll it shares.
    let solver = Solver::build(&Variant::scandinavian());
    let v = solver.variant();
    for game in 0..50 {
        let opt = play_game(v, &mut OptimalPolicy { solver: &solver }, 11, game, true).unwrap();
        let rnd = play_game(v, &mut RandomPolicy, 11, game, true).unwrap();
        let firsts = |log: &yatzy_solver::simulate::GameLog| -> Vec<Dice> {
            log.decisions.iter().filter(|d| d.situation.rolls_left == v.rolls() - 1).map(|d| d.situation.dice).collect()
        };
        // Each turn's first roll appears once per turn in the log (the first decision of the turn).
        let (a, b) = (firsts(&opt), firsts(&rnd));
        assert_eq!(a.len(), v.num_categories());
        assert_eq!(a, b, "game {game}");
        for t in 0..v.num_categories() as u32 {
            assert_eq!(a[t as usize], Rng::for_dice(11, game, t).roll_block(5, 5));
        }
    }
}

#[test]
fn simulation_is_deterministic_and_parallel_equals_sequential() {
    let solver = Solver::build(&Variant::american());
    let v = solver.variant();
    let par = solver.simulate_optimal(200, 99, true);
    let seq = simulate(v, &mut OptimalPolicy { solver: &solver }, 200, 99, true).unwrap();
    assert_eq!(par.scores, seq.scores);
    assert_eq!(par.logs, seq.logs);
    assert_eq!(solver.simulate_optimal(200, 99, false).scores, par.scores);
    assert_ne!(solver.simulate_optimal(200, 100, false).scores, par.scores);
    // A log replays: every decision is legal and optimal, and the scores add up.
    for log in &par.logs {
        let mut game = Game::new();
        for d in &log.decisions {
            assert_eq!(solver.regret(&d.situation, &d.action).unwrap(), 0.0);
            if let Action::Score(c) = d.action {
                assert_eq!(Some(game.score(v, &d.situation.dice, c).unwrap()), d.scored);
            }
        }
        assert_eq!(game.total(), log.final_score);
        assert_eq!(log.to_lines(v).last().unwrap(), &format!("final {}", log.final_score));
    }
}

#[test]
fn caller_policies_and_illegal_choices() {
    let solver = Solver::build(&Variant::scandinavian());
    let v = solver.variant();
    let random = simulate(v, &mut RandomPolicy, 2000, 1, false).unwrap().summary();
    assert!(random.mean < 120.0, "random play averages {}", random.mean);
    struct Cheat;
    impl Policy for Cheat {
        fn choose(&mut self, _: &Variant, _: &Game, _: &Situation, _: &[Action], _: &mut Rng) -> Action {
            Action::Score(99)
        }
    }
    let err = play_game(v, &mut Cheat, 1, 0, false).unwrap_err();
    assert_eq!(err.error, RulesError::CategoryNotAllowed(99));
}

#[test]
fn simulated_means_are_within_their_interval_of_the_exact_mean() {
    for id in ["american", "yatzy-scandinavian"] {
        let solver = Solver::build(&Variant::by_id(id).unwrap());
        let exact = solver.state_value(&State::new());
        let sum = solver.simulate_optimal(50_000, 2026, false).summary();
        let z = (sum.mean - exact) / sum.std_error;
        eprintln!(
            "{id}: exact {exact:.4}, simulated {:.4} +- {:.4} (z = {z:.2}), sd {:.4}, median {}",
            sum.mean, sum.std_error, sum.std_dev, sum.median
        );
        assert!(z.abs() < 4.0, "{id}: simulated mean {} is {z:.1} standard errors from {exact}", sum.mean);
    }
}

#[test]
fn tie_sets_agree_between_f32_and_f64_tables() {
    use yatzy_solver::{Precision, Table};
    for id in ["yatzy-scandinavian", "american"] {
        let f64s = Solver::build(&Variant::by_id(id).unwrap());
        let v = f64s.variant().clone();
        let f32s = Solver::from_table(&Table::from_values(&v, Precision::F32, f64s.values().to_vec()));
        let mut sits: Vec<Situation> = Vec::new();
        for log in f64s.simulate_optimal(300, 8, true).logs {
            sits.extend(log.decisions.iter().map(|d| d.situation));
        }
        let mut rng = Rng::new(31);
        sits.extend((0..10_000).map(|_| random_situation(&v, &mut rng)));
        let mut tied = 0;
        for sit in &sits {
            let a: Vec<Action> = f64s.best_options(sit).unwrap().iter().map(|o| o.action).collect();
            let b: Vec<Action> = f32s.best_options(sit).unwrap().iter().map(|o| o.action).collect();
            assert_eq!(a, b, "{id}: {}", v.format_situation(sit));
            tied += usize::from(a.len() > 1);
        }
        // The sample must contain ties, or it tests nothing.
        assert!(tied > 50, "{id}: only {tied} situations with ties");
    }
}

#[test]
fn score_so_far_plus_points_to_come_is_the_final_score() {
    // score_so_far includes every bonus already earned; the points still to come include every bonus not yet
    // earned. Checked at every decision of every logged game, with the card replayed independently.
    for id in ["american", "yatzy-scandinavian"] {
        let solver = Solver::build(&Variant::by_id(id).unwrap());
        let v = solver.variant();
        let mut upper_bonus_games = 0;
        for log in solver.simulate_optimal(300, 3, true).logs {
            let mut card = Game::new();
            for (i, d) in log.decisions.iter().enumerate() {
                let score_so_far = card.total();
                let to_come: u16 = log.decisions[i..].iter().filter_map(|x| x.scored).map(|s| s.total()).sum();
                assert_eq!(score_so_far + to_come, log.final_score, "{id} game {}", log.game);
                assert_eq!(*card.state(), d.situation.state);
                if let Action::Score(c) = d.action {
                    card.score(v, &d.situation.dice, c).unwrap();
                }
            }
            assert_eq!(card.total(), log.final_score);
            upper_bonus_games += usize::from(card.upper_bonus() > 0);
        }
        assert!(upper_bonus_games > 50, "{id}: the upper bonus must be exercised");
    }
}
