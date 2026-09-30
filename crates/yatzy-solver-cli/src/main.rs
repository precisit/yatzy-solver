//! `yatzy-solver build | query | simulate | export | verify`.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use num_rational::BigRational;
use yatzy_solver::export::{Source, export_rows, write_jsonl, write_parquet};
use yatzy_solver::simulate::{RNG_VERSION, RandomPolicy, simulate};
use yatzy_solver::table::hex;
use yatzy_solver::verify::{BruteForce, PUBLISHED, reduced_variants};
use yatzy_solver::{Action, Precision, Solver, State, Table, TurnModel, Variant};

#[derive(Parser)]
#[command(name = "yatzy-solver", version, about = "Exact solver for Scandinavian Yatzy and American rules")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum Prec {
    F32,
    F64,
}

#[derive(Clone, Copy, ValueEnum)]
enum SourceArg {
    Optimal,
    Perturbed,
    Uniform,
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    Jsonl,
    Parquet,
}

#[derive(Clone, Copy, ValueEnum)]
enum PolicyArg {
    Optimal,
    Random,
}

#[derive(Subcommand)]
enum Command {
    /// Solve a variant and write its table.
    Build {
        /// Variant id, e.g. yatzy-scandinavian or american.
        #[arg(long, default_value = Variant::SCANDINAVIAN)]
        variant: String,
        #[arg(long, value_enum, default_value = "f32")]
        precision: Prec,
        /// Output file; default tables/<variant>.<precision>.yzt
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Show the value of every legal action in a situation.
    Query {
        #[arg(long, default_value = Variant::SCANDINAVIAN)]
        variant: String,
        /// Table file; default tables/<variant>.f32.yzt
        #[arg(long)]
        table: Option<PathBuf>,
        /// A situation in the stable notation, e.g. "dice 1 3 3 5 6 | rolls 2 | upper 0 | filled -".
        situation: String,
    },
    /// Play games with a seeded generator and summarize the scores.
    Simulate {
        #[arg(long, default_value = Variant::SCANDINAVIAN)]
        variant: String,
        /// Table file; by default the variant is solved in memory.
        #[arg(long)]
        table: Option<PathBuf>,
        #[arg(long, default_value_t = 100_000)]
        games: u64,
        #[arg(long, default_value_t = 0)]
        seed: u64,
        #[arg(long, value_enum, default_value = "optimal")]
        policy: PolicyArg,
        /// Write one JSON object per game (decisions in the stable notation) to this file.
        #[arg(long)]
        log: Option<PathBuf>,
    },
    /// Export sampled situations with the value of every option (docs/export.md).
    Export {
        #[arg(long, default_value = Variant::SCANDINAVIAN)]
        variant: String,
        /// Table file; by default the variant is solved in memory (f64).
        #[arg(long)]
        table: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "optimal")]
        source: SourceArg,
        /// Share of random decisions for the perturbed source.
        #[arg(long, default_value_t = 0.1)]
        perturb: f64,
        #[arg(long, default_value_t = 100_000)]
        rows: usize,
        #[arg(long, default_value_t = 0)]
        seed: u64,
        #[arg(long, value_enum, default_value = "jsonl")]
        format: FormatArg,
        #[arg(long)]
        out: PathBuf,
    },
    /// Check the solver: the brute-force cross-check on reduced games, then the published values.
    Verify {
        /// Skip solving the full variants.
        #[arg(long)]
        quick: bool,
        /// Also check a golden set (golden/parity.jsonl): every option value, bit for bit.
        #[arg(long)]
        golden: Option<PathBuf>,
    },
}

fn variant(id: &str) -> Result<Variant, String> {
    Variant::by_id(id).ok_or_else(|| format!("unknown variant {id:?}"))
}

fn default_path(id: &str, p: Precision) -> PathBuf {
    let ext = if p == Precision::F32 { "f32" } else { "f64" };
    PathBuf::from("tables").join(format!("{id}.{ext}.yzt"))
}

/// A solver from a table file, or solved in memory when no file is given.
fn solver(v: &Variant, table: Option<PathBuf>) -> Result<Solver, String> {
    match table {
        Some(path) => {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(Solver::from_table(&Table::from_bytes(&bytes, v).map_err(|e| e.to_string())?))
        }
        None => Ok(Solver::build(v)),
    }
}

fn run(cli: Cli) -> Result<bool, String> {
    match cli.command {
        Command::Build { variant: id, precision, out } => {
            let v = variant(&id)?;
            let p = match precision {
                Prec::F32 => Precision::F32,
                Prec::F64 => Precision::F64,
            };
            let t0 = Instant::now();
            let table = Table::build(&v, p);
            let secs = t0.elapsed().as_secs_f64();
            let path = out.unwrap_or_else(|| default_path(&id, p));
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            let bytes = table.to_bytes();
            std::fs::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            let start = table.values()[yatzy_solver::StateSpace::of(&v).index(&State::new())];
            println!("variant      {id}");
            println!("expected     {start:.10}");
            println!("states       {}", table.values().len());
            println!("solve time   {secs:.2} s");
            println!("file         {} ({} bytes)", path.display(), bytes.len());
            println!("file sha256  {}", hex(&bytes[bytes.len() - 32..]));
            println!("values hash  {}", hex(&table.values_hash()));
            Ok(true)
        }
        Command::Query { variant: id, table, situation } => {
            let v = variant(&id)?;
            let path = table.unwrap_or_else(|| default_path(&id, Precision::F32));
            let solver = solver(&v, Some(path))?;
            let sit = v.parse_situation(&situation).map_err(|e| e.to_string())?;
            let mut options = solver.option_values(&sit).map_err(|e| e.to_string())?;
            let best: Vec<Action> =
                solver.best_options(&sit).map_err(|e| e.to_string())?.iter().map(|o| o.action).collect();
            options.sort_by(|a, b| b.value.total_cmp(&a.value));
            // Stop quietly when the reader closes the pipe (for example `| head`).
            let mut out = std::io::stdout().lock();
            let mut lines = vec![v.format_situation(&sit), format!("{:>12} {:>10}  option", "value", "regret")];
            for o in options {
                let regret = solver.regret(&sit, &o.action).map_err(|e| e.to_string())?;
                let mark = if best.contains(&o.action) { "  (best)" } else { "" };
                lines.push(format!("{:>12.6} {:>10.6}  {}{mark}", o.value, regret, v.format_action(&o.action)));
            }
            for line in lines {
                if writeln!(out, "{line}").is_err() {
                    break;
                }
            }
            Ok(true)
        }
        Command::Simulate { variant: id, table, games, seed, policy, log } => {
            let v = variant(&id)?;
            let solver = solver(&v, table)?;
            let t0 = Instant::now();
            let logs = log.is_some();
            let sim = match policy {
                PolicyArg::Optimal => solver.simulate_optimal(games, seed, logs),
                PolicyArg::Random => simulate(&v, &mut RandomPolicy, games, seed, logs).map_err(|e| e.to_string())?,
            };
            let secs = t0.elapsed().as_secs_f64();
            let sum = sim.summary();
            let exact = solver.state_value(&State::new());
            println!("variant      {id}");
            println!(
                "games        {games} (seed {seed}, policy {})",
                if matches!(policy, PolicyArg::Optimal) { "optimal" } else { "random" }
            );
            println!(
                "mean         {:.4} +- {:.4} (95% interval {:.4} to {:.4})",
                sum.mean,
                sum.std_error,
                sum.mean - 1.96 * sum.std_error,
                sum.mean + 1.96 * sum.std_error
            );
            println!("exact mean   {exact:.4} (optimal policy)");
            println!("std dev      {:.4}", sum.std_dev);
            println!("min / median / max   {} / {} / {}", sum.min, sum.median, sum.max);
            println!(
                "percentiles  10%: {}  25%: {}  75%: {}  90%: {}",
                sim.percentile(10.0),
                sim.percentile(25.0),
                sim.percentile(75.0),
                sim.percentile(90.0)
            );
            println!("time         {secs:.2} s");
            if let Some(path) = log {
                let mut out = String::new();
                for g in &sim.logs {
                    let decisions: Vec<String> = g
                        .decisions
                        .iter()
                        .map(|d| {
                            let points = d.scored.map_or("null".to_string(), |x| x.total().to_string());
                            format!(
                                "{{\"situation\":\"{}\",\"action\":\"{}\",\"points\":{points}}}",
                                v.format_situation(&d.situation),
                                v.format_action(&d.action)
                            )
                        })
                        .collect();
                    out.push_str(&format!(
                        "{{\"variant\":\"{id}\",\"rng\":{RNG_VERSION},\"seed\":{seed},\"game\":{},\"score\":{},\"decisions\":[{}]}}\n",
                        g.game,
                        g.final_score,
                        decisions.join(",")
                    ));
                }
                std::fs::write(&path, out).map_err(|e| format!("{}: {e}", path.display()))?;
                println!("log          {}", path.display());
            }
            Ok(true)
        }
        Command::Export { variant: id, table, source, perturb, rows, seed, format, out } => {
            let v = variant(&id)?;
            let solver = solver(&v, table)?;
            let src = match source {
                SourceArg::Optimal => Source::Optimal,
                SourceArg::Perturbed if (0.0..=1.0).contains(&perturb) => Source::Perturbed(perturb),
                SourceArg::Perturbed => return Err("--perturb must be between 0 and 1".into()),
                SourceArg::Uniform => Source::Uniform,
            };
            let t0 = Instant::now();
            let data = export_rows(&solver, src, seed, rows);
            let file = std::fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
            let mut w = std::io::BufWriter::new(file);
            match format {
                FormatArg::Jsonl => write_jsonl(&mut w, &solver, &data).map_err(|e| e.to_string())?,
                FormatArg::Parquet => write_parquet(w, &solver, &data).map_err(|e| e.to_string())?,
            }
            let bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
            println!(
                "{} rows ({}, seed {seed}) to {} ({bytes} bytes) in {:.2} s",
                data.len(),
                src.name(),
                out.display(),
                t0.elapsed().as_secs_f64()
            );
            Ok(true)
        }
        Command::Verify { quick, golden } => {
            let mut ok = true;
            println!("brute-force cross-check (exact rational arithmetic):");
            for v in reduced_variants() {
                let model = TurnModel::new(&v);
                let exact: Vec<BigRational> = model.solve();
                let mut bf = BruteForce::<BigRational>::new(&v);
                bf.state_value(&State::new());
                let (mut n, mut bad) = (0, 0);
                for (s, want) in bf.states() {
                    n += 1;
                    if &exact[model.space().index(s)] != want {
                        bad += 1;
                    }
                }
                ok &= bad == 0;
                println!("  {:28} {n:5} states, {bad} mismatches", v.id());
            }
            if !quick {
                println!("published values:");
                for p in PUBLISHED {
                    let v = p.variant();
                    let model = TurnModel::new(&v);
                    let t: Vec<f64> = model.solve();
                    let x = t[model.space().index(&State::new())];
                    let m = p.matches(x);
                    ok &= m;
                    let d = p.decimals as usize;
                    println!(
                        "  {:28} {x:.10}  published {:.d$}  {}  ({})",
                        p.variant,
                        p.expected,
                        if m { "ok" } else { "MISMATCH" },
                        p.source
                    );
                }
            }
            if let Some(path) = golden {
                let (n, bad) = check_golden(&path)?;
                ok &= bad == 0 && n > 0;
                println!("golden set {}: {n} situations, {bad} mismatches", path.display());
            }
            println!("{}", if ok { "all checks passed" } else { "CHECKS FAILED" });
            Ok(ok)
        }
    }
}

/// Checks every situation of a golden file (JSON Lines: variant, situation, state_value, options as
/// [code, value] pairs) against fresh solves, bit for bit. Returns (situations, mismatches).
fn check_golden(path: &PathBuf) -> Result<(usize, usize), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut solvers: std::collections::HashMap<String, Solver> = std::collections::HashMap::new();
    let (mut n, mut bad) = (0, 0);
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let row: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        let id = row["variant"].as_str().ok_or("golden row without variant")?.to_string();
        let solver = match solvers.get(&id) {
            Some(s) => s,
            None => {
                let s = Solver::build(&variant(&id)?);
                solvers.entry(id.clone()).or_insert(s)
            }
        };
        let v = solver.variant();
        let sit = v
            .parse_situation(row["situation"].as_str().ok_or("golden row without situation")?)
            .map_err(|e| e.to_string())?;
        let got = solver.option_values(&sit).map_err(|e| e.to_string())?;
        let want = row["options"].as_array().ok_or("golden row without options")?;
        let same = got.len() == want.len()
            && got.iter().zip(want).all(|(o, w)| {
                w[0].as_u64() == v.action_code(&o.action).map(u64::from)
                    && w[1].as_f64().map(f64::to_bits) == Some(o.value.to_bits())
            })
            && row["state_value"].as_f64().map(f64::to_bits) == Some(solver.state_value(&sit.state).to_bits());
        n += 1;
        bad += usize::from(!same);
    }
    Ok((n, bad))
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}
