use crate::position::Rank;
use crate::position::bitboard::Bitboard;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Player {
    White,
    Black,
}

impl Player {
    pub const COUNT: usize = 2;
    pub const fn other(self) -> Player {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
        }
    }
    #[inline(always)]
    pub const fn backrank(self) -> Bitboard<Rank> {
        match self {
            Player::Black => Bitboard(Rank::R8),
            Player::White => Bitboard(Rank::R1),
        }
    }
    pub fn from_usize(x: usize) -> Player {
        match x {
            0 => Player::White,
            1 => Player::Black,
            _ => panic!("Unknown player sent p={x}"),
        }
    }
}
