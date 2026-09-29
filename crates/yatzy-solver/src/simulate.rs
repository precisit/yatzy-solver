//! Simulation (F4): play games with a seeded random generator, under the optimal policy or a caller's policy.
//!
//! The generator is part of the stable contract ([`RNG_VERSION`] 1): the same seed gives the same games in every
//! version and every language binding. It is xoshiro256** (Blackman and Vigna), its state filled by four outputs
//! of SplitMix64. Streams are derived with the SplitMix64 mixer `mix(x)` (one SplitMix64 step from state `x`,
//! a bijection):
//!
//! - **dice**: turn `t` (0-based) of game `g` in a run with seed `s` has its own stream, seeded with
//!   `mix(mix(s ^ DICE_DOMAIN) ^ (g << 8 | t))`. Every roll of the turn draws a full block of `n` dice (`n` the
//!   variant's dice), in order; a reroll of `m` dice uses the first `m` of its block. So roll `r` of turn `t` is
//!   the same whatever any policy decides, and the first roll of a turn depends only on the seed, game and turn;
//! - **policy**: game `g` gives the policy its own stream, seeded with `mix(mix(s ^ POLICY_DOMAIN) ^ g)`, so a
//!   randomizing policy never shifts the dice;
//! - a die is `1 + x % 6` for the next output `x`, drawing again while `x >= 2^64 - (2^64 mod 6)` (no bias).
//!
//! For a fixed seed, `key -> mix(c ^ key)` is a bijection, so distinct (game, turn) pairs never share a stream.
//! Game indices must be below 2^56 and turns below 256. Each game has its own streams, so results do not depend
//! on how games are spread over threads, and different policies play on identical dice (common random numbers).

use crate::dice::Dice;
use crate::query::Solver;
use crate::rules::{Action, Game, RulesError, Scored, Situation};
use crate::variant::Variant;

/// The version of the generator contract, recorded in logs and exports.
pub const RNG_VERSION: u32 = 1;

/// Domain constant of the dice streams.
pub const DICE_DOMAIN: u64 = 0x6469_6365_0000_0001;
/// Domain constant of the policy streams.
pub const POLICY_DOMAIN: u64 = 0x706f_6c69_6379_0001;

/// The largest game index plus one.
pub const MAX_GAMES: u64 = 1 << 56;

/// The seeded random generator (xoshiro256**).
#[derive(Clone, Debug)]
pub struct Rng {
    s: [u64; 4],
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// The SplitMix64 mixer: one SplitMix64 step from state `x` (a bijection on 64-bit values).
pub fn mix(x: u64) -> u64 {
    let mut s = x;
    splitmix64(&mut s)
}

impl Rng {
    /// A generator seeded from one number with SplitMix64.
    pub fn new(seed: u64) -> Rng {
        let mut sm = seed;
        Rng { s: std::array::from_fn(|_| splitmix64(&mut sm)) }
    }

    /// The dice stream of turn `turn` of game `game` in a run with seed `seed`.
    pub fn for_dice(seed: u64, game: u64, turn: u32) -> Rng {
        assert!(game < MAX_GAMES && turn < 256, "game or turn index out of range");
        Rng::new(mix(mix(seed ^ DICE_DOMAIN) ^ (game << 8 | u64::from(turn))))
    }

    /// The policy stream of game `game` in a run with seed `seed`.
    pub fn for_policy(seed: u64, game: u64) -> Rng {
        assert!(game < MAX_GAMES, "game index out of range");
        Rng::new(mix(mix(seed ^ POLICY_DOMAIN) ^ game))
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.s;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// A uniform integer in `0..n`, without bias.
    pub fn below(&mut self, n: u64) -> u64 {
        let zone = u64::MAX - (u64::MAX % n + 1) % n;
        loop {
            let x = self.next_u64();
            if x <= zone {
                return x % n;
            }
        }
    }

    /// One die, 1 to 6.
    pub fn die(&mut self) -> u8 {
        self.below(6) as u8 + 1
    }

    /// `m` dice.
    pub fn roll(&mut self, m: usize) -> Dice {
        let faces: Vec<u8> = (0..m).map(|_| self.die()).collect();
        Dice::from_faces(&faces).expect("at most six dice")
    }

    /// A block of `n` dice for a roll that uses the first `m` of them.
    pub fn roll_block(&mut self, n: usize, m: usize) -> Dice {
        let faces: Vec<u8> = (0..n).map(|_| self.die()).collect();
        Dice::from_faces(&faces[..m]).expect("at most six dice")
    }
}

/// A way of choosing actions.
pub trait Policy {
    /// Chooses one of `legal` (never empty) in situation `sit`. `game` is the full score card so far, and `rng`
    /// the policy's own stream for this game (separate from the dice), for policies that randomize.
    fn choose(&mut self, variant: &Variant, game: &Game, sit: &Situation, legal: &[Action], rng: &mut Rng) -> Action;
}

/// Optimal play (maximizing the expected final score); among ties, the first best action in legal order.
pub struct OptimalPolicy<'a> {
    pub solver: &'a Solver,
}

impl Policy for OptimalPolicy<'_> {
    fn choose(&mut self, _: &Variant, _: &Game, sit: &Situation, _: &[Action], _: &mut Rng) -> Action {
        self.solver.best_action(sit).expect("situations reached in play are legal")
    }
}

/// Uniformly random legal actions.
pub struct RandomPolicy;

impl Policy for RandomPolicy {
    fn choose(&mut self, _: &Variant, _: &Game, _: &Situation, legal: &[Action], rng: &mut Rng) -> Action {
        legal[rng.below(legal.len() as u64) as usize]
    }
}

/// One decision in a game log.
#[derive(Clone, Debug, PartialEq)]
pub struct Decision {
    pub situation: Situation,
    pub action: Action,
    /// For a category: the points it earned.
    pub scored: Option<Scored>,
}

/// The log of one game.
#[derive(Clone, Debug, PartialEq)]
pub struct GameLog {
    /// The run's seed.
    pub seed: u64,
    /// The game's index in its run; with the seed (and [`RNG_VERSION`]) it reproduces the game.
    pub game: u64,
    pub decisions: Vec<Decision>,
    pub final_score: u16,
}

impl GameLog {
    /// The log as lines: `rng 1 | seed <s> | game <g>`, then `<situation> => <action>` in the stable notation
    /// per decision, then `final <score>`.
    pub fn to_lines(&self, v: &Variant) -> Vec<String> {
        let mut out = vec![format!("rng {RNG_VERSION} | seed {} | game {}", self.seed, self.game)];
        out.extend(
            self.decisions
                .iter()
                .map(|d| format!("{} => {}", v.format_situation(&d.situation), v.format_action(&d.action))),
        );
        out.push(format!("final {}", self.final_score));
        out
    }
}

/// An error from a simulation: a policy chose an action that is not legal.
#[derive(Clone, Debug, PartialEq)]
pub struct IllegalChoice {
    pub game: u64,
    pub situation: Situation,
    pub action: Action,
    pub error: RulesError,
}

impl std::fmt::Display for IllegalChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "game {}: the policy chose an illegal action {:?}: {}", self.game, self.action, self.error)
    }
}

impl std::error::Error for IllegalChoice {}

/// Plays game `game` of a run with seed `seed`. Returns the final score and, if `log`, the decisions.
pub fn play_game(
    v: &Variant,
    policy: &mut dyn Policy,
    seed: u64,
    game: u64,
    log: bool,
) -> Result<GameLog, IllegalChoice> {
    let n = v.dice();
    let mut policy_rng = Rng::for_policy(seed, game);
    let mut card = Game::new();
    let mut decisions = Vec::new();
    while !v.is_over(card.state()) {
        let mut dice_rng = Rng::for_dice(seed, game, card.state().turns_played());
        let mut sit = v.start_turn(card.state(), dice_rng.roll_block(n, n)).expect("a fresh roll is legal");
        loop {
            let legal = v.legal_actions(&sit).expect("the situation is legal");
            let action = policy.choose(v, &card, &sit, &legal, &mut policy_rng);
            let illegal = |error| IllegalChoice { game, situation: sit, action, error };
            if !legal.contains(&action) {
                let error = match action {
                    Action::Keep(_) => RulesError::BadKeep,
                    Action::Score(c) => RulesError::CategoryNotAllowed(c),
                };
                return Err(illegal(error));
            }
            match action {
                Action::Keep(k) => {
                    let rolled = dice_rng.roll_block(n, n - k.len());
                    let next = v.apply_keep(&sit, &k, &rolled).map_err(illegal)?;
                    if log {
                        decisions.push(Decision { situation: sit, action, scored: None });
                    }
                    sit = next;
                }
                Action::Score(c) => {
                    let scored = card.score(v, &sit.dice, c).map_err(illegal)?;
                    if log {
                        decisions.push(Decision { situation: sit, action, scored: Some(scored) });
                    }
                    break;
                }
            }
        }
    }
    Ok(GameLog { seed, game, decisions, final_score: card.total() })
}

/// The result of a run.
#[derive(Clone, Debug, Default)]
pub struct Simulation {
    /// The final score of each game, in game order.
    pub scores: Vec<u16>,
    /// The game logs, when requested.
    pub logs: Vec<GameLog>,
}

/// Plays games `0..games` with a caller-supplied policy, one after another.
pub fn simulate(
    v: &Variant,
    policy: &mut dyn Policy,
    games: u64,
    seed: u64,
    logs: bool,
) -> Result<Simulation, IllegalChoice> {
    let mut sim = Simulation::default();
    for g in 0..games {
        let log = play_game(v, policy, seed, g, logs)?;
        sim.scores.push(log.final_score);
        if logs {
            sim.logs.push(log);
        }
    }
    Ok(sim)
}

impl Solver {
    /// Plays games `0..games` under the optimal policy, in parallel with the `parallel` feature. The result is
    /// identical to [`simulate`] with [`OptimalPolicy`].
    pub fn simulate_optimal(&self, games: u64, seed: u64, logs: bool) -> Simulation {
        let one = |g: u64| {
            play_game(self.variant(), &mut OptimalPolicy { solver: self }, seed, g, logs)
                .expect("optimal play is legal")
        };
        #[cfg(feature = "parallel")]
        let results: Vec<GameLog> = {
            use rayon::prelude::*;
            (0..games).into_par_iter().map(one).collect()
        };
        #[cfg(not(feature = "parallel"))]
        let results: Vec<GameLog> = (0..games).map(one).collect();
        let scores = results.iter().map(|l| l.final_score).collect();
        Simulation { scores, logs: if logs { results } else { Vec::new() } }
    }
}

/// Summary statistics of a score sample.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub games: usize,
    pub mean: f64,
    /// Sample standard deviation.
    pub std_dev: f64,
    /// Standard error of the mean.
    pub std_error: f64,
    pub min: u16,
    pub max: u16,
    /// The lower median (the value at index `(n - 1) / 2` of the sorted scores).
    pub median: u16,
}

impl Simulation {
    /// Summary statistics of the scores.
    pub fn summary(&self) -> Summary {
        let n = self.scores.len();
        assert!(n > 0, "no games");
        let mean = self.scores.iter().map(|&s| f64::from(s)).sum::<f64>() / n as f64;
        let var = if n > 1 {
            self.scores.iter().map(|&s| (f64::from(s) - mean).powi(2)).sum::<f64>() / (n - 1) as f64
        } else {
            0.0
        };
        let mut sorted = self.scores.clone();
        sorted.sort_unstable();
        Summary {
            games: n,
            mean,
            std_dev: var.sqrt(),
            std_error: (var / n as f64).sqrt(),
            min: sorted[0],
            max: sorted[n - 1],
            median: sorted[(n - 1) / 2],
        }
    }

    /// The score at percentile `p` (0 to 100), nearest rank.
    pub fn percentile(&self, p: f64) -> u16 {
        let mut sorted = self.scores.clone();
        sorted.sort_unstable();
        let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
        sorted[rank.min(sorted.len()) - 1]
    }

    /// The number of games at each final score, `histogram()[s]` for score `s`.
    pub fn histogram(&self) -> Vec<u64> {
        let max = self.scores.iter().copied().max().unwrap_or(0);
        let mut h = vec![0u64; usize::from(max) + 1];
        for &s in &self.scores {
            h[usize::from(s)] += 1;
        }
        h
    }
}
