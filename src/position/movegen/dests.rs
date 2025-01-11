use super::bitboard;
use super::bitboard::GenericBB;
use super::bitboard::SpecialBB;
use super::bitboard::Square;
use super::bitboard::ToBB;

use super::AugmentedPos;
use super::bitboard::{Bitboard as BB, Rank};
use super::{Piece, Player};

// masks forbidden destinations
pub fn mask(p: Piece, meta: &AugmentedPos) -> BB<GenericBB> {
    let turn = meta.turn;
    match p {
        Piece::Pawn => todo!(),
        Piece::Knight => todo!(),
        Piece::Bishop => !meta.blockers,
        Piece::Rook => !meta.blockers,
        Piece::Queen => !meta.blockers,
        Piece::King => !meta.attacked[turn.other()] & !meta.occupied[turn],
    }
}

// mask to know when to trigger en passant => check [(src | dest) & mask == (src | dest)]
#[warn(deprecated)]
fn get_en_passant_mask() -> [BB<GenericBB>; 2] {
    [BB(Rank::R2) | BB(Rank::R4), BB(Rank::R7) | BB(Rank::R5)]
}
pub fn generate_next_en_passant_data(
    p: Piece,
    src: BB<Square>,
    dst: BB<Square>,
    pl: Player,
) -> BB<GenericBB> {
    if (p == Piece::Pawn) && (src | dst) & get_en_passant_mask()[pl as usize] == (src | dst) {
        match pl {
            Player::White => src + 1,
            Player::Black => src - 1,
        }
    } else {
        bitboard::SpecialBB::Empty.declass()
    }
}

// returns dsts
pub fn pawn_move_up_nocap(src: BB<Square>, p: Player, blockers: BB<GenericBB>) -> BB<GenericBB> {
    let mut out = match p {
        Player::White => src + 1,
        Player::Black => src - 1,
    } & !blockers;
    if src.declass() & (BB(Rank::R2) | BB(Rank::R7)) != SpecialBB::Empty.declass() {
        out = match p {
            Player::White => out | (out + 1),
            Player::Black => out | (out - 1),
        } & !blockers
    }
    out
}
