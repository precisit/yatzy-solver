//! The stable text notation (F6) for states, situations and actions.
//!
//! The canonical forms, documented in `docs/notation.md`:
//!
//! ```text
//! state:      upper 21 | filled ones,twos,chance
//! situation:  dice 1 3 3 5 6 | rolls 2 | upper 21 | filled ones,twos,chance
//! actions:    keep 3 3        keep -        score full_house
//! ```
//!
//! In a variant with a Yahtzee bonus, a state whose Yahtzee box is filled ends with `| yahtzee_box 50` (the box
//! holds its full points, so further Yahtzees earn the bonus) or `| yahtzee_box 0`.
//!
//! Formatting always produces the canonical form; these bytes are fixed across versions. Parsing accepts the
//! canonical form with any amount of whitespace around tokens and separators, and rejects anything else.

use std::fmt;

use crate::dice::Dice;
use crate::rules::{Action, Situation, State};
use crate::variant::{CategoryKind, Variant};

/// A notation string that could not be parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotationError(pub String);

impl fmt::Display for NotationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bad notation: {}", self.0)
    }
}

impl std::error::Error for NotationError {}

fn bad<T>(msg: impl Into<String>) -> Result<T, NotationError> {
    Err(NotationError(msg.into()))
}

fn yahtzee_points(v: &Variant) -> Option<u16> {
    let y = v.yahtzee_box()?;
    match v.categories()[y].kind {
        CategoryKind::AllSame { points } => Some(points),
        _ => None,
    }
}

impl Variant {
    /// The canonical notation of a state, e.g. `upper 21 | filled ones,twos,chance`.
    pub fn format_state(&self, s: &State) -> String {
        let filled: Vec<&str> =
            (0..self.num_categories()).filter(|&c| s.is_filled(c)).map(|c| self.categories()[c].id.as_str()).collect();
        let filled = if filled.is_empty() { "-".to_string() } else { filled.join(",") };
        let mut out = format!("upper {} | filled {}", s.upper, filled);
        if let Some(y) = self.yahtzee_box()
            && s.is_filled(y)
        {
            let pts = if s.yahtzee_armed { yahtzee_points(self).unwrap_or(0) } else { 0 };
            out.push_str(&format!(" | yahtzee_box {pts}"));
        }
        out
    }

    /// The canonical notation of a situation, e.g. `dice 1 3 3 5 6 | rolls 2 | upper 21 | filled ones,chance`.
    pub fn format_situation(&self, sit: &Situation) -> String {
        format!("dice {} | rolls {} | {}", sit.dice, sit.rolls_left, self.format_state(&sit.state))
    }

    /// The canonical notation of an action: `keep 3 3`, `keep -` (reroll everything) or `score full_house`.
    pub fn format_action(&self, a: &Action) -> String {
        match a {
            Action::Keep(d) => format!("keep {d}"),
            Action::Score(c) => format!("score {}", self.categories()[*c].id),
        }
    }

    /// Parses a state. The result is checked with [`Variant::check_state`].
    pub fn parse_state(&self, text: &str) -> Result<State, NotationError> {
        let fields = split_fields(text);
        self.parse_state_fields(&fields)
    }

    /// Parses a situation. The number of dice and the rerolls left are checked against the variant.
    pub fn parse_situation(&self, text: &str) -> Result<Situation, NotationError> {
        let fields = split_fields(text);
        if fields.len() < 2 {
            return bad("a situation needs dice, rolls, upper and filled fields");
        }
        let dice = parse_dice(field(&fields[0], "dice")?)?;
        if dice.len() != self.dice() {
            return bad(format!("{} dice; this variant uses {}", dice.len(), self.dice()));
        }
        let rolls = field(&fields[1], "rolls")?;
        let rolls_left: u8 = match rolls.as_slice() {
            [r] => r.parse().map_err(|_| NotationError(format!("bad rolls value {r:?}")))?,
            _ => return bad("rolls takes one number"),
        };
        if rolls_left >= self.rolls() {
            return bad(format!("rolls {rolls_left}; at most {} rerolls are left after a roll", self.rolls() - 1));
        }
        let state = self.parse_state_fields(&fields[2..])?;
        Ok(Situation { state, dice, rolls_left })
    }

    /// Parses an action. Legality in a given situation is checked separately (e.g. with
    /// [`Variant::legal_actions`]).
    pub fn parse_action(&self, text: &str) -> Result<Action, NotationError> {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        match tokens.as_slice() {
            ["keep", rest @ ..] => {
                let keep = parse_dice(rest.to_vec())?;
                if keep.len() >= self.dice() {
                    return bad(format!("a keep holds at most {} dice", self.dice() - 1));
                }
                Ok(Action::Keep(keep))
            }
            ["score", id] => match self.category_index(id) {
                Some(c) => Ok(Action::Score(c)),
                None => bad(format!("no category {id:?} in {}", self.id())),
            },
            _ => bad(format!("expected `keep <dice>` or `score <category>`, got {text:?}")),
        }
    }

    fn parse_state_fields(&self, fields: &[Vec<&str>]) -> Result<State, NotationError> {
        let (upper_f, filled_f, ybox_f) = match fields {
            [u, f] => (u, f, None),
            [u, f, y] => (u, f, Some(y)),
            _ => return bad("a state is `upper <n> | filled <ids>` with an optional `| yahtzee_box <n>`"),
        };
        let upper: u16 = match field(upper_f, "upper")?.as_slice() {
            [n] => n.parse().map_err(|_| NotationError(format!("bad upper value {n:?}")))?,
            _ => return bad("upper takes one number"),
        };
        let mut filled = 0u32;
        match field(filled_f, "filled")?.as_slice() {
            ["-"] => {}
            [list] => {
                let mut last = None;
                for id in list.split(',') {
                    let c = self.category_index(id).ok_or_else(|| NotationError(format!("no category {id:?}")))?;
                    if last.is_some_and(|l| c <= l) {
                        return bad("filled categories must be listed once each, in score-card order");
                    }
                    last = Some(c);
                    filled |= 1 << c;
                }
            }
            _ => return bad("filled takes a comma-separated list without spaces, or `-`"),
        }
        let y_filled = self.yahtzee_box().is_some_and(|y| filled & (1 << y) != 0);
        let yahtzee_armed = match ybox_f {
            None if y_filled => return bad("a filled Yahtzee box needs a `yahtzee_box` field"),
            None => false,
            Some(_) if !y_filled => return bad("`yahtzee_box` is only given when the Yahtzee box is filled"),
            Some(f) => match field(f, "yahtzee_box")?.as_slice() {
                ["0"] => false,
                [n] if n.parse::<u16>().ok() == yahtzee_points(self) => true,
                _ => return bad("yahtzee_box is 0 or the box's full points"),
            },
        };
        let s = State { filled, upper, yahtzee_armed };
        self.check_state(&s).map_err(|e| NotationError(e.to_string()))?;
        Ok(s)
    }
}

/// Splits on `|` into whitespace-separated tokens per field.
fn split_fields(text: &str) -> Vec<Vec<&str>> {
    text.split('|').map(|f| f.split_whitespace().collect()).collect()
}

/// Checks a field's name and returns its values.
fn field<'a>(f: &[&'a str], name: &str) -> Result<Vec<&'a str>, NotationError> {
    match f.split_first() {
        Some((n, rest)) if *n == name => Ok(rest.to_vec()),
        _ => bad(format!("expected field `{name}`, got {:?}", f.join(" "))),
    }
}

/// Parses dice faces in ascending order (`1 3 3 5 6`), or `-` for none.
fn parse_dice(tokens: Vec<&str>) -> Result<Dice, NotationError> {
    if tokens == ["-"] {
        return Ok(Dice::EMPTY);
    }
    if tokens.is_empty() {
        return bad("no dice; write `-` for none");
    }
    let mut faces = Vec::with_capacity(tokens.len());
    for t in tokens {
        match t.parse::<u8>() {
            Ok(v) if (1..=6).contains(&v) && t.len() == 1 => faces.push(v),
            _ => return bad(format!("bad die face {t:?}")),
        }
    }
    if faces.windows(2).any(|w| w[0] > w[1]) {
        return bad("dice must be in ascending order");
    }
    Dice::from_faces(&faces).map_err(|e| NotationError(e.to_string()))
}
