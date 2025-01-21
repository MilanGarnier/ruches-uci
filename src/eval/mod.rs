//! Evaluation module handles types & traits for evaluating game positions.
//!
//! This includes:
//! - `Eval` enum for representing evaluation results (mate or approximate score)
//! - `ApproxEval` for centipawn-based evaluations with search depth
//! - `ForcedMate` for forced mate sequences
//! - `EvalState` for maintaining evaluation and principal variation
//! - `MaterialBalance` trait for piece counting evaluations
//!
//! The evaluation system supports both mate-in-N and centipawn scores,
//! with proper comparison and nesting logic for search algorithms.
mod s_count_material;

use std::fmt::{Display, Formatter};

use movegen::SimplifiedMove;
pub use s_count_material::MaterialBalance;

use super::prelude::*;

#[derive(Clone, Copy)]
pub struct ApproxEval {
    cp: i32,
    depth: usize,
}
impl ApproxEval {
    pub const EQUAL: Self = ApproxEval { cp: 0, depth: 0 };
}

#[derive(Clone, Copy)]
pub struct ForcedMate {
    p: Player,
    hmove_count: usize,
}

pub trait BasicEvaluation: Clone {
    fn t() -> Self;
    fn eval(p: &Position) -> Eval;
}

#[derive(Clone, Copy)]
pub enum Eval {
    Mate(ForcedMate),
    Approx(ApproxEval),
}

pub struct EvalState {
    pub eval: Eval,
    pub pv: MoveList,
}

//// Internal defs/implementations

pub struct MoveList(Vec<Move>);
impl Default for MoveList {
    fn default() -> Self {
        MoveList { 0: Vec::new() }
    }
}
impl Display for MoveList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for m in &self.0 {
            write!(f, "{m:?} ")?;
        }
        Ok(())
    }
}

impl ForcedMate {
    fn pick_best_for<'a>(p: Player, e0: &'a Self, e1: &'a Self) -> bool {
        if e0.p == e1.p {
            if p == e0.p {
                // mate for player, pick faster
                e0.hmove_count >= e1.hmove_count
            } else {
                // mate against player, pick longer
                e0.hmove_count < e1.hmove_count
            }
        } else {
            // pick mate for player
            e0.p != p
        }
    }
    fn nest(&self) -> Self {
        Self {
            p: self.p,
            hmove_count: self.hmove_count + 1,
        }
    }
}
impl Display for ForcedMate {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self.p {
            Player::White => write!(f, "#{}", (self.hmove_count + 1) / 2),
            Player::Black => write!(f, "#-{}", (self.hmove_count + 1) / 2),
        }
    }
}

impl ApproxEval {
    fn pick_best_for<'a>(p: Player, e0: &'a Self, e1: &'a Self) -> bool {
        if e0.cp == e1.cp {
            // pick higher depth if equivalent
            e0.depth < e1.depth
        } else {
            !((e0.cp < e1.cp) ^ (p == Player::White))
        }
    }
    fn nest(&self) -> Self {
        Self {
            cp: self.cp,
            depth: self.depth + 1,
        }
    }
}
impl Display for ApproxEval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let depth = self.depth;
        let cp = self.cp; //TODO: add neg multiplier if engine is black side
        write!(f, "depth {depth} score cp {cp}",)
    }
}

impl Eval {
    // lost()
    pub fn m0(p: Player) -> Self {
        Eval::Mate(ForcedMate {
            p: p,
            hmove_count: 0,
        })
    }
    pub fn draw() -> Self {
        Eval::Approx(ApproxEval { cp: 0, depth: 0 })
    }
    // DEPRECATED: will be a reduction function instead
    fn pick_best_for(p: Player, e0: Self, e1: Self) -> bool {
        match (e0, e1) {
            (Self::Mate(x), Self::Mate(y)) => ForcedMate::pick_best_for(p, &x, &y),
            (Self::Approx(x), Self::Approx(y)) => ApproxEval::pick_best_for(p, &x, &y),
            (Self::Mate(x), Self::Approx(_)) => {
                if x.p == p {
                    false
                } else {
                    true
                }
            }
            _ => !Eval::pick_best_for(p, e1, e0),
        }
    }
    fn nest(self) -> Self {
        match self {
            Self::Approx(x) => Self::Approx(x.nest()),
            Self::Mate(x) => Self::Mate(x.nest()),
        }
    }
}
impl Display for Eval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Approx(x) => write!(f, "{x}"),
            Self::Mate(x) => write!(f, "{x}"),
        }
    }
}

impl EvalState {
    pub fn pick_best_for(p: Player, e0: Self, e1: Self) -> Self {
        match Eval::pick_best_for(p, e0.eval, e1.eval) {
            true => e1,
            _ => e0,
        }
    }

    // create the evalState for the current move, knowing that the eval is the best for player
    pub fn nest(&mut self, m: Move) {
        self.eval = self.eval.nest();
        self.pv.0.push(m);
    }
    pub fn new(e: Eval) -> Self {
        Self {
            eval: e,
            pv: MoveList::default(),
        }
    }
}

impl Display for EvalState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let eval = &self.eval;
        let pv = &self.pv;
        write!(f, "{eval} {pv}",)
    }
}
