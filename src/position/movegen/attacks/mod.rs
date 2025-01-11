mod dyn_attacks;

#[cfg(feature = "static_attacks")]
mod static_attacks;

#[cfg(feature = "static_attacks")]
pub use static_attacks::{generate_bishops, generate_queens, generate_rooks};

#[cfg(not(feature = "static_attacks"))]
pub use dyn_attacks::{generate_bishops, generate_queens, generate_rooks};

pub use dyn_attacks::{generate_king, generate_knights, generate_pawns};

use crate::position::{
    bitboard::{Bitboard, GenericBB, Square, ToBB},
    piece::Piece,
    player::Player,
};

use super::AugmentedPos;

// quickly fetch attacks of any piece in a precomputed position
#[inline]
pub fn generate(
    meta: &AugmentedPos,
    pl: Player,
    p: Piece,
    src: Bitboard<Square>,
) -> Bitboard<GenericBB> {
    match p {
        Piece::Bishop => generate_bishops(src.declass(), meta.blockers),
        Piece::Pawn => generate_pawns(src.declass(), pl),
        Piece::Knight => generate_knights(src.declass()),
        Piece::Rook => generate_rooks(src.declass(), meta.blockers),
        Piece::Queen => generate_queens(src.declass(), meta.blockers),
        Piece::King => generate_king(src),
    }
}
