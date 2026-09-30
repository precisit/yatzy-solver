//! WebAssembly bindings (npm package `yatzy-solver`). The API mirrors the Python package: notation strings for
//! single queries, typed arrays for batches. Runs on one thread.

use wasm_bindgen::prelude::*;
use yatzy_solver::simulate::RNG_VERSION;
use yatzy_solver::table::SOLVER_VERSION;
use yatzy_solver::{Dice, Game, Precision, Situation, Solver, State, Table, Variant};

fn err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

fn dice_of(faces: &[u8]) -> Result<Dice, JsError> {
    Dice::from_faces(faces).map_err(err)
}

/// The version of the solver.
#[wasm_bindgen(js_name = solverVersion)]
pub fn solver_version() -> String {
    SOLVER_VERSION.to_string()
}

/// The version of the simulator's generator contract.
#[wasm_bindgen(js_name = rngVersion)]
pub fn rng_version() -> u32 {
    RNG_VERSION
}

/// The ids of the built-in variants.
#[wasm_bindgen(js_name = builtinVariants)]
pub fn builtin_variants() -> Vec<String> {
    Variant::builtin_ids()
}

/// A rule set: `new Variant("yatzy-scandinavian")` or `new Variant("american")`.
#[wasm_bindgen(js_name = Variant)]
pub struct JsVariant {
    v: Variant,
}

impl JsVariant {
    fn category(&self, id: &str) -> Result<usize, JsError> {
        self.v.category_index(id).ok_or_else(|| err(format!("no category {id:?} in {}", self.v.id())))
    }
}

#[wasm_bindgen(js_class = Variant)]
impl JsVariant {
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str) -> Result<JsVariant, JsError> {
        Variant::by_id(id).map(|v| JsVariant { v }).ok_or_else(|| err(format!("unknown variant {id:?}")))
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.v.id().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.v.name().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn dice(&self) -> usize {
        self.v.dice()
    }

    #[wasm_bindgen(getter)]
    pub fn rolls(&self) -> u8 {
        self.v.rolls()
    }

    /// Category ids in score-card order.
    #[wasm_bindgen(getter)]
    pub fn categories(&self) -> Vec<String> {
        self.v.categories().iter().map(|c| c.id.clone()).collect()
    }

    /// Category display names in score-card order.
    #[wasm_bindgen(getter, js_name = categoryNames)]
    pub fn category_names(&self) -> Vec<String> {
        self.v.categories().iter().map(|c| c.name.clone()).collect()
    }

    #[wasm_bindgen(getter, js_name = numActionCodes)]
    pub fn num_action_codes(&self) -> usize {
        self.v.num_action_codes()
    }

    #[wasm_bindgen(getter, js_name = maxOptions)]
    pub fn max_options(&self) -> usize {
        self.v.max_options()
    }

    #[wasm_bindgen(js_name = actionCode)]
    pub fn action_code(&self, action: &str) -> Result<u16, JsError> {
        let a = self.v.parse_action(action).map_err(err)?;
        self.v.action_code(&a).ok_or_else(|| err("no code for this action"))
    }

    #[wasm_bindgen(js_name = actionNotation)]
    pub fn action_notation(&self, code: u16) -> Result<String, JsError> {
        self.v.action_from_code(code).map(|a| self.v.format_action(&a)).ok_or_else(|| err("no such code"))
    }

    /// The score of dice in a category under the normal rules.
    pub fn score(&self, category: &str, dice: &[u8]) -> Result<u16, JsError> {
        Ok(self.v.score(self.category(category)?, &dice_of(dice)?))
    }

    /// The points (bonuses included) that scoring the situation's dice in a category would earn, or an error
    /// when that category may not be used now. Mirrors the rules engine's `score_in`, jokers included.
    #[wasm_bindgen(js_name = scoreIn)]
    pub fn score_in(&self, situation: &str, category: &str) -> Result<u16, JsError> {
        let sit = self.v.parse_situation(situation).map_err(err)?;
        Ok(self.v.score_in(&sit.state, &sit.dice, self.category(category)?).map_err(err)?.total())
    }

    #[wasm_bindgen(js_name = startTurn)]
    pub fn start_turn(&self, state: &str, dice: &[u8]) -> Result<String, JsError> {
        let s = self.v.parse_state(state).map_err(err)?;
        Ok(self.v.format_situation(&self.v.start_turn(&s, dice_of(dice)?).map_err(err)?))
    }

    #[wasm_bindgen(js_name = legalActions)]
    pub fn legal_actions(&self, situation: &str) -> Result<Vec<String>, JsError> {
        let sit = self.v.parse_situation(situation).map_err(err)?;
        Ok(self.v.legal_actions(&sit).map_err(err)?.iter().map(|a| self.v.format_action(a)).collect())
    }

    #[wasm_bindgen(js_name = applyKeep)]
    pub fn apply_keep(&self, situation: &str, keep: &[u8], rolled: &[u8]) -> Result<String, JsError> {
        let sit = self.v.parse_situation(situation).map_err(err)?;
        let next = self.v.apply_keep(&sit, &dice_of(keep)?, &dice_of(rolled)?).map_err(err)?;
        Ok(self.v.format_situation(&next))
    }

    /// Scores dice in a category: `[nextState, points]` (points include bonuses).
    #[wasm_bindgen(js_name = applyScore)]
    pub fn apply_score(&self, state: &str, dice: &[u8], category: &str) -> Result<Vec<JsValue>, JsError> {
        let s = self.v.parse_state(state).map_err(err)?;
        let (next, scored) = self.v.apply_score(&s, &dice_of(dice)?, self.category(category)?).map_err(err)?;
        Ok(vec![JsValue::from(self.v.format_state(&next)), JsValue::from(scored.total())])
    }

    #[wasm_bindgen(js_name = isOver)]
    pub fn is_over(&self, state: &str) -> Result<bool, JsError> {
        Ok(self.v.is_over(&self.v.parse_state(state).map_err(err)?))
    }
}

/// A full score card for playing a game.
#[wasm_bindgen(js_name = Game)]
pub struct JsGame {
    v: Variant,
    g: Game,
}

#[wasm_bindgen(js_class = Game)]
impl JsGame {
    #[wasm_bindgen(constructor)]
    pub fn new(variant: &JsVariant) -> JsGame {
        JsGame { v: variant.v.clone(), g: Game::new() }
    }

    #[wasm_bindgen(getter)]
    pub fn state(&self) -> String {
        self.v.format_state(self.g.state())
    }

    #[wasm_bindgen(getter)]
    pub fn total(&self) -> u16 {
        self.g.total()
    }

    #[wasm_bindgen(getter, js_name = upperBonus)]
    pub fn upper_bonus(&self) -> u16 {
        self.g.upper_bonus()
    }

    /// Five-of-a-kind bonus points earned so far (American rules).
    #[wasm_bindgen(getter)]
    pub fn bonus(&self) -> u16 {
        self.g.all_same_bonus()
    }

    #[wasm_bindgen(getter, js_name = isOver)]
    pub fn is_over(&self) -> bool {
        self.v.is_over(self.g.state())
    }

    /// The points in a category, or undefined when it is open.
    pub fn points(&self, category: &str) -> Result<Option<u16>, JsError> {
        let c = self.v.category_index(category).ok_or_else(|| err(format!("no category {category:?}")))?;
        Ok(self.g.points(c))
    }

    /// Scores dice in a category; returns the total points earned (bonuses included).
    pub fn score(&mut self, dice: &[u8], category: &str) -> Result<u16, JsError> {
        let c = self.v.category_index(category).ok_or_else(|| err(format!("no category {category:?}")))?;
        Ok(self.g.score(&self.v, &dice_of(dice)?, c).map_err(err)?.total())
    }
}

/// The flat batch layout: `codes` and `values` of `rows x width`, padded with -1 and NaN, and `counts`.
#[wasm_bindgen]
pub struct FlatOptions {
    rows: usize,
    width: usize,
    codes: Vec<i16>,
    values: Vec<f64>,
    counts: Vec<u16>,
}

#[wasm_bindgen]
impl FlatOptions {
    #[wasm_bindgen(getter)]
    pub fn rows(&self) -> usize {
        self.rows
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> usize {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn codes(&self) -> Vec<i16> {
        self.codes.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn values(&self) -> Vec<f64> {
        self.values.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn counts(&self) -> Vec<u16> {
        self.counts.clone()
    }
}

/// An option and its value.
#[wasm_bindgen]
pub struct OptionValue {
    action: String,
    value: f64,
    code: u16,
}

#[wasm_bindgen]
impl OptionValue {
    #[wasm_bindgen(getter)]
    pub fn action(&self) -> String {
        self.action.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> f64 {
        self.value
    }

    #[wasm_bindgen(getter)]
    pub fn code(&self) -> u16 {
        self.code
    }
}

/// A solved variant.
#[wasm_bindgen(js_name = Solver)]
pub struct JsSolver {
    s: Solver,
}

impl JsSolver {
    fn situation(&self, text: &str) -> Result<Situation, JsError> {
        self.s.variant().parse_situation(text).map_err(err)
    }

    fn options(&self, list: Vec<yatzy_solver::OptionValue>) -> Vec<OptionValue> {
        let v = self.s.variant();
        list.iter()
            .map(|o| OptionValue {
                action: v.format_action(&o.action),
                value: o.value,
                code: v.action_code(&o.action).expect("legal actions have codes"),
            })
            .collect()
    }
}

#[wasm_bindgen(js_class = Solver)]
impl JsSolver {
    /// Solves a variant in memory (f64 values). Slow in a browser; see the docs.
    pub fn build(variant: &JsVariant) -> JsSolver {
        JsSolver { s: Solver::build(&variant.v) }
    }

    /// A solver from table-file bytes (checked: checksum and variant definition).
    #[wasm_bindgen(js_name = fromTable)]
    pub fn from_table(variant: &JsVariant, bytes: &[u8]) -> Result<JsSolver, JsError> {
        let t = Table::from_bytes(bytes, &variant.v).map_err(err)?;
        Ok(JsSolver { s: Solver::from_table(&t) })
    }

    /// Solves a variant and returns the table-file bytes (`"f32"` or `"f64"`).
    #[wasm_bindgen(js_name = buildTable)]
    pub fn build_table(variant: &JsVariant, precision: &str) -> Result<Vec<u8>, JsError> {
        let p = match precision {
            "f32" => Precision::F32,
            "f64" => Precision::F64,
            _ => return Err(err("precision is f32 or f64")),
        };
        Ok(Table::build(&variant.v, p).to_bytes())
    }

    #[wasm_bindgen(getter)]
    pub fn precision(&self) -> String {
        match self.s.precision() {
            Precision::F32 => "f32".into(),
            Precision::F64 => "f64".into(),
        }
    }

    #[wasm_bindgen(getter, js_name = tieEpsilon)]
    pub fn tie_epsilon(&self) -> f64 {
        self.s.tie_epsilon()
    }

    #[wasm_bindgen(js_name = stateValue)]
    pub fn state_value(&self, state: &str) -> Result<f64, JsError> {
        Ok(self.s.state_value(&self.s.variant().parse_state(state).map_err(err)?))
    }

    /// Every legal option with its value and action code, in legal-action order.
    #[wasm_bindgen(js_name = optionValues)]
    pub fn option_values(&self, situation: &str) -> Result<Vec<OptionValue>, JsError> {
        let sit = self.situation(situation)?;
        Ok(self.options(self.s.option_values(&sit).map_err(err)?))
    }

    #[wasm_bindgen(js_name = bestOptions)]
    pub fn best_options(&self, situation: &str) -> Result<Vec<OptionValue>, JsError> {
        let sit = self.situation(situation)?;
        Ok(self.options(self.s.best_options(&sit).map_err(err)?))
    }

    #[wasm_bindgen(js_name = bestAction)]
    pub fn best_action(&self, situation: &str) -> Result<String, JsError> {
        let sit = self.situation(situation)?;
        Ok(self.s.variant().format_action(&self.s.best_action(&sit).map_err(err)?))
    }

    pub fn regret(&self, situation: &str, action: &str) -> Result<f64, JsError> {
        let sit = self.situation(situation)?;
        let a = self.s.variant().parse_action(action).map_err(err)?;
        self.s.regret(&sit, &a).map_err(err)
    }

    /// Batch state values from `filled` (Uint32Array), `upper` (Uint16Array), `armed` (Uint8Array, 0 or 1).
    #[wasm_bindgen(js_name = stateValues)]
    pub fn state_values(&self, filled: &[u32], upper: &[u16], armed: &[u8]) -> Result<Vec<f64>, JsError> {
        Ok(self.s.state_values(&self.states(filled, upper, armed)?))
    }

    /// Batch option values in the flat layout; `dice` is a Uint8Array of `n x dice` faces.
    #[wasm_bindgen(js_name = optionValuesBatch)]
    pub fn option_values_batch(
        &self,
        filled: &[u32],
        upper: &[u16],
        armed: &[u8],
        dice: &[u8],
        rolls_left: &[u8],
    ) -> Result<FlatOptions, JsError> {
        let v = self.s.variant();
        let states = self.states(filled, upper, armed)?;
        let n = states.len();
        if dice.len() != n * v.dice() || rolls_left.len() != n {
            return Err(err(format!("dice must hold n x {} faces and rollsLeft n values", v.dice())));
        }
        let sits = (0..n)
            .map(|i| {
                Ok(Situation {
                    state: states[i],
                    dice: dice_of(&dice[i * v.dice()..(i + 1) * v.dice()])?,
                    rolls_left: rolls_left[i],
                })
            })
            .collect::<Result<Vec<_>, JsError>>()?;
        let f = self.s.option_values_flat(&sits).map_err(|(i, e)| err(format!("row {i}: {e}")))?;
        Ok(FlatOptions { rows: f.rows, width: f.width, codes: f.codes, values: f.values, counts: f.counts })
    }

    /// Plays games under the optimal policy; returns the final scores.
    #[wasm_bindgen(js_name = simulateOptimal)]
    pub fn simulate_optimal(&self, games: u32, seed: u64) -> Vec<u16> {
        self.s.simulate_optimal(u64::from(games), seed, false).scores
    }
}

impl JsSolver {
    fn states(&self, filled: &[u32], upper: &[u16], armed: &[u8]) -> Result<Vec<State>, JsError> {
        if filled.len() != upper.len() || filled.len() != armed.len() {
            return Err(err("filled, upper and armed must have the same length"));
        }
        let v = self.s.variant();
        (0..filled.len())
            .map(|i| {
                let s = State { filled: filled[i], upper: upper[i], bonus_armed: armed[i] != 0 };
                v.check_state(&s).map_err(|e| err(format!("row {i}: {e}")))?;
                Ok(s)
            })
            .collect()
    }
}
