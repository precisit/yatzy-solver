//! The rules engine (F7): legal actions, applying them, scoring, game over and the final score.
//!
//! Three levels of state:
//! - [`State`]: what matters for future decisions between turns (the filled categories, the upper-section total
//!   and, with a Yahtzee bonus, whether the Yahtzee box holds its full points);
//! - [`Situation`]: a [`State`] within a turn, with the dice showing and the rerolls left;
//! - [`Game`]: a full score card, for playing games (a [`State`] plus the points recorded in each box).
//!
//! All functions take the [`Variant`] explicitly; the state types are plain data.

use std::fmt;

use crate::dice::Dice;
use crate::variant::{JokerRule, MAX_CATEGORIES, Variant};

/// The decision-relevant state between turns.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct State {
    /// Bit `c` is set when category `c` is filled.
    pub filled: u32,
    /// The upper-section total so far (not capped).
    pub upper: u16,
    /// The Yahtzee box is filled with its full points, so further all-same hands earn the Yahtzee bonus. Always
    /// false in variants without a Yahtzee bonus.
    pub yahtzee_armed: bool,
}

/// A decision point within a turn.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Situation {
    pub state: State,
    /// The dice showing.
    pub dice: Dice,
    /// Rerolls left in this turn: 2 after the first roll of a three-roll turn, 0 after the last.
    pub rolls_left: u8,
}

/// A decision: keep some dice and reroll the rest, or score the dice in a category.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// Keep these dice (a proper sub-multiset of the dice showing) and reroll the others.
    Keep(Dice),
    /// Score the dice in the category with this index.
    Score(usize),
}

/// The points an action of type [`Action::Score`] earns.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Scored {
    pub category: usize,
    /// Points written in the box.
    pub points: u16,
    /// Upper-section bonus earned by this score (the upper total reached the threshold).
    pub upper_bonus: u16,
    /// Yahtzee bonus earned by this score.
    pub yahtzee_bonus: u16,
}

impl Scored {
    /// All points earned.
    pub fn total(&self) -> u16 {
        self.points + self.upper_bonus + self.yahtzee_bonus
    }
}

/// Why an action or state is not legal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RulesError {
    /// The hand has the wrong number of dice.
    WrongDiceCount { expected: usize, got: usize },
    /// There is no category with this index.
    NoSuchCategory(usize),
    /// The category is already filled.
    CategoryFilled(usize),
    /// The category is open but may not be used now (forced order, or the forced-joker rule).
    CategoryNotAllowed(usize),
    /// No rerolls are left.
    NoRollsLeft,
    /// The kept dice are not a proper sub-multiset of the dice showing.
    BadKeep,
    /// Rerolled dice of the wrong number.
    BadReroll { expected: usize, got: usize },
    /// Every category is filled.
    GameOver,
    /// The state is inconsistent with the variant (e.g. bits outside its categories).
    BadState(String),
}

impl fmt::Display for RulesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RulesError::WrongDiceCount { expected, got } => write!(f, "expected {expected} dice, got {got}"),
            RulesError::NoSuchCategory(c) => write!(f, "no category with index {c}"),
            RulesError::CategoryFilled(c) => write!(f, "category {c} is already filled"),
            RulesError::CategoryNotAllowed(c) => write!(f, "category {c} may not be used now"),
            RulesError::NoRollsLeft => f.write_str("no rerolls left"),
            RulesError::BadKeep => f.write_str("the kept dice are not a proper subset of the dice showing"),
            RulesError::BadReroll { expected, got } => write!(f, "expected {expected} rerolled dice, got {got}"),
            RulesError::GameOver => f.write_str("the game is over"),
            RulesError::BadState(s) => write!(f, "bad state: {s}"),
        }
    }
}

impl std::error::Error for RulesError {}

impl State {
    /// The state at the start of a game.
    pub fn new() -> State {
        State::default()
    }

    pub fn is_filled(&self, c: usize) -> bool {
        self.filled & (1 << c) != 0
    }

    /// The number of filled categories, which is also the number of turns played.
    pub fn turns_played(&self) -> u32 {
        self.filled.count_ones()
    }
}

impl Variant {
    /// Checks that a state is consistent with this variant.
    pub fn check_state(&self, s: &State) -> Result<(), RulesError> {
        if s.filled & !self.all_mask() != 0 {
            return Err(RulesError::BadState(format!("filled mask {:#x} has bits outside the categories", s.filled)));
        }
        if s.filled & self.upper_mask() == 0 && s.upper != 0 {
            return Err(RulesError::BadState("upper total without filled upper categories".into()));
        }
        if s.yahtzee_armed && !self.yahtzee_box().is_some_and(|y| s.is_filled(y)) {
            return Err(RulesError::BadState("Yahtzee bonus armed but the Yahtzee box is not filled".into()));
        }
        Ok(())
    }

    /// True when every category is filled.
    pub fn is_over(&self, s: &State) -> bool {
        s.filled == self.all_mask()
    }

    /// True when `dice` is an extra Yahtzee: all dice the same while the Yahtzee box is filled. It earns the
    /// Yahtzee bonus when the box holds its full points.
    pub fn extra_yahtzee(&self, s: &State, dice: &Dice) -> bool {
        self.yahtzee_box().is_some_and(|y| s.is_filled(y)) && dice.all_same()
    }

    /// True when `dice` scores as a joker in state `s` (full points for full house and the straights).
    pub fn joker_applies(&self, s: &State, dice: &Dice) -> bool {
        if !self.extra_yahtzee(s, dice) {
            return false;
        }
        match self.joker_rule() {
            JokerRule::None => false,
            JokerRule::Forced => true,
            JokerRule::Free => self.upper_of_face(dice.faces()[0]).is_none_or(|u| s.is_filled(u)),
        }
    }

    /// Whether category `c` may be scored now, ignoring whether the dice are complete.
    fn category_allowed(&self, s: &State, dice: &Dice, c: usize) -> Result<(), RulesError> {
        if c >= self.num_categories() {
            return Err(RulesError::NoSuchCategory(c));
        }
        if s.is_filled(c) {
            return Err(RulesError::CategoryFilled(c));
        }
        let open = self.all_mask() & !s.filled;
        if self.forced_order() && c != open.trailing_zeros() as usize {
            return Err(RulesError::CategoryNotAllowed(c));
        }
        if self.joker_rule() == JokerRule::Forced && self.extra_yahtzee(s, dice) {
            let face = dice.faces()[0];
            let own_upper_open = self.upper_of_face(face).filter(|&u| !s.is_filled(u));
            let lower_open = open & !self.upper_mask() != 0;
            let allowed = match own_upper_open {
                Some(u) => c == u,
                None if lower_open => self.upper_mask() & (1 << c) == 0,
                None => true,
            };
            if !allowed {
                return Err(RulesError::CategoryNotAllowed(c));
            }
        }
        Ok(())
    }

    fn check_dice(&self, dice: &Dice) -> Result<(), RulesError> {
        if dice.len() != self.dice() {
            return Err(RulesError::WrongDiceCount { expected: self.dice(), got: dice.len() });
        }
        Ok(())
    }

    /// The points scoring `dice` in category `c` would earn, or why that is not legal.
    pub fn score_in(&self, s: &State, dice: &Dice, c: usize) -> Result<Scored, RulesError> {
        self.check_dice(dice)?;
        self.category_allowed(s, dice, c)?;
        let joker = self.joker_applies(s, dice);
        let points = if joker { self.joker_score(c, dice) } else { self.score(c, dice) };
        let upper_bonus = match self.upper_bonus() {
            Some(b)
                if self.upper_mask() & (1 << c) != 0 && s.upper < b.threshold && s.upper + points >= b.threshold =>
            {
                b.points
            }
            _ => 0,
        };
        let yahtzee_bonus = match self.yahtzee_bonus() {
            Some(b) if s.yahtzee_armed && self.extra_yahtzee(s, dice) => b.points,
            _ => 0,
        };
        Ok(Scored { category: c, points, upper_bonus, yahtzee_bonus })
    }

    /// Every legal category for `dice`, in score-card order, with the points each earns.
    pub fn score_choices(&self, s: &State, dice: &Dice) -> Result<Vec<Scored>, RulesError> {
        self.check_dice(dice)?;
        if self.is_over(s) {
            return Err(RulesError::GameOver);
        }
        Ok((0..self.num_categories()).filter_map(|c| self.score_in(s, dice, c).ok()).collect())
    }

    /// The state after scoring `dice` in category `c`, and the points earned.
    pub fn apply_score(&self, s: &State, dice: &Dice, c: usize) -> Result<(State, Scored), RulesError> {
        let scored = self.score_in(s, dice, c)?;
        let mut next = *s;
        next.filled |= 1 << c;
        if self.upper_mask() & (1 << c) != 0 {
            next.upper += scored.points;
        }
        if Some(c) == self.yahtzee_box() {
            next.yahtzee_armed = self.yahtzee_bonus().is_some() && scored.points > 0;
        }
        Ok((next, scored))
    }

    /// The situation after the first roll of a turn.
    pub fn start_turn(&self, s: &State, dice: Dice) -> Result<Situation, RulesError> {
        self.check_dice(&dice)?;
        if self.is_over(s) {
            return Err(RulesError::GameOver);
        }
        Ok(Situation { state: *s, dice, rolls_left: self.rolls() - 1 })
    }

    /// Every legal action, in a fixed order: the categories in score-card order, then (when rerolls are left)
    /// every distinct proper sub-multiset of the dice to keep, by size and then lexicographically. Keeping all
    /// dice is not an action: not rerolling is the same as scoring now.
    pub fn legal_actions(&self, sit: &Situation) -> Result<Vec<Action>, RulesError> {
        let mut out: Vec<Action> =
            self.score_choices(&sit.state, &sit.dice)?.into_iter().map(|s| Action::Score(s.category)).collect();
        if sit.rolls_left > 0 {
            let subs = sit.dice.sub_multisets();
            out.extend(subs[..subs.len() - 1].iter().map(|&k| Action::Keep(k)));
        }
        Ok(out)
    }

    /// Checks that `keep` may be kept in `sit`.
    pub fn check_keep(&self, sit: &Situation, keep: &Dice) -> Result<(), RulesError> {
        self.check_dice(&sit.dice)?;
        if self.is_over(&sit.state) {
            return Err(RulesError::GameOver);
        }
        if sit.rolls_left == 0 {
            return Err(RulesError::NoRollsLeft);
        }
        if !keep.is_subset_of(&sit.dice) || keep.len() == sit.dice.len() {
            return Err(RulesError::BadKeep);
        }
        Ok(())
    }

    /// The situation after keeping `keep` and rerolling the other dice, which came up `rolled`.
    pub fn apply_keep(&self, sit: &Situation, keep: &Dice, rolled: &Dice) -> Result<Situation, RulesError> {
        self.check_keep(sit, keep)?;
        let expected = sit.dice.len() - keep.len();
        if rolled.len() != expected {
            return Err(RulesError::BadReroll { expected, got: rolled.len() });
        }
        let dice = keep.plus(rolled).expect("sizes checked");
        Ok(Situation { state: sit.state, dice, rolls_left: sit.rolls_left - 1 })
    }
}

/// A full score card for playing a game.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Game {
    state: State,
    card: [u16; MAX_CATEGORIES],
    upper_bonus: u16,
    yahtzee_bonus: u16,
}

impl Game {
    /// A new game with an empty score card.
    pub fn new() -> Game {
        Game::default()
    }

    /// The decision-relevant state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// The points in category `c`, or `None` when it is open.
    pub fn points(&self, c: usize) -> Option<u16> {
        (c < MAX_CATEGORIES && self.state.is_filled(c)).then(|| self.card[c])
    }

    /// Upper-section bonus earned so far.
    pub fn upper_bonus(&self) -> u16 {
        self.upper_bonus
    }

    /// Yahtzee bonus points earned so far.
    pub fn yahtzee_bonus(&self) -> u16 {
        self.yahtzee_bonus
    }

    /// The score so far, bonuses included. This is the final score once the game is over.
    pub fn total(&self) -> u16 {
        self.card.iter().sum::<u16>() + self.upper_bonus + self.yahtzee_bonus
    }

    /// Scores `dice` in category `c`.
    pub fn score(&mut self, variant: &Variant, dice: &Dice, c: usize) -> Result<Scored, RulesError> {
        let (next, scored) = variant.apply_score(&self.state, dice, c)?;
        self.state = next;
        self.card[c] = scored.points;
        self.upper_bonus += scored.upper_bonus;
        self.yahtzee_bonus += scored.yahtzee_bonus;
        Ok(scored)
    }
}
