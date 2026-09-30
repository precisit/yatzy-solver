//! Variants: rules as data.
//!
//! A [`Variant`] defines the dice, the categories and how each scores, the bonuses and the special rules. The
//! rules engine ([`crate::rules`]) and the solver work from these definitions only, so a new rule set is a new
//! [`VariantDef`], not new code.

use std::fmt;

use crate::dice::{Dice, FACES, MAX_DICE};

/// Largest number of categories any variant may use.
pub const MAX_CATEGORIES: usize = 20;

/// How an n-of-a-kind category scores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfAKindScoring {
    /// n times the face (Scandinavian: three of a kind with 4 4 4 1 2 scores 12). With several qualifying faces,
    /// the highest.
    Matched,
    /// The sum of all dice (American: three of a kind with 4 4 4 1 2 scores 15).
    AllDice,
}

/// How a full house scores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FullHouseScoring {
    /// The sum of the dice (Scandinavian).
    AllDice,
    /// A fixed number of points (American: 25).
    Fixed(u16),
}

/// How a category scores a hand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CategoryKind {
    /// The sum of the dice showing `face` (Ones to Sixes). These form the upper section.
    Upper { face: u8 },
    /// At least `n` dice of one face.
    OfAKind { n: u8, scoring: OfAKindScoring },
    /// Two pairs of different faces, scoring the sum of the four dice (the two highest pairs if there are more).
    /// With `four_of_a_kind_counts`, four (or more) of one face also counts, scoring four times the face.
    /// With `single_pair_counts`, a single pair also scores, as twice its face. This is not a rule anyone plays
    /// by: it reproduces the scoring code behind the published 248.63 ([`crate::verify`]).
    TwoPairs { four_of_a_kind_counts: bool, single_pair_counts: bool },
    /// Three of one face and two of another. With `five_of_a_kind_counts`, five of a kind (all dice the same)
    /// also counts.
    FullHouse { scoring: FullHouseScoring, five_of_a_kind_counts: bool },
    /// The dice contain every face of at least one pattern (a face bitmask, bit `f - 1` for face `f`).
    Straight { patterns: Vec<u8>, points: u16 },
    /// The sum of the dice.
    Chance,
    /// All dice show the same face (Yatzy, Yahtzee).
    AllSame { points: u16 },
}

/// One category (box) on the score card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Category {
    /// Stable identifier used in the notation, e.g. `full_house`. Lowercase letters, digits and underscores.
    pub id: String,
    /// Display name, e.g. `Full house`.
    pub name: String,
    pub kind: CategoryKind,
}

/// The upper-section bonus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpperBonus {
    /// The upper-section total needed, e.g. 63.
    pub threshold: u16,
    /// The bonus points, e.g. 50 (Scandinavian) or 35 (American).
    pub points: u16,
}

/// When an all-same hand (a Yahtzee) may be used as a joker: scored in full house and the straights for their
/// full points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JokerRule {
    /// No joker: an all-same hand scores normally everywhere.
    None,
    /// Verhoeff's reading of the official rules, which gives the published 254.5896: the joker applies when the
    /// Yahtzee box and the upper box of the hand's face are both filled (with any score, zero included), and
    /// the player may score the hand in any open box.
    Free,
    /// The forced joker of the official rules: when the Yahtzee box is filled (with any score), the upper box of
    /// the hand's face must be used if it is open; otherwise any open box outside the upper section, with the
    /// joker applying; otherwise any open upper box, which scores 0.
    Forced,
}

/// The American Yahtzee bonus: when all dice show the same face and the Yahtzee box (the all-same category)
/// holds its full points, `points` (100) are added, wherever the hand is scored. The bonus does not count
/// toward the upper section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllSameBonus {
    pub points: u16,
    pub joker: JokerRule,
}

/// A variant definition: plain data, validated by [`Variant::new`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantDef {
    /// Stable identifier, e.g. `yatzy-scandinavian`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Number of dice (1 to 6).
    pub dice: u8,
    /// Rolls per turn, including the first (3 in both standard variants).
    pub rolls: u8,
    /// The categories, in score-card order.
    pub categories: Vec<Category>,
    pub upper_bonus: Option<UpperBonus>,
    pub all_same_bonus: Option<AllSameBonus>,
    /// Categories must be filled in score-card order ("tvångsyatzy").
    pub forced_order: bool,
}

/// A validated variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    def: VariantDef,
    /// Category index of the upper box for each face.
    upper_of_face: [Option<usize>; FACES],
    /// Mask of the upper-section categories.
    upper_mask: u32,
    /// The Yahtzee box, when the variant has a Yahtzee bonus.
    all_same_box: Option<usize>,
    /// The keeps that can be legal (multisets of fewer dice than a hand), in code order.
    keep_codes: Vec<Dice>,
}

/// Why a variant definition is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantError(pub String);

impl fmt::Display for VariantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid variant: {}", self.0)
    }
}

impl std::error::Error for VariantError {}

/// House-rule switches for Scandinavian Yatzy (SPEC 2.1). The defaults are all off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HouseRules {
    /// Five of a kind counts as a full house.
    pub five_of_a_kind_full_house: bool,
    /// Four of a kind counts as two pairs.
    pub four_of_a_kind_two_pairs: bool,
    /// Categories are filled top to bottom ("tvångsyatzy").
    pub forced_order: bool,
}

impl HouseRules {
    /// The suffix these switches add to the variant id: empty for the defaults, otherwise `+` and the switch
    /// names, e.g. `+fh5+forced`, in the order `+fh5`, `+tp4`, `+forced`.
    pub fn id_suffix(&self) -> String {
        let mut s = String::new();
        if self.five_of_a_kind_full_house {
            s.push_str("+fh5");
        }
        if self.four_of_a_kind_two_pairs {
            s.push_str("+tp4");
        }
        if self.forced_order {
            s.push_str("+forced");
        }
        s
    }
}

fn cat(id: &str, name: &str, kind: CategoryKind) -> Category {
    Category { id: id.to_string(), name: name.to_string(), kind }
}

fn upper_categories() -> Vec<Category> {
    const IDS: [(&str, &str); 6] = [
        ("ones", "Ones"),
        ("twos", "Twos"),
        ("threes", "Threes"),
        ("fours", "Fours"),
        ("fives", "Fives"),
        ("sixes", "Sixes"),
    ];
    IDS.iter().enumerate().map(|(i, (id, name))| cat(id, name, CategoryKind::Upper { face: i as u8 + 1 })).collect()
}

/// Face bitmask for a list of faces.
pub fn face_mask(faces: &[u8]) -> u8 {
    faces.iter().fold(0, |m, &f| m | 1 << (f - 1))
}

impl Variant {
    /// Id of Scandinavian Yatzy with default house rules.
    pub const SCANDINAVIAN: &'static str = "yatzy-scandinavian";
    /// Id of the American rules (Yahtzee-compatible) variant.
    pub const AMERICAN: &'static str = "american";

    /// Validates a definition.
    pub fn new(def: VariantDef) -> Result<Variant, VariantError> {
        let err = |s: String| Err(VariantError(s));
        if !valid_id(&def.id, true) {
            return err(format!("id {:?} must be lowercase letters, digits, '-', '_' or '+'", def.id));
        }
        if def.dice == 0 || usize::from(def.dice) > MAX_DICE {
            return err(format!("{} dice; must be 1 to {MAX_DICE}", def.dice));
        }
        if def.rolls == 0 {
            return err("rolls per turn must be at least 1".into());
        }
        if def.categories.is_empty() || def.categories.len() > MAX_CATEGORIES {
            return err(format!("{} categories; must be 1 to {MAX_CATEGORIES}", def.categories.len()));
        }
        let mut upper_of_face = [None; FACES];
        let mut upper_mask = 0u32;
        let mut all_same = Vec::new();
        for (i, c) in def.categories.iter().enumerate() {
            if !valid_id(&c.id, false) {
                return err(format!("category id {:?} must be lowercase letters, digits or '_'", c.id));
            }
            if def.categories[..i].iter().any(|o| o.id == c.id) {
                return err(format!("duplicate category id {:?}", c.id));
            }
            match &c.kind {
                CategoryKind::Upper { face } => {
                    if !(1..=6).contains(face) {
                        return err(format!("upper category {:?} has face {face}", c.id));
                    }
                    let slot = &mut upper_of_face[usize::from(face - 1)];
                    if slot.is_some() {
                        return err(format!("two upper categories for face {face}"));
                    }
                    *slot = Some(i);
                    upper_mask |= 1 << i;
                }
                CategoryKind::OfAKind { n, .. } => {
                    if *n == 0 {
                        return err(format!("category {:?}: n must be at least 1", c.id));
                    }
                }
                CategoryKind::Straight { patterns, .. } => {
                    if patterns.is_empty() || patterns.iter().any(|&p| p == 0 || p >= 1 << FACES) {
                        return err(format!("category {:?}: bad straight patterns", c.id));
                    }
                }
                CategoryKind::AllSame { .. } => all_same.push(i),
                _ => {}
            }
        }
        if let Some(b) = def.upper_bonus
            && upper_mask == 0
            && b.points > 0
        {
            return err("upper bonus without upper categories".into());
        }
        if def.all_same_bonus.is_some_and(|b| b.joker == JokerRule::Forced) && def.forced_order {
            return err("the forced-joker rule and forced order cannot be combined".into());
        }
        let all_same_box = match def.all_same_bonus {
            None => None,
            Some(_) => match all_same.as_slice() {
                [i] => Some(*i),
                _ => return err("a Yahtzee bonus needs exactly one all-same category".into()),
            },
        };
        let keep_codes = crate::dice::all_multisets_up_to(usize::from(def.dice) - 1);
        Ok(Variant { def, upper_of_face, upper_mask, all_same_box, keep_codes })
    }

    /// Scandinavian Yatzy with default house rules (SPEC 2.1).
    pub fn scandinavian() -> Variant {
        Variant::scandinavian_with(HouseRules::default())
    }

    /// Scandinavian Yatzy with the given house-rule switches. The id is `yatzy-scandinavian` plus
    /// [`HouseRules::id_suffix`].
    pub fn scandinavian_with(rules: HouseRules) -> Variant {
        use CategoryKind::*;
        let mut categories = upper_categories();
        categories.extend([
            cat("one_pair", "One pair", OfAKind { n: 2, scoring: OfAKindScoring::Matched }),
            cat(
                "two_pairs",
                "Two pairs",
                TwoPairs { four_of_a_kind_counts: rules.four_of_a_kind_two_pairs, single_pair_counts: false },
            ),
            cat("three_of_a_kind", "Three of a kind", OfAKind { n: 3, scoring: OfAKindScoring::Matched }),
            cat("four_of_a_kind", "Four of a kind", OfAKind { n: 4, scoring: OfAKindScoring::Matched }),
            cat(
                "small_straight",
                "Small straight",
                Straight { patterns: vec![face_mask(&[1, 2, 3, 4, 5])], points: 15 },
            ),
            cat(
                "large_straight",
                "Large straight",
                Straight { patterns: vec![face_mask(&[2, 3, 4, 5, 6])], points: 20 },
            ),
            cat(
                "full_house",
                "Full house",
                FullHouse {
                    scoring: FullHouseScoring::AllDice,
                    five_of_a_kind_counts: rules.five_of_a_kind_full_house,
                },
            ),
            cat("chance", "Chance", Chance),
            cat("yatzy", "Yatzy", AllSame { points: 50 }),
        ]);
        Variant::new(VariantDef {
            id: format!("{}{}", Variant::SCANDINAVIAN, rules.id_suffix()),
            name: "Scandinavian Yatzy".into(),
            dice: 5,
            rolls: 3,
            categories,
            upper_bonus: Some(UpperBonus { threshold: 63, points: 50 }),
            all_same_bonus: None,
            forced_order: rules.forced_order,
        })
        .expect("built-in variant is valid")
    }

    /// American rules, Yahtzee-compatible (SPEC 2.2), as used for the published values: the Yahtzee bonus and
    /// Verhoeff's free-placement joker ([`JokerRule::Free`]).
    pub fn american() -> Variant {
        Variant::american_with(Some(JokerRule::Free))
    }

    /// American rules with a choice of Yahtzee bonus and joker rule: `Some(joker)` for the 100-point Yahtzee
    /// bonus with that joker rule, `None` for neither bonus nor joker. Ids: `american` (free joker),
    /// `american+forced-joker`, `american+no-joker` (bonus, no joker) and `american+no-bonus`.
    pub fn american_with(bonus: Option<JokerRule>) -> Variant {
        use CategoryKind::*;
        let mut categories = upper_categories();
        categories.extend([
            cat("three_of_a_kind", "Three of a kind", OfAKind { n: 3, scoring: OfAKindScoring::AllDice }),
            cat("four_of_a_kind", "Four of a kind", OfAKind { n: 4, scoring: OfAKindScoring::AllDice }),
            cat(
                "full_house",
                "Full house",
                FullHouse { scoring: FullHouseScoring::Fixed(25), five_of_a_kind_counts: false },
            ),
            cat(
                "small_straight",
                "Small straight",
                Straight {
                    patterns: vec![face_mask(&[1, 2, 3, 4]), face_mask(&[2, 3, 4, 5]), face_mask(&[3, 4, 5, 6])],
                    points: 30,
                },
            ),
            cat(
                "large_straight",
                "Large straight",
                Straight { patterns: vec![face_mask(&[1, 2, 3, 4, 5]), face_mask(&[2, 3, 4, 5, 6])], points: 40 },
            ),
            cat("five_of_a_kind", "Yahtzee", AllSame { points: 50 }),
            cat("chance", "Chance", Chance),
        ]);
        let suffix = match bonus {
            Some(JokerRule::Free) => "",
            Some(JokerRule::Forced) => "+forced-joker",
            Some(JokerRule::None) => "+no-joker",
            None => "+no-bonus",
        };
        Variant::new(VariantDef {
            id: format!("{}{suffix}", Variant::AMERICAN),
            name: "American rules (Yahtzee-compatible)".into(),
            dice: 5,
            rolls: 3,
            categories,
            upper_bonus: Some(UpperBonus { threshold: 63, points: 35 }),
            all_same_bonus: bonus.map(|joker| AllSameBonus { points: 100, joker }),
            forced_order: false,
        })
        .expect("built-in variant is valid")
    }

    /// The ids of all built-in variants: American rules with each joker choice, and Scandinavian Yatzy with
    /// every combination of house-rule switches.
    pub fn builtin_ids() -> Vec<String> {
        let mut ids: Vec<String> = ["", "+forced-joker", "+no-joker", "+no-bonus"]
            .iter()
            .map(|s| format!("{}{s}", Variant::AMERICAN))
            .collect();
        for bits in 0..8u8 {
            let rules = HouseRules {
                five_of_a_kind_full_house: bits & 1 != 0,
                four_of_a_kind_two_pairs: bits & 2 != 0,
                forced_order: bits & 4 != 0,
            };
            ids.push(format!("{}{}", Variant::SCANDINAVIAN, rules.id_suffix()));
        }
        ids
    }

    /// A built-in variant by id: `american` (optionally `+forced-joker`, `+no-joker` or `+no-bonus`), or
    /// `yatzy-scandinavian` optionally followed by house-rule switches in canonical order (`+fh5`, `+tp4`,
    /// `+forced`).
    pub fn by_id(id: &str) -> Option<Variant> {
        if let Some(rest) = id.strip_prefix(Variant::AMERICAN) {
            let bonus = match rest {
                "" => Some(JokerRule::Free),
                "+forced-joker" => Some(JokerRule::Forced),
                "+no-joker" => Some(JokerRule::None),
                "+no-bonus" => None,
                _ => return None,
            };
            return Some(Variant::american_with(bonus));
        }
        let rest = id.strip_prefix(Variant::SCANDINAVIAN)?;
        let rules = HouseRules {
            five_of_a_kind_full_house: rest.contains("+fh5"),
            four_of_a_kind_two_pairs: rest.contains("+tp4"),
            forced_order: rest.contains("+forced"),
        };
        (rules.id_suffix() == rest).then(|| Variant::scandinavian_with(rules))
    }

    /// The definition.
    pub fn def(&self) -> &VariantDef {
        &self.def
    }

    pub fn id(&self) -> &str {
        &self.def.id
    }

    pub fn name(&self) -> &str {
        &self.def.name
    }

    /// Number of dice.
    pub fn dice(&self) -> usize {
        usize::from(self.def.dice)
    }

    /// Rolls per turn, including the first.
    pub fn rolls(&self) -> u8 {
        self.def.rolls
    }

    pub fn categories(&self) -> &[Category] {
        &self.def.categories
    }

    pub fn num_categories(&self) -> usize {
        self.def.categories.len()
    }

    /// Mask with one bit per category.
    pub fn all_mask(&self) -> u32 {
        (1u32 << self.num_categories()) - 1
    }

    /// Mask of the upper-section categories.
    pub fn upper_mask(&self) -> u32 {
        self.upper_mask
    }

    pub fn upper_bonus(&self) -> Option<UpperBonus> {
        self.def.upper_bonus
    }

    pub fn all_same_bonus(&self) -> Option<AllSameBonus> {
        self.def.all_same_bonus
    }

    /// The keeps that can be legal, in action-code order ([`crate::codes`]).
    pub fn keep_codes(&self) -> &[Dice] {
        &self.keep_codes
    }

    /// The joker rule; [`JokerRule::None`] without a Yahtzee bonus.
    pub fn joker_rule(&self) -> JokerRule {
        self.def.all_same_bonus.map_or(JokerRule::None, |b| b.joker)
    }

    /// The Yahtzee box (the all-same category), when the variant has a Yahtzee bonus.
    pub fn all_same_box(&self) -> Option<usize> {
        self.all_same_box
    }

    /// The upper category for a face, if any.
    pub fn upper_of_face(&self, face: u8) -> Option<usize> {
        self.upper_of_face[usize::from(face - 1)]
    }

    pub fn forced_order(&self) -> bool {
        self.def.forced_order
    }

    /// The index of the category with this id.
    pub fn category_index(&self, id: &str) -> Option<usize> {
        self.def.categories.iter().position(|c| c.id == id)
    }

    /// The largest upper-section total that matters to the rules: the bonus threshold, or 0 without a bonus.
    /// The solver caps the upper sum here.
    pub fn upper_cap(&self) -> u16 {
        self.def.upper_bonus.map_or(0, |b| b.threshold)
    }

    /// The score of `dice` in category `c` under the normal rules (no joker).
    pub fn score(&self, c: usize, dice: &Dice) -> u16 {
        score_kind(&self.def.categories[c].kind, dice)
    }

    /// A canonical text form of everything in the definition that affects play, one item per line. The table
    /// file stores a hash of it, so a table cannot be used with a variant whose rules differ. Display names are
    /// not included.
    pub fn canonical_text(&self) -> String {
        let d = &self.def;
        let mut out = format!("variant {}\ndice {}\nrolls {}\n", d.id, d.dice, d.rolls);
        for c in &d.categories {
            let kind = match &c.kind {
                CategoryKind::Upper { face } => format!("upper {face}"),
                CategoryKind::OfAKind { n, scoring } => format!("of_a_kind {n} {scoring:?}"),
                CategoryKind::TwoPairs { four_of_a_kind_counts, single_pair_counts } => {
                    format!("two_pairs {four_of_a_kind_counts} {single_pair_counts}")
                }
                CategoryKind::FullHouse { scoring, five_of_a_kind_counts } => {
                    format!("full_house {scoring:?} {five_of_a_kind_counts}")
                }
                CategoryKind::Straight { patterns, points } => format!("straight {patterns:?} {points}"),
                CategoryKind::Chance => "chance".to_string(),
                CategoryKind::AllSame { points } => format!("all_same {points}"),
            };
            out.push_str(&format!("category {} {kind}\n", c.id));
        }
        if let Some(b) = d.upper_bonus {
            out.push_str(&format!("upper_bonus {} {}\n", b.threshold, b.points));
        }
        if let Some(b) = d.all_same_bonus {
            out.push_str(&format!("all_same_bonus {} {:?}\n", b.points, b.joker));
        }
        out.push_str(&format!("forced_order {}\n", d.forced_order));
        out
    }

    /// The score of a joker in category `c`: full house and straights
    /// score their full points (the sum of the dice for a full house scored as the sum), everything else scores
    /// normally.
    pub fn joker_score(&self, c: usize, dice: &Dice) -> u16 {
        match &self.def.categories[c].kind {
            CategoryKind::FullHouse { scoring: FullHouseScoring::Fixed(p), .. } => *p,
            CategoryKind::FullHouse { scoring: FullHouseScoring::AllDice, .. } => dice.sum(),
            CategoryKind::Straight { points, .. } => *points,
            k => score_kind(k, dice),
        }
    }
}

fn valid_id(s: &str, variant: bool) -> bool {
    !s.is_empty()
        && s.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || (variant && (ch == '-' || ch == '+'))
        })
}

/// The score of `dice` in a category of this kind, under the normal rules.
pub fn score_kind(kind: &CategoryKind, dice: &Dice) -> u16 {
    let counts = dice.counts();
    // Highest face with at least n dice.
    let highest_with = |n: u8| (1..=6u8).rev().find(|&f| counts[usize::from(f - 1)] >= n);
    match kind {
        CategoryKind::Upper { face } => u16::from(*face) * u16::from(dice.count(*face)),
        CategoryKind::OfAKind { n, scoring } => match highest_with(*n) {
            None => 0,
            Some(f) => match scoring {
                OfAKindScoring::Matched => u16::from(f) * u16::from(*n),
                OfAKindScoring::AllDice => dice.sum(),
            },
        },
        CategoryKind::TwoPairs { four_of_a_kind_counts, single_pair_counts } => {
            let pairs: Vec<u8> = (1..=6u8).rev().filter(|&f| counts[usize::from(f - 1)] >= 2).take(2).collect();
            match pairs.as_slice() {
                [a, b] => 2 * (u16::from(*a) + u16::from(*b)),
                _ => {
                    let four = highest_with(4).filter(|_| *four_of_a_kind_counts).map_or(0, |f| 4 * u16::from(f));
                    let single = pairs.first().filter(|_| *single_pair_counts).map_or(0, |&f| 2 * u16::from(f));
                    four.max(single)
                }
            }
        }
        CategoryKind::FullHouse { scoring, five_of_a_kind_counts } => {
            let mut nonzero: Vec<u8> = counts.iter().copied().filter(|&c| c > 0).collect();
            nonzero.sort_unstable();
            let is_full_house = nonzero == [2, 3] || (*five_of_a_kind_counts && nonzero.len() == 1 && dice.len() >= 2);
            match (is_full_house, scoring) {
                (false, _) => 0,
                (true, FullHouseScoring::AllDice) => dice.sum(),
                (true, FullHouseScoring::Fixed(p)) => *p,
            }
        }
        CategoryKind::Straight { patterns, points } => {
            let m = dice.face_mask();
            if patterns.iter().any(|&p| p & !m == 0) { *points } else { 0 }
        }
        CategoryKind::Chance => dice.sum(),
        CategoryKind::AllSame { points } => {
            if dice.all_same() {
                *points
            } else {
                0
            }
        }
    }
}
