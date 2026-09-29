//! The published values (SPEC 5.3): each full variant is solved and its expected score compared, both to the
//! published digits and to the full-precision value pinned here, so any change that moves a value is noticed.

#![cfg(feature = "verify")]

use yatzy_solver::table::hex;
use yatzy_solver::verify::PUBLISHED;
use yatzy_solver::{Precision, State, Table, TurnModel, Variant};

const PINNED: &[(&str, f64)] = &[
    ("yahtzee", 254.5896094820),
    ("yahtzee+no-bonus", 245.8707745141),
    ("yatzy-scandinavian+tp1", 248.6328539121),
    ("yatzy-scandinavian", 248.4399893779),
];

/// SHA-256 of the f32 value arrays: the same on every platform (SPEC 5.3).
const VALUES_HASHES: &[(&str, &str)] = &[
    ("yatzy-scandinavian", "3a5bb59a68632c6236028a6753e01ee39c92c6294d669ad55ec26290e651cf7d"),
    ("yahtzee", "588e16790260e43cfce2222a96e517cdc15091600672234a1da92639abde7c9b"),
];

#[test]
fn published_values_reproduce() {
    for p in PUBLISHED {
        let v = Variant::by_id(p.variant).unwrap();
        let m = TurnModel::new(&v);
        let t: Vec<f64> = m.solve();
        let x = t[m.space().index(&State::new())];
        assert!(p.matches(x), "{}: {x} does not reproduce {}", p.variant, p.expected);
        let pinned = PINNED.iter().find(|q| q.0 == p.variant).expect("every published value is pinned").1;
        assert!((x - pinned).abs() < 1e-9, "{}: {x:.10} != {pinned:.10}", p.variant);
        if let Some((_, want)) = VALUES_HASHES.iter().find(|h| h.0 == p.variant) {
            let table = Table::from_values(&v, Precision::F32, t);
            assert_eq!(hex(&table.values_hash()), *want, "{}: table differs from the pinned hash", p.variant);
        }
    }
}
