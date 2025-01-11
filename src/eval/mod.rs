mod s_count_material;

use std::fmt::{Display, Formatter};

pub use s_count_material::MaterialBalance;

use super::position;
use super::position::{Position, movegen::Move, player::Player};

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

#[derive(Clone)]
pub struct EvalState {
    pub eval: Eval,
    pub pv: String,
}

//// Internal defs/implementations

#[derive(Clone)]
struct MoveList<'a>(Vec<Move<'a>>);
impl<'a> Default for MoveList<'a> {
    fn default() -> Self {
        MoveList { 0: Vec::new() }
    }
}
impl<'a> Display for MoveList<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for m in &self.0 {
            write!(f, "{m} ")?;
        }
        Ok(())
    }
}

impl ForcedMate {
    fn pick_best_for<'a>(p: Player, e0: &'a Self, e1: &'a Self) -> usize {
        if e0.p == e1.p {
            if p == e0.p {
                // mate for player, pick faster
                match e0.hmove_count < e1.hmove_count {
                    true => 0,
                    false => 1,
                }
            } else {
                // mate against player, pick longer
                match e0.hmove_count < e1.hmove_count {
                    false => 0,
                    true => 1,
                }
            }
        } else {
            // pick mate for player
            match e0.p == p {
                true => 0,
                false => 1,
            }
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
    fn pick_best_for<'a>(p: Player, e0: &'a Self, e1: &'a Self) -> usize {
        if e0.cp == e1.cp {
            // pick higher depth if equivalent
            match e0.depth < e1.depth {
                true => 1,
                false => 0,
            }
        } else {
            match (e0.cp < e1.cp) ^ (p == Player::White) {
                true => 0,
                false => 1,
            }
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
    fn pick_best_for(p: Player, e0: &Self, e1: &Self) -> usize {
        match (e0, e1) {
            (Self::Mate(x), Self::Mate(y)) => ForcedMate::pick_best_for(p, &x, &y),
            (Self::Approx(x), Self::Approx(y)) => ApproxEval::pick_best_for(p, &x, &y),
            (Self::Mate(x), Self::Approx(_)) => {
                if x.p == p {
                    0
                } else {
                    1
                }
            }
            _ => 1 - Eval::pick_best_for(p, e1, e0),
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
    pub fn pick_best_for(p: Player, e0: &Self, e1: &Self) -> usize {
        Eval::pick_best_for(p, &e0.eval, &e1.eval)
    }

    // create the evalState for the current move, knowing that the eval is the best for player
    pub fn nest(&mut self, m: &str) {
        self.eval = self.eval.nest();
        self.pv.insert_str(0, m);
    }
    pub fn new(e: Eval) -> Self {
        Self {
            eval: e,
            pv: String::new(),
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
