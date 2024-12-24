pub mod s_count_material;
use super::position;

use super::position::{Player, Position, movegen::Move};

struct MoveList(Vec<Move>);
impl MoveList {
    pub fn repr(&self) -> String {
        let mut s: String = String::new();
        for m in self.0.iter().rev() {
            s += &m.uci();
            s += " ";
        }
        s.pop();
        s
    }
    pub fn new() -> Self {
        MoveList { 0: Vec::new() }
    }
}

#[derive(Clone, Copy)]
pub struct ForcedMate {
    p: Player,
    hmove_count: usize,
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
    fn rich_repr(&self) -> String {
        format!("#{}", (self.hmove_count + 1) / 2)
    }
}
#[derive(Clone, Copy)]
pub struct ApproxEval {
    x: i32,
    depth: usize,
}
impl ApproxEval {
    fn pick_best_for<'a>(p: Player, e0: &'a Self, e1: &'a Self) -> usize {
        if e0.x == e1.x {
            // pick higher depth if equivalent
            match e0.depth < e1.depth {
                true => 1,
                false => 0,
            }
        } else {
            match (e0.x < e1.x) ^ (p == Player::White) {
                true => 0,
                false => 1,
            }
        }
    }
    fn nest(&self) -> Self {
        Self {
            x: self.x,
            depth: self.depth + 1,
        }
    }
    fn rich_repr(&self) -> String {
        format!("{:+.2} (d={})", self.x, self.depth)
    }
}

#[derive(Clone, Copy)]
pub enum Eval {
    Mate(ForcedMate),
    Approx(ApproxEval),
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
        Eval::Approx(ApproxEval { x: 0, depth: 0 })
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
            _ => {
                1-Eval::pick_best_for(p, e1, e0)
            }
        }
    }
    fn nest(&self) -> Self {
        match self {
            Self::Approx(x) => Self::Approx(x.nest()),
            Self::Mate(x) => Self::Mate(x.nest()),
        }
    }
    fn rich_repr(&self) -> String {
        match self {
            Self::Approx(x) => x.rich_repr(),
            Self::Mate(x) => x.rich_repr(),
        }
    }
}

pub struct EvalState {
    eval: Eval,
    movelist: MoveList,
}
impl EvalState {
    pub fn pick_best_for(p: Player, e0: &Self, e1: &Self) -> usize {
        Eval::pick_best_for(p, &e0.eval, &e1.eval)
    }

    // create the evalState for the current move, knowing that the eval is the best for player
    pub fn nest(&mut self, m: &Move) {
        self.eval = self.eval.nest();
        self.movelist.0.push(*m);
    }
    pub fn new(e: &Eval) -> Self {
        Self {
            eval: *e,
            movelist: MoveList::new(),
        }
    }
    pub fn rich_repr(&self) -> String {
        self.eval.rich_repr() + " [ " + &self.movelist.repr() + " ]"
    }
}

pub type EvalFun = dyn Fn(&Position) -> Eval;
