//! Traditional material balance evaluation function
//!
//! Simply counts the number of pieces each side has and assigns them standard weights:
//! - Pawn = 100
//! - Knight = 300
//! - Bishop = 300
//! - Rook = 500
//! - Queen = 900
//!
//! Difference between white and black material is returned as the evaluation score.use super::Eval;
use super::Eval;
use super::Player;
use crate::prelude::*;

use super::BasicEvaluation;

#[derive(Clone)]
pub struct MaterialBalance {}
impl BasicEvaluation for MaterialBalance {
    fn eval(p: &Position) -> Eval {
        eval_fn(p)
    }
    fn t() -> Self {
        MaterialBalance {}
    }
}

impl Piece {
    fn value(self) -> usize {
        match self {
            Piece::Pawn => 100,
            Piece::Knight => 300,
            Piece::Bishop => 300,
            Piece::Rook => 500,
            Piece::Queen => 900,
            Piece::King => panic!(),
        }
    }
}

// TODO get rid of it when switching to parallel
pub static mut NODES: usize = 0;

fn eval_fn(p: &Position) -> Eval {
    unsafe { NODES += 1 };
    use enum_iterator::all;
    let a = all::<Player>()
        .zip(all::<Piece>())
        .map(|(pl, pc)| -> isize {
            let ps = p.pos();
            let bb = ps[(pl, pc)];
            (1 - 2 * (pl as isize)) * (bb.into_iter().count() as isize)
        });
    let s: isize = a.sum();
    Eval::Approx(super::ApproxEval {
        cp: i32::try_from(s).unwrap(),
        depth: 0,
    })
}
