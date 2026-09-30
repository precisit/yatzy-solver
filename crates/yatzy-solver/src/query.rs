//! Queries (F2) and batch queries (F3): the exact value of every option in any situation.
//!
//! All values are **expected remaining scores**: the points still to come from this point on, under optimal
//! play afterwards, bonuses included. The score already on the card is not included; add it for the expected
//! final score.

use crate::rules::{Action, RulesError, Situation, State};
use crate::solver::TurnModel;
use crate::table::{Precision, Table};
use crate::variant::Variant;

/// Two option values closer than this are a tie, for a solver with f64 values. Exactly tied options are sums of
/// the same terms in different orders and differ by at most about 3e-14; the smallest gap between genuinely
/// different options measured on 200 000 situations is 5.7e-7 (docs/queries.md).
pub const TIE_EPSILON: f64 = 1e-9;

/// The tie tolerance for a solver loaded from an f32 table. Exactly tied options share their table entries, so
/// they stay within about 3e-14 in f32 too, and the same tolerance gives the same tie sets as f64 on every
/// measured situation, while 4 x the f32 value error (6e-5) would merge real near-ties (docs/queries.md).
pub const TIE_EPSILON_F32: f64 = 1e-9;

/// An option and its exact expected remaining score.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptionValue {
    pub action: Action,
    pub value: f64,
}

/// A solved variant ready for queries: the table of V and the precomputed turn model.
#[derive(Clone, Debug)]
pub struct Solver {
    model: TurnModel,
    values: Vec<f64>,
    precision: Precision,
}

impl Solver {
    /// A solver from a loaded table.
    pub fn from_table(table: &Table) -> Solver {
        Solver { model: TurnModel::new(table.variant()), values: table.values().to_vec(), precision: table.precision() }
    }

    /// Solves a variant in memory, keeping f64 values (about 4 s for Scandinavian Yatzy on all cores).
    pub fn build(v: &Variant) -> Solver {
        let model = TurnModel::new(v);
        let values = model.solve();
        Solver { model, values, precision: Precision::F64 }
    }

    pub fn variant(&self) -> &Variant {
        self.model.variant()
    }

    pub fn model(&self) -> &TurnModel {
        &self.model
    }

    /// The tolerance within which two option values are a tie: [`TIE_EPSILON`] for f64 values,
    /// [`TIE_EPSILON_F32`] for values from an f32 table.
    pub fn tie_epsilon(&self) -> f64 {
        match self.precision {
            Precision::F64 => TIE_EPSILON,
            Precision::F32 => TIE_EPSILON_F32,
        }
    }

    /// The precision of the table the values came from.
    pub fn precision(&self) -> Precision {
        self.precision
    }

    /// V for every state, in [`crate::StateSpace`] order.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// The expected remaining score from the start of a turn in state `s` (before the first roll). Zero when
    /// the game is over.
    #[inline]
    pub fn state_value(&self, s: &State) -> f64 {
        self.values[self.model.space().index(s)]
    }

    /// Checks that a situation can be queried: a valid state that is not final, the right number of dice,
    /// and a possible number of rerolls left.
    pub fn check_situation(&self, sit: &Situation) -> Result<(), RulesError> {
        let v = self.variant();
        v.check_state(&sit.state)?;
        if sit.dice.len() != v.dice() {
            return Err(RulesError::WrongDiceCount { expected: v.dice(), got: sit.dice.len() });
        }
        if v.is_over(&sit.state) {
            return Err(RulesError::GameOver);
        }
        if sit.rolls_left >= v.rolls() {
            return Err(RulesError::BadState(format!(
                "{} rerolls left; at most {} after a roll",
                sit.rolls_left,
                v.rolls() - 1
            )));
        }
        Ok(())
    }

    /// Every legal option with its exact value, in [`Variant::legal_actions`] order.
    pub fn option_values(&self, sit: &Situation) -> Result<Vec<OptionValue>, RulesError> {
        self.check_situation(sit)?;
        Ok(self
            .model
            .action_values(sit, &self.values)
            .into_iter()
            .map(|(action, value)| OptionValue { action, value })
            .collect())
    }

    /// The value of a situation: the value of its best option.
    pub fn situation_value(&self, sit: &Situation) -> Result<f64, RulesError> {
        Ok(best_value(&self.option_values(sit)?))
    }

    /// All the best options (every option within [`Solver::tie_epsilon`] of the maximum), in legal-action
    /// order.
    pub fn best_options(&self, sit: &Situation) -> Result<Vec<OptionValue>, RulesError> {
        let all = self.option_values(sit)?;
        let best = best_value(&all);
        let eps = self.tie_epsilon();
        Ok(all.into_iter().filter(|o| o.value >= best - eps).collect())
    }

    /// The best action; among ties, the first in legal-action order (categories before keeps).
    pub fn best_action(&self, sit: &Situation) -> Result<Action, RulesError> {
        Ok(self.best_options(sit)?[0].action)
    }

    /// The expected points lost by choosing `action` instead of a best option. Zero for a best option (ties
    /// within [`Solver::tie_epsilon`] count as zero); an error when the action is not legal.
    pub fn regret(&self, sit: &Situation, action: &Action) -> Result<f64, RulesError> {
        let all = self.option_values(sit)?;
        let best = best_value(&all);
        let chosen = all.iter().find(|o| o.action == *action).ok_or(match action {
            Action::Keep(_) => RulesError::BadKeep,
            Action::Score(c) => RulesError::CategoryNotAllowed(*c),
        })?;
        let r = best - chosen.value;
        Ok(if r <= self.tie_epsilon() { 0.0 } else { r })
    }

    /// Batch [`Solver::state_value`].
    pub fn state_values(&self, states: &[State]) -> Vec<f64> {
        states.iter().map(|s| self.state_value(s)).collect()
    }

    /// Batch [`Solver::option_values`], in parallel with the `parallel` feature. Results are in input order.
    pub fn option_values_batch(&self, sits: &[Situation]) -> Vec<Result<Vec<OptionValue>, RulesError>> {
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            sits.par_iter().with_min_len(64).map(|s| self.option_values(s)).collect()
        }
        #[cfg(not(feature = "parallel"))]
        {
            sits.iter().map(|s| self.option_values(s)).collect()
        }
    }
}

/// Batch option values in the flat, padded layout (for numpy and typed arrays): row `i` holds situation `i`'s
/// options in legal-action order, as action codes ([`crate::codes`]) and values, padded to `width` =
/// [`Variant::max_options`] with code -1 and value NaN.
#[derive(Clone, Debug, PartialEq)]
pub struct FlatOptions {
    pub rows: usize,
    pub width: usize,
    /// `rows x width` action codes, -1 for padding.
    pub codes: Vec<i16>,
    /// `rows x width` values, NaN for padding.
    pub values: Vec<f64>,
    /// The number of options in each row.
    pub counts: Vec<u16>,
}

impl FlatOptions {
    /// Scatters the rows into the dense layout: `rows x num_codes` values indexed by action code, NaN for
    /// actions that are not legal.
    pub fn to_dense(&self, num_codes: usize) -> Vec<f64> {
        let mut out = vec![f64::NAN; self.rows * num_codes];
        for r in 0..self.rows {
            for j in 0..usize::from(self.counts[r]) {
                let code = self.codes[r * self.width + j] as usize;
                out[r * num_codes + code] = self.values[r * self.width + j];
            }
        }
        out
    }
}

impl Solver {
    /// Batch option values in the flat layout. Fails on the first situation that cannot be queried, with its
    /// index.
    pub fn option_values_flat(&self, sits: &[Situation]) -> Result<FlatOptions, (usize, RulesError)> {
        let v = self.variant();
        let width = v.max_options();
        let mut flat = FlatOptions {
            rows: sits.len(),
            width,
            codes: vec![-1; sits.len() * width],
            values: vec![f64::NAN; sits.len() * width],
            counts: vec![0; sits.len()],
        };
        for (r, res) in self.option_values_batch(sits).into_iter().enumerate() {
            let opts = res.map_err(|e| (r, e))?;
            flat.counts[r] = opts.len() as u16;
            for (j, o) in opts.iter().enumerate() {
                flat.codes[r * width + j] = v.action_code(&o.action).expect("legal actions have codes") as i16;
                flat.values[r * width + j] = o.value;
            }
        }
        Ok(flat)
    }
}

fn best_value(all: &[OptionValue]) -> f64 {
    all.iter().map(|o| o.value).fold(f64::NEG_INFINITY, f64::max)
}
