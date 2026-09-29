//! Export (F5): sampled situations with the exact value of every option, as JSON Lines (and Parquet with the
//! `parquet` feature). The schema is documented in `docs/export.md`.
//!
//! Three sources, all deterministic given the seed:
//! - **optimal**: every decision of games played optimally (game `g` is the same game as in the simulator);
//! - **perturbed**: every decision of games where each decision is uniformly random with probability `p`
//!   (drawn from the policy stream) and optimal otherwise, which reaches states a good player rarely sees;
//! - **uniform**: situations drawn uniformly from the reachable states, with a fair roll of the dice and the
//!   rerolls left uniform.

use std::fmt::Write as _;

use crate::query::{OptionValue, Solver};
use crate::rules::{Action, Game, Situation, State};
use crate::simulate::{RNG_VERSION, Rng};
use crate::table::{Precision, SOLVER_VERSION};
use crate::variant::Variant;

/// Where a row comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Source {
    Optimal,
    /// Each decision is uniformly random with this probability.
    Perturbed(f64),
    Uniform,
}

impl Source {
    pub fn name(&self) -> &'static str {
        match self {
            Source::Optimal => "optimal",
            Source::Perturbed(_) => "perturbed",
            Source::Uniform => "uniform",
        }
    }
}

/// One exported situation.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub source: Source,
    pub seed: u64,
    /// The game (trajectory sources) or sample (uniform) index.
    pub game: u64,
    /// The decision's index within its game (0 for uniform samples).
    pub decision: u32,
    pub situation: Situation,
    /// The score on the card, bonuses included; unknown (`None`) for uniform samples.
    pub score_so_far: Option<u16>,
    pub options: Vec<OptionValue>,
    pub best_value: f64,
    /// The action taken (trajectory sources); `None` for uniform samples.
    pub chosen: Option<Action>,
}

/// The stream of uniform sample `i` of a run with seed `seed` (generator version 1): seeded with
/// `mix(mix(seed ^ SAMPLE_DOMAIN) ^ i)`.
pub const SAMPLE_DOMAIN: u64 = 0x7361_6d70_6c65_0001;

fn sample_rng(seed: u64, i: u64) -> Rng {
    use crate::simulate::mix;
    Rng::new(mix(mix(seed ^ SAMPLE_DOMAIN) ^ i))
}

/// Plays game `game` under an optimal or perturbed policy and returns a row per decision. The dice come from
/// the same streams as the simulator's, so the optimal trajectory is the simulator's game.
pub fn trajectory(solver: &Solver, source: Source, seed: u64, game: u64) -> Vec<Row> {
    let v = solver.variant();
    let n = v.dice();
    let p = match source {
        Source::Perturbed(p) => p,
        _ => 0.0,
    };
    let mut policy_rng = Rng::for_policy(seed, game);
    let mut card = Game::new();
    let mut rows = Vec::new();
    while !v.is_over(card.state()) {
        let mut dice_rng = Rng::for_dice(seed, game, card.state().turns_played());
        let mut sit = v.start_turn(card.state(), dice_rng.roll_block(n, n)).expect("a fresh roll is legal");
        loop {
            let options = solver.option_values(&sit).expect("situations reached in play are legal");
            let best = options.iter().map(|o| o.value).fold(f64::NEG_INFINITY, f64::max);
            let action = if p > 0.0 && (policy_rng.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64) < p {
                options[policy_rng.below(options.len() as u64) as usize].action
            } else {
                solver.best_action(&sit).expect("legal")
            };
            rows.push(Row {
                source,
                seed,
                game,
                decision: rows.len() as u32,
                situation: sit,
                score_so_far: Some(card.total()),
                options,
                best_value: best,
                chosen: Some(action),
            });
            match action {
                Action::Keep(k) => {
                    let rolled = dice_rng.roll_block(n, n - k.len());
                    sit = v.apply_keep(&sit, &k, &rolled).expect("legal keep");
                }
                Action::Score(c) => {
                    card.score(v, &sit.dice, c).expect("legal category");
                    break;
                }
            }
        }
    }
    rows
}

/// The reachable states of a variant, for uniform sampling: every non-final filled mask (only prefixes under
/// forced order), every upper total that the filled upper boxes can add up to, and, when the five-of-a-kind
/// box is filled in a variant with its bonus, both box values.
pub struct ReachableStates {
    masks: Vec<u32>,
    /// Upper totals reachable for each mask, as a bitset over 0..=127.
    sums: Vec<u128>,
    /// Cumulative state counts, `cum[i]` = states of masks before `i`.
    cum: Vec<u64>,
    armed_choices: Vec<u8>,
}

impl ReachableStates {
    pub fn new(v: &Variant) -> ReachableStates {
        let n = v.dice() as u32;
        // Sums reachable from each set of filled upper faces.
        let upper_sums = |mask: u32| -> u128 {
            let mut set: u128 = 1;
            for f in 1..=6u8 {
                if let Some(c) = v.upper_of_face(f)
                    && mask & (1 << c) != 0
                {
                    let mut next = 0u128;
                    for k in 0..=n {
                        next |= set << (u32::from(f) * k);
                    }
                    set = next;
                }
            }
            set
        };
        let (mut masks, mut sums, mut cum, mut armed_choices) = (Vec::new(), Vec::new(), vec![0u64], Vec::new());
        for mask in 0..v.all_mask() {
            if v.forced_order() && (mask + 1) & mask != 0 {
                continue;
            }
            let s = upper_sums(mask);
            let armed = if v.all_same_bonus().is_some() && v.all_same_box().is_some_and(|y| mask & (1 << y) != 0) {
                2
            } else {
                1
            };
            masks.push(mask);
            sums.push(s);
            armed_choices.push(armed as u8);
            cum.push(cum.last().unwrap() + u64::from(s.count_ones()) * armed);
        }
        ReachableStates { masks, sums, cum, armed_choices }
    }

    /// The number of reachable non-final states.
    pub fn len(&self) -> u64 {
        *self.cum.last().unwrap()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The state with index `i` (0 <= i < len), in (mask, upper total, armed) order.
    pub fn get(&self, i: u64) -> State {
        let m = self.cum.partition_point(|&c| c <= i) - 1;
        let mut r = i - self.cum[m];
        let armed_n = u64::from(self.armed_choices[m]);
        let armed = r % armed_n == 1;
        r /= armed_n;
        let mut bits = self.sums[m];
        for _ in 0..r {
            bits &= bits - 1;
        }
        State { filled: self.masks[m], upper: bits.trailing_zeros() as u16, bonus_armed: armed }
    }
}

/// Uniform sample `i`: a reachable state chosen uniformly, a fair roll, and the rerolls left uniform.
pub fn uniform_sample(solver: &Solver, reachable: &ReachableStates, seed: u64, i: u64) -> Row {
    let v = solver.variant();
    let mut rng = sample_rng(seed, i);
    let state = reachable.get(rng.below(reachable.len()));
    let dice = rng.roll_block(v.dice(), v.dice());
    let rolls_left = rng.below(u64::from(v.rolls())) as u8;
    let situation = Situation { state, dice, rolls_left };
    let options = solver.option_values(&situation).expect("reachable states are legal");
    let best_value = options.iter().map(|o| o.value).fold(f64::NEG_INFINITY, f64::max);
    Row {
        source: Source::Uniform,
        seed,
        game: i,
        decision: 0,
        situation,
        score_so_far: None,
        options,
        best_value,
        chosen: None,
    }
}

/// Exactly `rows` rows from a source: whole trajectories from games 0, 1, ... (the last one cut to fit), or
/// uniform samples 0 to `rows - 1`. Computed in parallel with the `parallel` feature, always in the same order.
pub fn export_rows(solver: &Solver, source: Source, seed: u64, rows: usize) -> Vec<Row> {
    let mut out = Vec::with_capacity(rows);
    match source {
        Source::Uniform => {
            let reach = ReachableStates::new(solver.variant());
            let one = |i: u64| uniform_sample(solver, &reach, seed, i);
            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;
                out = (0..rows as u64).into_par_iter().map(one).collect();
            }
            #[cfg(not(feature = "parallel"))]
            out.extend((0..rows as u64).map(one));
        }
        _ => {
            // Games have about 30 to 45 decisions; play them in parallel batches until there are enough rows.
            let mut game = 0u64;
            while out.len() < rows {
                let need = rows - out.len();
                let batch = (need / 30 + 1) as u64;
                let games = game..game + batch;
                #[cfg(feature = "parallel")]
                let played: Vec<Vec<Row>> = {
                    use rayon::prelude::*;
                    games.into_par_iter().map(|g| trajectory(solver, source, seed, g)).collect()
                };
                #[cfg(not(feature = "parallel"))]
                let played: Vec<Vec<Row>> = games.map(|g| trajectory(solver, source, seed, g)).collect();
                for r in played.into_iter().flatten() {
                    if out.len() == rows {
                        break;
                    }
                    out.push(r);
                }
                game += batch;
            }
        }
    }
    out
}

/// The status of the five-of-a-kind box for the schema: `None` when the variant has no bonus or the box is
/// open, otherwise the points in it (0 or 50).
pub fn five_of_a_kind_box(v: &Variant, s: &State) -> Option<u16> {
    let y = v.all_same_box().filter(|&y| s.is_filled(y))?;
    v.all_same_bonus()?;
    Some(if s.bonus_armed {
        match v.categories()[y].kind {
            crate::variant::CategoryKind::AllSame { points } => points,
            _ => 0,
        }
    } else {
        0
    })
}

/// The export's metadata, the same for every row of one export.
#[derive(Clone, Debug)]
pub struct Meta {
    pub variant: String,
    pub solver_version: String,
    pub rng: u32,
    pub precision: &'static str,
}

impl Meta {
    pub fn of(solver: &Solver) -> Meta {
        Meta {
            variant: solver.variant().id().to_string(),
            solver_version: SOLVER_VERSION.to_string(),
            rng: RNG_VERSION,
            precision: match solver.precision() {
                Precision::F32 => "f32",
                Precision::F64 => "f64",
            },
        }
    }
}

/// One row as a JSON object on one line (no trailing newline). Floats use the shortest representation that
/// reads back to the same f64.
pub fn row_json(v: &Variant, meta: &Meta, row: &Row) -> String {
    let s = &row.situation;
    let mut o = String::with_capacity(4096);
    let opt_num = |x: Option<u16>| x.map_or("null".to_string(), |x| x.to_string());
    let dice: Vec<String> = s.dice.faces().iter().map(|f| f.to_string()).collect();
    let _ = write!(
        o,
        "{{\"variant\":\"{}\",\"solver_version\":\"{}\",\"rng\":{},\"precision\":\"{}\",\"source\":\"{}\",\
         \"perturb\":{},\"seed\":{},\"game\":{},\"decision\":{},\"turn\":{},\"filled\":{},\"upper\":{},\
         \"five_of_a_kind\":{},\"score_so_far\":{},\"dice\":[{}],\"rolls_left\":{},\"notation\":\"{}\",\"options\":[",
        meta.variant,
        meta.solver_version,
        meta.rng,
        meta.precision,
        row.source.name(),
        match row.source {
            Source::Perturbed(p) => format!("{p:?}"),
            _ => "null".to_string(),
        },
        row.seed,
        row.game,
        row.decision,
        s.state.turns_played() + 1,
        s.state.filled,
        s.state.upper,
        opt_num(five_of_a_kind_box(v, &s.state)),
        opt_num(row.score_so_far),
        dice.join(","),
        s.rolls_left,
        v.format_situation(s),
    );
    for (i, opt) in row.options.iter().enumerate() {
        let kind = match opt.action {
            Action::Keep(_) => "keep",
            Action::Score(_) => "score",
        };
        let _ = write!(
            o,
            "{}{{\"code\":{},\"type\":\"{kind}\",\"notation\":\"{}\",\"value\":{:?}}}",
            if i > 0 { "," } else { "" },
            v.action_code(&opt.action).expect("legal actions have codes"),
            v.format_action(&opt.action),
            opt.value
        );
    }
    let chosen = row.chosen.map_or("null".to_string(), |a| v.action_code(&a).expect("legal").to_string());
    let _ = write!(o, "],\"best_value\":{:?},\"chosen\":{chosen}}}", row.best_value);
    o
}

/// Writes rows as JSON Lines.
pub fn write_jsonl(w: &mut impl std::io::Write, solver: &Solver, rows: &[Row]) -> std::io::Result<()> {
    let meta = Meta::of(solver);
    for r in rows {
        writeln!(w, "{}", row_json(solver.variant(), &meta, r))?;
    }
    Ok(())
}

#[cfg(feature = "parquet")]
pub use parquet_out::{schema, write_parquet};

#[cfg(feature = "parquet")]
mod parquet_out {
    use std::sync::Arc;

    use arrow_array::builder::{Float64Builder, Int16Builder, ListBuilder, StringBuilder, StructBuilder, UInt8Builder};
    use arrow_array::{
        ArrayRef, Float64Array, Int16Array, RecordBatch, StringArray, UInt8Array, UInt16Array, UInt32Array, UInt64Array,
    };
    use arrow_schema::{DataType, Field, Fields, Schema};
    use parquet::arrow::ArrowWriter;
    use parquet::basic::Compression;
    use parquet::file::properties::WriterProperties;

    use super::{Meta, Row, Source, five_of_a_kind_box};
    use crate::query::Solver;
    use crate::rules::Action;

    fn option_fields() -> Fields {
        Fields::from(vec![
            Field::new("code", DataType::Int16, false),
            Field::new("type", DataType::Utf8, false),
            Field::new("notation", DataType::Utf8, false),
            Field::new("value", DataType::Float64, false),
        ])
    }

    /// The Arrow schema of the export (the same fields as the JSON Lines rows).
    pub fn schema() -> Schema {
        Schema::new(vec![
            Field::new("variant", DataType::Utf8, false),
            Field::new("solver_version", DataType::Utf8, false),
            Field::new("rng", DataType::UInt32, false),
            Field::new("precision", DataType::Utf8, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("perturb", DataType::Float64, true),
            Field::new("seed", DataType::UInt64, false),
            Field::new("game", DataType::UInt64, false),
            Field::new("decision", DataType::UInt32, false),
            Field::new("turn", DataType::UInt8, false),
            Field::new("filled", DataType::UInt32, false),
            Field::new("upper", DataType::UInt16, false),
            Field::new("five_of_a_kind", DataType::UInt16, true),
            Field::new("score_so_far", DataType::UInt16, true),
            Field::new("dice", DataType::List(Arc::new(Field::new("item", DataType::UInt8, true))), false),
            Field::new("rolls_left", DataType::UInt8, false),
            Field::new("notation", DataType::Utf8, false),
            Field::new(
                "options",
                DataType::List(Arc::new(Field::new("item", DataType::Struct(option_fields()), true))),
                false,
            ),
            Field::new("best_value", DataType::Float64, false),
            Field::new("chosen", DataType::Int16, true),
        ])
    }

    /// Writes rows as Parquet (zstd-compressed, one row group per 100 000 rows).
    pub fn write_parquet<W: std::io::Write + Send>(w: W, solver: &Solver, rows: &[Row]) -> parquet::errors::Result<()> {
        let v = solver.variant();
        let meta = Meta::of(solver);
        let schema = Arc::new(schema());
        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(Default::default()))
            .set_max_row_group_row_count(Some(100_000))
            .build();
        let mut writer = ArrowWriter::try_new(w, schema.clone(), Some(props))?;
        for chunk in rows.chunks(100_000) {
            let n = chunk.len();
            let strs = |s: &str| Arc::new(StringArray::from(vec![s; n])) as ArrayRef;
            let mut dice = ListBuilder::new(UInt8Builder::new());
            let mut options = ListBuilder::new(StructBuilder::from_fields(option_fields(), 0));
            for r in chunk {
                for f in r.situation.dice.faces() {
                    dice.values().append_value(f);
                }
                dice.append(true);
                let sb = options.values();
                for o in &r.options {
                    sb.field_builder::<Int16Builder>(0).unwrap().append_value(v.action_code(&o.action).unwrap() as i16);
                    sb.field_builder::<StringBuilder>(1).unwrap().append_value(match o.action {
                        Action::Keep(_) => "keep",
                        Action::Score(_) => "score",
                    });
                    sb.field_builder::<StringBuilder>(2).unwrap().append_value(v.format_action(&o.action));
                    sb.field_builder::<Float64Builder>(3).unwrap().append_value(o.value);
                    sb.append(true);
                }
                options.append(true);
            }
            let perturb: Vec<Option<f64>> = chunk
                .iter()
                .map(|r| match r.source {
                    Source::Perturbed(p) => Some(p),
                    _ => None,
                })
                .collect();
            let cols: Vec<ArrayRef> = vec![
                strs(&meta.variant),
                strs(&meta.solver_version),
                Arc::new(UInt32Array::from(vec![meta.rng; n])),
                strs(meta.precision),
                Arc::new(StringArray::from(chunk.iter().map(|r| r.source.name()).collect::<Vec<_>>())),
                Arc::new(Float64Array::from(perturb)),
                Arc::new(UInt64Array::from(chunk.iter().map(|r| r.seed).collect::<Vec<_>>())),
                Arc::new(UInt64Array::from(chunk.iter().map(|r| r.game).collect::<Vec<_>>())),
                Arc::new(UInt32Array::from(chunk.iter().map(|r| r.decision).collect::<Vec<_>>())),
                Arc::new(UInt8Array::from(
                    chunk.iter().map(|r| r.situation.state.turns_played() as u8 + 1).collect::<Vec<_>>(),
                )),
                Arc::new(UInt32Array::from(chunk.iter().map(|r| r.situation.state.filled).collect::<Vec<_>>())),
                Arc::new(UInt16Array::from(chunk.iter().map(|r| r.situation.state.upper).collect::<Vec<_>>())),
                Arc::new(UInt16Array::from(
                    chunk.iter().map(|r| five_of_a_kind_box(v, &r.situation.state)).collect::<Vec<_>>(),
                )),
                Arc::new(UInt16Array::from(chunk.iter().map(|r| r.score_so_far).collect::<Vec<_>>())),
                Arc::new(dice.finish()),
                Arc::new(UInt8Array::from(chunk.iter().map(|r| r.situation.rolls_left).collect::<Vec<_>>())),
                Arc::new(StringArray::from(chunk.iter().map(|r| v.format_situation(&r.situation)).collect::<Vec<_>>())),
                Arc::new(options.finish()),
                Arc::new(Float64Array::from(chunk.iter().map(|r| r.best_value).collect::<Vec<_>>())),
                Arc::new(Int16Array::from(
                    chunk.iter().map(|r| r.chosen.map(|a| v.action_code(&a).unwrap() as i16)).collect::<Vec<_>>(),
                )),
            ];
            let batch = RecordBatch::try_new(schema.clone(), cols)
                .map_err(|e| parquet::errors::ParquetError::General(e.to_string()))?;
            writer.write(&batch)?;
        }
        writer.close()?;
        Ok(())
    }
}
