//! Python bindings. The Python package `yatzy_solver` re-exports these classes; see its docstrings.

use std::path::PathBuf;
use std::sync::Arc;

use numpy::{
    IntoPyArray, PyArray1, PyArray2, PyArrayMethods, PyReadonlyArray1, PyReadonlyArray2, PyUntypedArrayMethods,
};
use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use yatzy_solver::export::{Source, export_rows, write_jsonl, write_parquet};
use yatzy_solver::simulate::{GameLog, Policy, RNG_VERSION, Rng, Simulation, play_game};
use yatzy_solver::{Action, Dice, Game, Precision, Situation, Solver, State, Table, Variant};

fn err(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn dice_of(faces: &[u8]) -> PyResult<Dice> {
    Dice::from_faces(faces).map_err(err)
}

/// A rule set: dice, categories, scoring and bonuses. `Variant("yatzy-scandinavian")`,
/// `Variant("american")`; `builtin_variants()` lists the ids.
#[pyclass(name = "Variant", module = "yatzy_solver", frozen, from_py_object)]
#[derive(Clone)]
struct PyVariant {
    v: Variant,
}

impl PyVariant {
    fn category(&self, id: &str) -> PyResult<usize> {
        self.v.category_index(id).ok_or_else(|| err(format!("no category {id:?} in {}", self.v.id())))
    }

    fn situation(&self, text: &str) -> PyResult<Situation> {
        self.v.parse_situation(text).map_err(err)
    }

    fn state(&self, text: &str) -> PyResult<State> {
        self.v.parse_state(text).map_err(err)
    }
}

#[pymethods]
impl PyVariant {
    #[new]
    fn new(id: &str) -> PyResult<Self> {
        Variant::by_id(id).map(|v| PyVariant { v }).ok_or_else(|| err(format!("unknown variant {id:?}")))
    }

    #[getter]
    fn id(&self) -> String {
        self.v.id().to_string()
    }

    #[getter]
    fn name(&self) -> String {
        self.v.name().to_string()
    }

    /// Number of dice.
    #[getter]
    fn dice(&self) -> usize {
        self.v.dice()
    }

    /// Rolls per turn, including the first.
    #[getter]
    fn rolls(&self) -> u8 {
        self.v.rolls()
    }

    /// Category ids in score-card order.
    #[getter]
    fn categories(&self) -> Vec<String> {
        self.v.categories().iter().map(|c| c.id.clone()).collect()
    }

    /// Number of action codes (categories + keeps that can be legal).
    #[getter]
    fn num_action_codes(&self) -> usize {
        self.v.num_action_codes()
    }

    /// The largest number of legal actions in a situation: the width of the flat batch layout.
    #[getter]
    fn max_options(&self) -> usize {
        self.v.max_options()
    }

    /// The code of an action given in notation, e.g. `"keep 6 6"`.
    fn action_code(&self, action: &str) -> PyResult<u16> {
        let a = self.v.parse_action(action).map_err(err)?;
        self.v.action_code(&a).ok_or_else(|| err("no code for this action"))
    }

    /// The notation of the action with a code.
    fn action_notation(&self, code: u16) -> PyResult<String> {
        self.v.action_from_code(code).map(|a| self.v.format_action(&a)).ok_or_else(|| err("no such code"))
    }

    /// The score of dice in a category under the normal rules.
    fn score(&self, category: &str, dice: Vec<u8>) -> PyResult<u16> {
        Ok(self.v.score(self.category(category)?, &dice_of(&dice)?))
    }

    /// The situation after the first roll of a turn in `state`.
    fn start_turn(&self, state: &str, dice: Vec<u8>) -> PyResult<String> {
        let sit = self.v.start_turn(&self.state(state)?, dice_of(&dice)?).map_err(err)?;
        Ok(self.v.format_situation(&sit))
    }

    /// Every legal action in a situation, in legal-action order.
    fn legal_actions(&self, situation: &str) -> PyResult<Vec<String>> {
        let sit = self.situation(situation)?;
        Ok(self.v.legal_actions(&sit).map_err(err)?.iter().map(|a| self.v.format_action(a)).collect())
    }

    /// The situation after keeping `keep` and rerolling the rest, which came up `rolled`.
    fn apply_keep(&self, situation: &str, keep: Vec<u8>, rolled: Vec<u8>) -> PyResult<String> {
        let sit = self.situation(situation)?;
        let next = self.v.apply_keep(&sit, &dice_of(&keep)?, &dice_of(&rolled)?).map_err(err)?;
        Ok(self.v.format_situation(&next))
    }

    /// Scores dice in a category: returns the next state and the points earned (bonuses included).
    fn apply_score(&self, state: &str, dice: Vec<u8>, category: &str) -> PyResult<(String, u16)> {
        let (next, scored) =
            self.v.apply_score(&self.state(state)?, &dice_of(&dice)?, self.category(category)?).map_err(err)?;
        Ok((self.v.format_state(&next), scored.total()))
    }

    /// True when every category of the state is filled.
    fn is_over(&self, state: &str) -> PyResult<bool> {
        Ok(self.v.is_over(&self.state(state)?))
    }

    /// Situations in notation as arrays: `filled` (uint32), `upper` (uint16), `armed` (bool), `dice`
    /// (uint8, n x dice) and `rolls_left` (uint8), the inputs of the batch functions.
    fn situation_arrays<'py>(&self, py: Python<'py>, situations: Vec<String>) -> PyResult<Bound<'py, PyDict>> {
        let n = situations.len();
        let (mut filled, mut upper, mut armed, mut dice, mut rolls) =
            (Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n), Vec::new(), Vec::with_capacity(n));
        for s in &situations {
            let sit = self.situation(s)?;
            filled.push(sit.state.filled);
            upper.push(sit.state.upper);
            armed.push(sit.state.bonus_armed);
            dice.extend(sit.dice.faces());
            rolls.push(sit.rolls_left);
        }
        let d = PyDict::new(py);
        d.set_item("filled", filled.into_pyarray(py))?;
        d.set_item("upper", upper.into_pyarray(py))?;
        d.set_item("armed", armed.into_pyarray(py))?;
        d.set_item("dice", dice.into_pyarray(py).reshape([n, self.v.dice()])?)?;
        d.set_item("rolls_left", rolls.into_pyarray(py))?;
        Ok(d)
    }

    /// Scatters the flat batch layout into the dense one: `n x num_action_codes` values, NaN where an action
    /// is not legal.
    fn dense<'py>(
        &self,
        py: Python<'py>,
        codes: PyReadonlyArray2<'py, i16>,
        values: PyReadonlyArray2<'py, f64>,
        counts: PyReadonlyArray1<'py, u16>,
    ) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let (n, w) = (codes.shape()[0], codes.shape()[1]);
        if values.shape() != codes.shape() || counts.len() != n {
            return Err(err("codes, values and counts must have matching shapes"));
        }
        let (codes, values, counts) = (codes.as_slice()?, values.as_slice()?, counts.as_slice()?);
        let k = self.v.num_action_codes();
        let mut out = vec![f64::NAN; n * k];
        for r in 0..n {
            for j in 0..usize::from(counts[r]).min(w) {
                let c = codes[r * w + j];
                if c < 0 || c as usize >= k {
                    return Err(err(format!("bad action code {c}")));
                }
                out[r * k + c as usize] = values[r * w + j];
            }
        }
        out.into_pyarray(py).reshape([n, k])
    }

    fn __repr__(&self) -> String {
        format!("Variant({:?})", self.v.id())
    }
}

/// A full score card, for playing a game with the rules engine.
#[pyclass(name = "Game", module = "yatzy_solver")]
struct PyGame {
    v: Variant,
    g: Game,
}

#[pymethods]
impl PyGame {
    #[new]
    fn new(variant: &PyVariant) -> Self {
        PyGame { v: variant.v.clone(), g: Game::new() }
    }

    /// The decision-relevant state in notation.
    #[getter]
    fn state(&self) -> String {
        self.v.format_state(self.g.state())
    }

    /// The score so far, bonuses included (the final score once the game is over).
    #[getter]
    fn total(&self) -> u16 {
        self.g.total()
    }

    #[getter]
    fn upper_bonus(&self) -> u16 {
        self.g.upper_bonus()
    }

    /// Five-of-a-kind bonus points earned so far (American rules).
    #[getter]
    fn bonus(&self) -> u16 {
        self.g.all_same_bonus()
    }

    #[getter]
    fn is_over(&self) -> bool {
        self.v.is_over(self.g.state())
    }

    /// The points in a category, or None when it is open.
    fn points(&self, category: &str) -> PyResult<Option<u16>> {
        let c = self.v.category_index(category).ok_or_else(|| err(format!("no category {category:?}")))?;
        Ok(self.g.points(c))
    }

    /// Scores dice in a category; returns the points earned by kind.
    fn score<'py>(&mut self, py: Python<'py>, dice: Vec<u8>, category: &str) -> PyResult<Bound<'py, PyDict>> {
        let c = self.v.category_index(category).ok_or_else(|| err(format!("no category {category:?}")))?;
        let s = self.g.score(&self.v, &dice_of(&dice)?, c).map_err(err)?;
        let d = PyDict::new(py);
        d.set_item("points", s.points)?;
        d.set_item("upper_bonus", s.upper_bonus)?;
        d.set_item("bonus", s.all_same_bonus)?;
        d.set_item("total", s.total())?;
        Ok(d)
    }
}

/// A solved variant: exact values for every state and option.
#[pyclass(name = "Solver", module = "yatzy_solver", frozen)]
struct PySolver {
    s: Arc<Solver>,
}

/// A Python callable as a simulation policy: `policy(situation, legal_actions, score_so_far) -> action`.
struct PyPolicy<'py> {
    f: Bound<'py, PyAny>,
    error: Option<PyErr>,
}

impl Policy for PyPolicy<'_> {
    fn choose(&mut self, v: &Variant, game: &Game, sit: &Situation, legal: &[Action], _: &mut Rng) -> Action {
        if self.error.is_some() {
            return legal[0];
        }
        let names: Vec<String> = legal.iter().map(|a| v.format_action(a)).collect();
        let result = self
            .f
            .call1((v.format_situation(sit), names, game.total()))
            .and_then(|r| r.extract::<String>())
            .and_then(|s| v.parse_action(&s).map_err(err));
        match result {
            Ok(a) => a,
            Err(e) => {
                self.error = Some(e);
                legal[0]
            }
        }
    }
}

fn simulation_dict<'py>(py: Python<'py>, v: &Variant, sim: Simulation, logs: bool) -> PyResult<Bound<'py, PyDict>> {
    let sum = sim.summary();
    let d = PyDict::new(py);
    d.set_item("mean", sum.mean)?;
    d.set_item("std_dev", sum.std_dev)?;
    d.set_item("std_error", sum.std_error)?;
    d.set_item("min", sum.min)?;
    d.set_item("median", sum.median)?;
    d.set_item("max", sum.max)?;
    d.set_item("rng", RNG_VERSION)?;
    let lines: Option<Vec<Vec<String>>> = logs.then(|| sim.logs.iter().map(|l: &GameLog| l.to_lines(v)).collect());
    d.set_item("logs", lines)?;
    d.set_item("scores", sim.scores.into_pyarray(py))?;
    Ok(d)
}

#[pymethods]
impl PySolver {
    /// Solves a variant in memory (f64 values; a few seconds on all cores).
    #[staticmethod]
    fn build(py: Python<'_>, variant: &PyVariant) -> Self {
        let v = variant.v.clone();
        PySolver { s: Arc::new(py.detach(|| Solver::build(&v))) }
    }

    /// Loads a table file written by `yatzy-solver build` for this variant.
    #[staticmethod]
    fn load(py: Python<'_>, path: PathBuf, variant: &PyVariant) -> PyResult<Self> {
        let bytes = std::fs::read(&path).map_err(|e| PyIOError::new_err(format!("{}: {e}", path.display())))?;
        let v = variant.v.clone();
        let table = py.detach(|| Table::from_bytes(&bytes, &v)).map_err(err)?;
        Ok(PySolver { s: Arc::new(Solver::from_table(&table)) })
    }

    /// Solves a variant and writes its table file.
    #[staticmethod]
    #[pyo3(signature = (variant, path, precision = "f32"))]
    fn build_table(py: Python<'_>, variant: &PyVariant, path: PathBuf, precision: &str) -> PyResult<()> {
        let p = match precision {
            "f32" => Precision::F32,
            "f64" => Precision::F64,
            _ => return Err(err("precision is f32 or f64")),
        };
        let v = variant.v.clone();
        let bytes = py.detach(|| Table::build(&v, p).to_bytes());
        std::fs::write(&path, bytes).map_err(|e| PyIOError::new_err(format!("{}: {e}", path.display())))
    }

    #[getter]
    fn variant(&self) -> PyVariant {
        PyVariant { v: self.s.variant().clone() }
    }

    /// `"f32"` or `"f64"`: the precision of the values.
    #[getter]
    fn precision(&self) -> &'static str {
        match self.s.precision() {
            Precision::F32 => "f32",
            Precision::F64 => "f64",
        }
    }

    /// Options closer than this are tied.
    #[getter]
    fn tie_epsilon(&self) -> f64 {
        self.s.tie_epsilon()
    }

    /// The expected remaining score from the start of a turn in a state (notation, e.g. `"upper 0 | filled -"`).
    fn state_value(&self, state: &str) -> PyResult<f64> {
        Ok(self.s.state_value(&self.s.variant().parse_state(state).map_err(err)?))
    }

    /// Every legal option of a situation with its value, as (action notation, value) pairs.
    fn option_values(&self, situation: &str) -> PyResult<Vec<(String, f64)>> {
        let v = self.s.variant();
        let sit = v.parse_situation(situation).map_err(err)?;
        Ok(self.s.option_values(&sit).map_err(err)?.iter().map(|o| (v.format_action(&o.action), o.value)).collect())
    }

    /// The best options (ties included), as (action notation, value) pairs.
    fn best_options(&self, situation: &str) -> PyResult<Vec<(String, f64)>> {
        let v = self.s.variant();
        let sit = v.parse_situation(situation).map_err(err)?;
        Ok(self.s.best_options(&sit).map_err(err)?.iter().map(|o| (v.format_action(&o.action), o.value)).collect())
    }

    /// The first best action (a reproducibility rule for ties, not a preference).
    fn best_action(&self, situation: &str) -> PyResult<String> {
        let v = self.s.variant();
        let sit = v.parse_situation(situation).map_err(err)?;
        Ok(v.format_action(&self.s.best_action(&sit).map_err(err)?))
    }

    /// The expected points lost by an action against the best option.
    fn regret(&self, situation: &str, action: &str) -> PyResult<f64> {
        let v = self.s.variant();
        let sit = v.parse_situation(situation).map_err(err)?;
        let a = v.parse_action(action).map_err(err)?;
        self.s.regret(&sit, &a).map_err(err)
    }

    /// Batch state values: `filled` (uint32), `upper` (uint16), `armed` (bool) -> float64 values.
    fn state_values<'py>(
        &self,
        py: Python<'py>,
        filled: PyReadonlyArray1<'py, u32>,
        upper: PyReadonlyArray1<'py, u16>,
        armed: PyReadonlyArray1<'py, bool>,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let states = states_of(self.s.variant(), filled.as_slice()?, upper.as_slice()?, armed.as_slice()?)?;
        let s = self.s.clone();
        Ok(py.detach(move || s.state_values(&states)).into_pyarray(py))
    }

    /// Batch option values in the flat layout: returns `(codes, values, counts)`, `codes` int16 and `values`
    /// float64 of shape `n x max_options` (padding -1 and NaN), `counts` uint16. Inputs as from
    /// `Variant.situation_arrays`.
    fn option_values_batch<'py>(
        &self,
        py: Python<'py>,
        filled: PyReadonlyArray1<'py, u32>,
        upper: PyReadonlyArray1<'py, u16>,
        armed: PyReadonlyArray1<'py, bool>,
        dice: PyReadonlyArray2<'py, u8>,
        rolls_left: PyReadonlyArray1<'py, u8>,
    ) -> PyResult<FlatArrays<'py>> {
        let v = self.s.variant();
        let states = states_of(v, filled.as_slice()?, upper.as_slice()?, armed.as_slice()?)?;
        let n = states.len();
        if dice.shape() != [n, v.dice()] || rolls_left.len() != n {
            return Err(err(format!("dice must be n x {} and rolls_left n", v.dice())));
        }
        let (dice, rolls) = (dice.as_slice()?, rolls_left.as_slice()?);
        let sits: Vec<Situation> = (0..n)
            .map(|i| {
                Ok(Situation {
                    state: states[i],
                    dice: dice_of(&dice[i * v.dice()..(i + 1) * v.dice()])?,
                    rolls_left: rolls[i],
                })
            })
            .collect::<PyResult<_>>()?;
        let s = self.s.clone();
        let flat = py.detach(move || s.option_values_flat(&sits)).map_err(|(i, e)| err(format!("row {i}: {e}")))?;
        let w = flat.width;
        Ok((
            flat.codes.into_pyarray(py).reshape([n, w])?,
            flat.values.into_pyarray(py).reshape([n, w])?,
            flat.counts.into_pyarray(py),
        ))
    }

    /// Plays games with a seeded generator. With `policy=None` the optimal policy (in parallel); otherwise
    /// `policy(situation, legal_actions, score_so_far) -> action` (notation strings), called for every decision.
    /// Returns a dict with `scores` (uint16 array), summary statistics, `rng` and, if `logs`, the game logs.
    #[pyo3(signature = (games, seed = 0, logs = false, policy = None))]
    fn simulate<'py>(
        &self,
        py: Python<'py>,
        games: u64,
        seed: u64,
        logs: bool,
        policy: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let v = self.s.variant().clone();
        let sim = match policy {
            None => {
                let s = self.s.clone();
                py.detach(move || s.simulate_optimal(games, seed, logs))
            }
            Some(f) => {
                let mut p = PyPolicy { f, error: None };
                let mut sim = Simulation::default();
                for g in 0..games {
                    let log = play_game(&v, &mut p, seed, g, logs).map_err(err)?;
                    if let Some(e) = p.error.take() {
                        return Err(e);
                    }
                    sim.scores.push(log.final_score);
                    if logs {
                        sim.logs.push(log);
                    }
                }
                sim
            }
        };
        simulation_dict(py, &v, sim, logs)
    }

    /// Exports sampled situations with every option's value to a file (docs/export.md). `source` is
    /// `"optimal"`, `"perturbed"` or `"uniform"`; `format` is `"jsonl"` or `"parquet"`. Returns the row count.
    #[pyo3(signature = (path, rows, source = "optimal", seed = 0, format = "jsonl", perturb = 0.1))]
    #[allow(clippy::too_many_arguments)] // Python keyword arguments, not a Rust API.
    fn export(
        &self,
        py: Python<'_>,
        path: PathBuf,
        rows: usize,
        source: &str,
        seed: u64,
        format: &str,
        perturb: f64,
    ) -> PyResult<usize> {
        let src = match source {
            "optimal" => Source::Optimal,
            "perturbed" if (0.0..=1.0).contains(&perturb) => Source::Perturbed(perturb),
            "perturbed" => return Err(err("perturb must be between 0 and 1")),
            "uniform" => Source::Uniform,
            _ => return Err(err("source is optimal, perturbed or uniform")),
        };
        if format != "jsonl" && format != "parquet" {
            return Err(err("format is jsonl or parquet"));
        }
        let s = self.s.clone();
        let format = format.to_string();
        py.detach(move || -> Result<usize, String> {
            let data = export_rows(&s, src, seed, rows);
            let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let mut w = std::io::BufWriter::new(file);
            if format == "jsonl" {
                write_jsonl(&mut w, &s, &data).map_err(|e| e.to_string())?;
            } else {
                write_parquet(w, &s, &data).map_err(|e| e.to_string())?;
            }
            Ok(data.len())
        })
        .map_err(PyIOError::new_err)
    }

    fn __repr__(&self) -> String {
        format!("Solver({:?}, {})", self.s.variant().id(), self.precision())
    }
}

/// The flat batch layout as numpy arrays: codes, values, counts.
type FlatArrays<'py> = (Bound<'py, PyArray2<i16>>, Bound<'py, PyArray2<f64>>, Bound<'py, PyArray1<u16>>);

fn states_of(v: &Variant, filled: &[u32], upper: &[u16], armed: &[bool]) -> PyResult<Vec<State>> {
    if filled.len() != upper.len() || filled.len() != armed.len() {
        return Err(err("filled, upper and armed must have the same length"));
    }
    (0..filled.len())
        .map(|i| {
            let s = State { filled: filled[i], upper: upper[i], bonus_armed: armed[i] };
            v.check_state(&s).map_err(|e| err(format!("row {i}: {e}")))?;
            Ok(s)
        })
        .collect()
}

/// The ids of the built-in variants.
#[pyfunction]
fn builtin_variants() -> Vec<String> {
    Variant::builtin_ids()
}

#[pymodule]
fn _yatzy_solver(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyVariant>()?;
    m.add_class::<PyGame>()?;
    m.add_class::<PySolver>()?;
    m.add_function(wrap_pyfunction!(builtin_variants, m)?)?;
    m.add("RNG_VERSION", RNG_VERSION)?;
    m.add("SOLVER_VERSION", yatzy_solver::table::SOLVER_VERSION)?;
    Ok(())
}
