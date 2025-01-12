use crate::prelude::*;

use super::AugmentedPos;
use super::attacks; // goal is to get rid of it
use super::{Piece, Player};

pub fn generate_king_dests(src: Bitboard<Square>, meta: &AugmentedPos) -> Bitboard<GenericBB> {
    let player = meta.turn;
    let king = src; // meta.p.pos[player].pieces[PieceN::King as usize];

    // squares that arent attacked by opponent, and not occupied by my own pieces
    let free_sq_for_king = !meta.attacked[player.other() as usize] & !meta.p.pos.occupied(player);

    attacks::generate_king(king) & free_sq_for_king
}

// mask to know when to trigger en passant => check [(src | dest) & mask == (src | dest)]
#[warn(deprecated)]
fn get_en_passant_mask() -> [Bitboard<GenericBB>; 2] {
    [
        Bitboard(Rank::R2) | Bitboard(Rank::R4),
        Bitboard(Rank::R7) | Bitboard(Rank::R5),
    ]
}
pub fn generate_next_en_passant_data(
    p: Piece,
    src: Bitboard<Square>,
    dst: Bitboard<Square>,
    pl: Player,
) -> Bitboard<GenericBB> {
    if (p == Piece::Pawn) && (src | dst) & get_en_passant_mask()[pl as usize] == (src | dst) {
        match pl {
            Player::White => src + 1,
            Player::Black => src - 1,
        }
    } else {
        SpecialBB::Empty.declass()
    }
}

// returns dsts
pub fn pawn_move_up_nocap(
    src: Bitboard<Square>,
    p: Player,
    blockers: Bitboard<GenericBB>,
) -> Bitboard<GenericBB> {
    let mut out = match p {
        Player::White => src + 1,
        Player::Black => src - 1,
    } & !blockers;
    if src.declass() & (Bitboard(Rank::R2) | Bitboard(Rank::R7)) != SpecialBB::Empty.declass() {
        out = match p {
            Player::White => out | (out + 1),
            Player::Black => out | (out - 1),
        } & !blockers
    }
    out
}
