//! Export (F5): the three sources, the JSON Lines schema, and Parquet (feature `parquet`).

use yatzy_solver::export::{ReachableStates, Row, Source, export_rows, trajectory, write_jsonl};
use yatzy_solver::rules::Action;
use yatzy_solver::simulate::{OptimalPolicy, play_game};
use yatzy_solver::{Solver, State, Variant};

fn solver(id: &str) -> Solver {
    Solver::build(&Variant::by_id(id).unwrap())
}

fn check_rows(s: &Solver, rows: &[Row]) {
    let v = s.variant();
    for r in rows {
        let opts = s.option_values(&r.situation).unwrap();
        assert_eq!(r.options, opts);
        assert_eq!(r.best_value, opts.iter().map(|o| o.value).fold(f64::NEG_INFINITY, f64::max));
        if let Some(a) = r.chosen {
            assert!(opts.iter().any(|o| o.action == a));
        }
        v.check_state(&r.situation.state).unwrap();
    }
}

#[test]
fn optimal_trajectories_are_the_simulators_games() {
    let s = solver("american");
    let v = s.variant();
    for game in 0..20 {
        let rows = trajectory(&s, Source::Optimal, 5, game);
        let log = play_game(v, &mut OptimalPolicy { solver: &s }, 5, game, true).unwrap();
        assert_eq!(rows.len(), log.decisions.len());
        for (r, d) in rows.iter().zip(&log.decisions) {
            assert_eq!(r.situation, d.situation);
            assert_eq!(r.chosen, Some(d.action));
        }
        // score_so_far at each decision plus the points to come is the final score.
        let final_score = log.final_score;
        for (i, r) in rows.iter().enumerate() {
            let to_come: u16 = log.decisions[i..].iter().filter_map(|d| d.scored).map(|x| x.total()).sum();
            assert_eq!(r.score_so_far.unwrap() + to_come, final_score);
        }
        check_rows(&s, &rows);
    }
}

#[test]
fn exact_row_counts_and_determinism() {
    let s = solver("yatzy-scandinavian");
    for src in [Source::Optimal, Source::Perturbed(0.3), Source::Uniform] {
        let a = export_rows(&s, src, 9, 1234);
        assert_eq!(a.len(), 1234);
        assert_eq!(a, export_rows(&s, src, 9, 1234));
        assert_ne!(a, export_rows(&s, src, 10, 1234));
        // A prefix of a longer export.
        assert_eq!(a[..1000], export_rows(&s, src, 9, 1000)[..]);
        check_rows(&s, &a);
    }
}

#[test]
fn perturbed_share_and_limits() {
    let s = solver("yatzy-scandinavian");
    let off_best = |p: f64| {
        let rows = export_rows(&s, Source::Perturbed(p), 4, 20_000);
        let bad = rows.iter().filter(|r| s.regret(&r.situation, &r.chosen.unwrap()).unwrap() > 0.0).count();
        bad as f64 / rows.len() as f64
    };
    // The random flag: never set for optimal play; a non-random decision is always a best action; about p of
    // the decisions are random.
    let rows = export_rows(&s, Source::Perturbed(0.3), 4, 20_000);
    assert!(rows.iter().all(|r| r.random == Some(false) || r.random == Some(true)));
    for r in rows.iter().filter(|r| r.random == Some(false)) {
        assert_eq!(s.regret(&r.situation, &r.chosen.unwrap()).unwrap(), 0.0);
    }
    let share = rows.iter().filter(|r| r.random == Some(true)).count() as f64 / rows.len() as f64;
    assert!((share - 0.3).abs() < 0.02, "{share}");
    assert!(export_rows(&s, Source::Optimal, 4, 2000).iter().all(|r| r.random == Some(false)));
    // p = 0 is optimal play.
    let opt: Vec<_> = export_rows(&s, Source::Optimal, 4, 500).into_iter().map(|r| r.situation).collect();
    let p0: Vec<_> = export_rows(&s, Source::Perturbed(0.0), 4, 500).into_iter().map(|r| r.situation).collect();
    assert_eq!(opt, p0);
    assert_eq!(off_best(0.0), 0.0);
    // A random choice is sometimes a best option, so the share of suboptimal choices is a little below p.
    let share = off_best(0.3);
    assert!(share > 0.2 && share < 0.3, "{share}");
    // Perturbed play reaches lower scores on average.
    let final_scores = |src| {
        let rows = export_rows(&s, src, 4, 20_000);
        rows.iter().map(|r| r.best_value + f64::from(r.score_so_far.unwrap())).sum::<f64>() / rows.len() as f64
    };
    assert!(final_scores(Source::Perturbed(0.5)) < final_scores(Source::Optimal));
}

#[test]
fn uniform_samples_cover_the_reachable_states() {
    // Reduced check of the reachable set: upper totals are sums of face multiples.
    let v = Variant::scandinavian();
    let reach = ReachableStates::new(&v);
    let ones_twos = (1 << v.category_index("ones").unwrap()) | (1 << v.category_index("twos").unwrap());
    let mut uppers: Vec<u16> =
        (0..reach.len()).map(|i| reach.get(i)).filter(|s| s.filled == ones_twos).map(|s| s.upper).collect();
    uppers.sort();
    // k1 + 2 k2 for k1, k2 in 0..=5: 0 to 15.
    assert_eq!(uppers, (0..=15).collect::<Vec<u16>>());
    assert!(reach.get(0) == State::new());
    // Forced order: prefixes only.
    let f = ReachableStates::new(&Variant::by_id("yatzy-scandinavian+forced").unwrap());
    assert!((0..f.len()).all(|i| (f.get(i).filled + 1) & f.get(i).filled == 0));
    // American: both box values once the box is filled.
    let a = Variant::american();
    let ar = ReachableStates::new(&a);
    let y = 1 << a.category_index("five_of_a_kind").unwrap();
    let armed: Vec<bool> = (0..ar.len()).map(|i| ar.get(i)).filter(|s| s.filled == y).map(|s| s.bonus_armed).collect();
    assert_eq!(armed, [false, true]);
    // Samples: every state valid, score so far unknown, rerolls left spread.
    let s = Solver::build(&v);
    let rows = export_rows(&s, Source::Uniform, 2, 3000);
    check_rows(&s, &rows);
    assert!(rows.iter().all(|r| r.score_so_far.is_none() && r.chosen.is_none() && r.random.is_none()));
    for rl in 0..3 {
        assert!(rows.iter().filter(|r| r.situation.rolls_left == rl).count() > 800);
    }
    eprintln!("reachable non-final states: Scandinavian {}, American {}", reach.len(), ar.len());
}

#[test]
fn json_lines_schema() {
    let s = solver("american");
    let v = s.variant();
    let rows = export_rows(&s, Source::Perturbed(0.25), 1, 300);
    let mut buf = Vec::new();
    write_jsonl(&mut buf, &s, &rows).unwrap();
    let text = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), rows.len());
    let fields = [
        "variant",
        "solver_version",
        "rng",
        "precision",
        "source",
        "perturb",
        "seed",
        "game",
        "decision",
        "turn",
        "filled",
        "upper",
        "five_of_a_kind",
        "score_so_far",
        "dice",
        "rolls_left",
        "notation",
        "options",
        "best_value",
        "chosen",
        "random",
    ];
    for (line, r) in lines.iter().zip(&rows) {
        let j: serde_json::Value = serde_json::from_str(line).unwrap();
        let obj = j.as_object().unwrap();
        assert_eq!(
            obj.keys().map(String::as_str).collect::<std::collections::BTreeSet<_>>(),
            fields.into_iter().collect()
        );
        assert_eq!(j["variant"], "american");
        assert_eq!(j["rng"], 1);
        assert_eq!(j["precision"], "f64");
        assert_eq!(j["source"], "perturbed");
        assert_eq!(j["perturb"], 0.25);
        let sit = v.parse_situation(j["notation"].as_str().unwrap()).unwrap();
        assert_eq!(sit, r.situation);
        assert_eq!(j["turn"], r.situation.state.turns_played() + 1);
        assert_eq!(j["filled"], r.situation.state.filled);
        let dice: Vec<u8> = j["dice"].as_array().unwrap().iter().map(|d| d.as_u64().unwrap() as u8).collect();
        assert_eq!(dice, r.situation.dice.faces());
        let opts = j["options"].as_array().unwrap();
        assert_eq!(opts.len(), r.options.len());
        for (o, want) in opts.iter().zip(&r.options) {
            // Values read back to the same f64, bit for bit.
            assert_eq!(o["value"].as_f64().unwrap().to_bits(), want.value.to_bits());
            let a = v.action_from_code(o["code"].as_u64().unwrap() as u16).unwrap();
            assert_eq!(a, want.action);
            assert_eq!(o["notation"], v.format_action(&a));
            assert_eq!(o["type"], if matches!(a, Action::Keep(_)) { "keep" } else { "score" });
        }
        assert_eq!(j["best_value"].as_f64().unwrap(), r.best_value);
        assert_eq!(j["chosen"].as_u64().unwrap() as u16, v.action_code(&r.chosen.unwrap()).unwrap());
        assert_eq!(j["random"].as_bool(), r.random);
        match r.situation.state.filled & (1 << v.category_index("five_of_a_kind").unwrap()) {
            0 => assert!(j["five_of_a_kind"].is_null()),
            _ => assert!(j["five_of_a_kind"] == 0 || j["five_of_a_kind"] == 50),
        }
    }
}

#[cfg(feature = "parquet")]
#[test]
fn parquet_round_trip() {
    use parquet::file::reader::{FileReader, SerializedFileReader};
    let s = solver("yatzy-scandinavian");
    let rows = export_rows(&s, Source::Uniform, 3, 2500);
    let mut buf = Vec::new();
    yatzy_solver::export::write_parquet(&mut buf, &s, &rows).unwrap();
    let reader = SerializedFileReader::new(bytes::Bytes::from(buf)).unwrap();
    let meta = reader.metadata();
    assert_eq!(meta.file_metadata().num_rows(), 2500);
    let names: Vec<String> =
        meta.file_metadata().schema_descr().root_schema().get_fields().iter().map(|f| f.name().to_string()).collect();
    assert_eq!(names.len(), 21);
    assert_eq!(names[0], "variant");
    assert_eq!(names[19], "chosen");
    assert_eq!(names[20], "random");
    // Values survive: read the first row's best_value back.
    let mut it = reader.get_row_iter(None).unwrap();
    let first = it.next().unwrap().unwrap();
    let best = first.get_column_iter().find(|(n, _)| n.as_str() == "best_value").unwrap().1.clone();
    assert_eq!(best, parquet::record::Field::Double(rows[0].best_value));
}
