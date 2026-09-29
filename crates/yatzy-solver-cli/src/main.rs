//! `yatzy-solver build | query | verify`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use num_rational::BigRational;
use yatzy_solver::table::hex;
use yatzy_solver::verify::{BruteForce, PUBLISHED, reduced_variants};
use yatzy_solver::{Precision, State, Table, TurnModel, Variant};

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

#[derive(Subcommand)]
enum Command {
    /// Solve a variant and write its table.
    Build {
        /// Variant id, e.g. yatzy-scandinavian or yahtzee.
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
    /// Check the solver: the brute-force cross-check on reduced games, then the published values.
    Verify {
        /// Skip solving the full variants.
        #[arg(long)]
        quick: bool,
    },
}

fn variant(id: &str) -> Result<Variant, String> {
    Variant::by_id(id).ok_or_else(|| format!("unknown variant {id:?}"))
}

fn default_path(id: &str, p: Precision) -> PathBuf {
    let ext = if p == Precision::F32 { "f32" } else { "f64" };
    PathBuf::from("tables").join(format!("{id}.{ext}.yzt"))
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
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let table = Table::from_bytes(&bytes, &v).map_err(|e| e.to_string())?;
            let sit = v.parse_situation(&situation).map_err(|e| e.to_string())?;
            let model = TurnModel::new(&v);
            let mut vals = model.action_values(&sit, table.values());
            vals.sort_by(|a, b| b.1.total_cmp(&a.1));
            let best = vals[0].1;
            println!("{}", v.format_situation(&sit));
            for (a, x) in vals {
                println!("{:>12.6} {:>10.6}  {}", x, best - x, v.format_action(&a));
            }
            Ok(true)
        }
        Command::Verify { quick } => {
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
                    let v = variant(p.variant)?;
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
            println!("{}", if ok { "all checks passed" } else { "CHECKS FAILED" });
            Ok(ok)
        }
    }
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
