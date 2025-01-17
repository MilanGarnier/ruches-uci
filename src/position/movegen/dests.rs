use log::debug;

use crate::prelude::*;

use super::AugmentedPos;
use super::Player;
use super::attacks; // goal is to get rid of it

pub fn generate_king_dests(src: Bitboard<Square>, meta: &AugmentedPos) -> Bitboard<GenericBB> {
    let player = meta.turn;
    let king = src; // meta.p.pos[player].pieces[PieceN::King as usize];

    // squares that arent attacked by opponent, and not occupied by my own pieces
    log::trace!(
        "King gen: Other player attacks: {}, occupied {}",
        meta.attacked[player.other() as usize],
        meta.p.pos.occupied(player)
    );
    let free_sq_for_king =
        (!meta.attacked[player.other() as usize]) & (!meta.p.pos.occupied(player));

    let r = attacks::generate_king(king) & free_sq_for_king;
    debug!(
        "King dests for this turn {} -> {} \n (details) -- {} & {} ",
        src,
        r,
        attacks::generate_king(king),
        free_sq_for_king
    );
    r
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
