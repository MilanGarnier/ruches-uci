use super::Eval;
use super::Player;

use super::position::PieceSet;
use super::position::Position;
use super::position::piece::Piece;

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

fn held_value(pp: &PieceSet, p: Piece) -> usize {
    pp[p].count() * p.value()
}
fn player_material(pp: &PieceSet) -> usize {
    held_value(pp, Piece::Pawn)
        + held_value(pp, Piece::Knight)
        + held_value(pp, Piece::Bishop)
        + held_value(pp, Piece::Rook)
        + held_value(pp, Piece::Queen)
}

// TODO get rid of it
pub static mut NODES: usize = 0;

fn eval_fn(p: &Position) -> Eval {
    unsafe { NODES += 1 };
    let p = p.pos();
    let x =
        player_material(&p[Player::White]) as isize - player_material(&p[Player::Black]) as isize;
    Eval::Approx(super::ApproxEval {
        cp: i32::try_from(x).unwrap(),
        depth: 0,
    })
}
