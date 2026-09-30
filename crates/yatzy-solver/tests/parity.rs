//! The cross-language golden set (SPEC 5.3): `golden/parity.jsonl` holds situations with the exact value of
//! every option, from f64 solves. Rust, Python and WASM must reproduce every value bit for bit.
//! Regenerate with `UPDATE_GOLDEN=1 cargo test --test parity` (a value change is a deliberate, reviewed event).

use yatzy_solver::export::{Source, export_rows};
use yatzy_solver::{Solver, Variant};

fn golden_lines() -> Vec<String> {
    let mut out = Vec::new();
    for id in ["yatzy-scandinavian", "american"] {
        let s = Solver::build(&Variant::by_id(id).unwrap());
        let v = s.variant();
        let mut rows = export_rows(&s, Source::Optimal, 2026, 100);
        rows.extend(export_rows(&s, Source::Uniform, 2026, 50));
        for r in rows {
            let opts: Vec<String> =
                r.options.iter().map(|o| format!("[{},{:?}]", v.action_code(&o.action).unwrap(), o.value)).collect();
            out.push(format!(
                "{{\"variant\":\"{id}\",\"situation\":\"{}\",\"state_value\":{:?},\"options\":[{}]}}",
                v.format_situation(&r.situation),
                s.state_value(&r.situation.state),
                opts.join(",")
            ));
        }
    }
    out
}

#[test]
fn golden_parity_set() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/parity.jsonl");
    let lines = golden_lines();
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(path, lines.join("\n") + "\n").unwrap();
    }
    let file = std::fs::read_to_string(path).expect("golden/parity.jsonl exists");
    let want: Vec<&str> = file.lines().collect();
    assert_eq!(want.len(), lines.len());
    for (a, b) in lines.iter().zip(want) {
        assert_eq!(a, b);
    }
}
