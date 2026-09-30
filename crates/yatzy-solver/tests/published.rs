//! The published values (SPEC 5.3): each full variant is solved and its expected score compared, both to the
//! published digits and to the full-precision value pinned here, so any change that moves a value is noticed.

#![cfg(feature = "verify")]

use yatzy_solver::table::hex;
use yatzy_solver::verify::PUBLISHED;
use yatzy_solver::{Precision, State, Table, TurnModel};

const PINNED: &[(&str, f64)] = &[
    ("american", 254.5896094820),
    ("american+no-bonus", 245.8707745141),
    ("yatzy-scandinavian-ls2012-code", 248.6328539121),
    ("yatzy-scandinavian", 248.4399893779),
];

/// SHA-256 of the value arrays (f32 and f64): the same on every platform (SPEC 5.3). Also in
/// `release/values.sha256`.
const VALUES_HASHES: &[(&str, &str, &str)] = &[
    (
        "yatzy-scandinavian",
        "3a5bb59a68632c6236028a6753e01ee39c92c6294d669ad55ec26290e651cf7d",
        "28f2e4e27877a88a8acb2a5a18ce42c1ed12ef47c488568cebc288f149d320e8",
    ),
    (
        "american",
        "588e16790260e43cfce2222a96e517cdc15091600672234a1da92639abde7c9b",
        "e0d99a6affa13216707222ace6289885320b2da2304571c02b1c448685216e62",
    ),
];

#[test]
fn published_values_reproduce() {
    for p in PUBLISHED {
        let v = p.variant();
        let m = TurnModel::new(&v);
        let t: Vec<f64> = m.solve();
        let x = t[m.space().index(&State::new())];
        assert!(p.matches(x), "{}: {x} does not reproduce {}", p.variant, p.expected);
        let pinned = PINNED.iter().find(|q| q.0 == p.variant).expect("every published value is pinned").1;
        assert!((x - pinned).abs() < 1e-9, "{}: {x:.10} != {pinned:.10}", p.variant);
        if let Some((_, f32_hash, f64_hash)) = VALUES_HASHES.iter().find(|h| h.0 == p.variant) {
            let f64_table = Table::from_values(&v, Precision::F64, t.clone());
            assert_eq!(
                hex(&f64_table.values_hash()),
                *f64_hash,
                "{}: f64 table differs from the pinned hash",
                p.variant
            );
            let f32_table = Table::from_values(&v, Precision::F32, t);
            assert_eq!(
                hex(&f32_table.values_hash()),
                *f32_hash,
                "{}: f32 table differs from the pinned hash",
                p.variant
            );
        }
    }
}
