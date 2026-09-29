//! # yatzy-solver
//!
//! An exact solver, rules engine and stable notation for Scandinavian Yatzy and American rules
//! (Yahtzee-compatible).
//!
//! - [`variant`]: rules as data. [`Variant::scandinavian`] and [`Variant::american`] are built in.
//! - [`rules`]: the rules engine: legal actions, applying them, scoring and the final score.
//! - [`notation`]: the canonical text form of states, situations and actions.
//! - [`solver`]: the exact solver (backward induction) and the within-turn values derived from its table.
//! - [`table`]: the table file format.
//! - [`verify`]: the brute-force reference solver, reduced games and published values (feature `verify`).
//!
//! ```
//! use yatzy_solver::{Dice, State, Variant};
//!
//! let v = Variant::scandinavian();
//! let dice = Dice::from_faces(&[3, 3, 3, 5, 5]).unwrap();
//! let sit = v.start_turn(&State::new(), dice).unwrap();
//! assert_eq!(v.format_situation(&sit), "dice 3 3 3 5 5 | rolls 2 | upper 0 | filled -");
//! let full_house = v.category_index("full_house").unwrap();
//! assert_eq!(v.score(full_house, &dice), 19);
//! ```

pub mod dice;
pub mod notation;
pub mod rules;
pub mod solver;
pub mod table;
pub mod value;
pub mod variant;
#[cfg(feature = "verify")]
pub mod verify;

pub use dice::Dice;
pub use notation::NotationError;
pub use rules::{Action, Game, RulesError, Scored, Situation, State};
pub use solver::{StateSpace, TurnModel};
pub use table::{Precision, Table};
pub use variant::{HouseRules, JokerRule, Variant, VariantDef};
