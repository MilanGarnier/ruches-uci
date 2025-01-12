use enum_iterator::Sequence;

use super::bitboard::{Bitboard, File, Rank};
use super::{
    Player,
    bitboard::{GenericBB, SpecialBB, ToBB},
};

#[derive(Clone, Copy, PartialEq, Debug, Sequence)]
pub enum Piece {
    // neutral piece type
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}
impl Piece {
    pub const COUNT: usize = 6;
}
impl Piece {
    pub fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(Self::Pawn),
            1 => Some(Self::Knight),
            2 => Some(Self::King),
            3 => Some(Self::Bishop),
            4 => Some(Self::Rook),
            5 => Some(Self::Queen),
            _ => None,
        }
    }
}

impl Piece {
    // return ranks of start positions (combined in a bitboard)
    fn starting_files(self) -> Bitboard<GenericBB> {
        match self {
            Piece::Pawn => SpecialBB::Full.declass(), // all ranks
            Piece::Knight => File::B.declass() | File::G,
            Piece::Bishop => File::C.declass() | File::F,
            Piece::Rook => File::A.declass() | File::H,
            Piece::Queen => File::D.declass(),
            Piece::King => File::E.declass(),
        }
    }
    pub fn startingpos(self, pl: Player) -> Bitboard<GenericBB> {
        match self {
            Piece::Pawn => match pl {
                Player::White => Rank::R2.declass(),
                Player::Black => Rank::R7.declass(),
            },
            p => pl.backrank().declass() & p.starting_files(),
        }
    }
    pub const fn from_notation(c: char) -> Option<(Player, Piece)> {
        let piece = match c.to_ascii_lowercase() {
            'q' => Some(Piece::Queen),
            'n' => Some(Piece::Knight),
            'k' => Some(Piece::King),
            'b' => Some(Piece::Bishop),
            'r' => Some(Piece::Rook),
            'p' => Some(Piece::Pawn),
            _ => None,
        };
        let player = match c.is_uppercase() {
            true => Player::White,
            false => Player::Black,
        };
        match piece {
            None => None,
            Some(x) => Some((player, x)),
        }
    }
}
