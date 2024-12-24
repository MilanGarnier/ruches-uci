use super::bitboard;
use super::bitboard::GenericBB;
use super::bitboard::SpecialBB;
use super::bitboard::Square;
use super::bitboard::ToBB;

use super::AugmentedPos;
use super::bitboard::{Bitboard as BB, Rank};
use super::dyn_attacks; // goal is to get rid of it
use super::{Piece, Player};

pub fn generate_king_dests(src: BB<Square>, meta: &AugmentedPos) -> BB<GenericBB> {
    let player = meta.turn;
    let king = src; // meta.p.pos[player].pieces[PieceN::King as usize];

    // squares that arent attacked by opponent, and not occupied by my own pieces
    let free_sq_for_king = !meta.attacked[player.other()] & !meta.occupied[player];

    dyn_attacks::generate_king(king) & free_sq_for_king
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
