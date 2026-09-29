//! The brute-force cross-check (F8): on reduced games, the fast solver and the brute-force reference solver,
//! both in exact rational arithmetic, agree exactly on every reachable state; the f64 solver agrees to rounding.

#![cfg(feature = "verify")]

use num_rational::BigRational;
use yatzy_solver::verify::{BruteForce, reduced_variants};
use yatzy_solver::{State, TurnModel};

#[test]
fn reduced_games_match_brute_force_exactly() {
    let mut starts = Vec::new();
    for v in reduced_variants() {
        let model = TurnModel::new(&v);
        let exact: Vec<BigRational> = model.solve();
        let fast: Vec<f64> = model.solve();
        let mut bf = BruteForce::<BigRational>::new(&v);
        let start = bf.state_value(&State::new());
        let mut checked = 0;
        for (s, want) in bf.states() {
            let i = model.space().index(s);
            assert_eq!(&exact[i], want, "{}: state {}", v.id(), v.format_state(s));
            let w = num_traits::ToPrimitive::to_f64(want).unwrap();
            assert!((fast[i] - w).abs() <= 1e-12 * w.abs().max(1.0), "{}: f64 {} vs {w}", v.id(), fast[i]);
            checked += 1;
        }
        assert!(checked > 1);
        starts.push((v.id().to_string(), start.clone()));
        eprintln!(
            "{}: {checked} states match, start value {}",
            v.id(),
            num_traits::ToPrimitive::to_f64(&start).unwrap()
        );
    }
    // The joker rules must make a difference in these games, or they are not tested.
    let start = |id: &str| starts.iter().find(|s| s.0 == id).unwrap().1.clone();
    assert_ne!(start("reduced-american+free"), start("reduced-american+forced"));
    assert_ne!(start("reduced-american+free"), start("reduced-american+none"));
}
