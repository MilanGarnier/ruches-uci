use std::ops::Index;

use super::bitboard::{self, Bitboard, File, GenericBB};
use super::{Player, UciNotation};

#[derive(Copy, Clone)]
pub enum Castle {
    Short,
    Long,
}
impl Castle {
    const COUNT: usize = 2;
    pub const fn king_dest_file(&self) -> Bitboard<File> {
        match self {
            Self::Long => Bitboard(File::C),
            Self::Short => Bitboard(File::G),
        }
    }
    pub const fn files(&self) -> Bitboard<GenericBB> {
        match self {
            Self::Long => CASTLE_FILES_LONG,
            Self::Short => CASTLE_FILES_SHORT,
        }
    }
    pub const fn free_files(&self) -> Bitboard<GenericBB> {
        match self {
            Self::Long => CASTLE_FILES_LONG_FREE,
            Self::Short => CASTLE_FILES_SHORT_FREE,
        }
    }
    pub const fn rook_file(&self) -> Bitboard<File> {
        match self {
            Self::Short => Bitboard(File::H),
            Self::Long => Bitboard(File::A),
        }
    }
}
impl UciNotation for (&Castle, &Player) {
    fn to_uci(&self) -> String {
        match self.0 {
            Castle::Short => match self.1 {
                Player::Black => String::from("e8g8"),
                Player::White => String::from("e1g1"),
            },
            Castle::Long => match self.1 {
                Player::Black => String::from("e8c8"),
                Player::White => String::from("e1c1"),
            },
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CastleRights {
    pub x: [bool; Castle::COUNT],
}

trait Revertible<Commit>: Sized + Copy + PartialEq {
    // with stack() then unstack() the object has to remain unchanged
    fn stack(&mut self, c: &Commit);
    
    /*
    fn unstack(&mut self, c: &Commit) {
        self.stack(c);
    }*/
    /*
    // use this function only for debug purposes
    fn assert_safety(&mut self, c : &Commit) {
        let mut s1 = *self;
        s1.stack(c);
        s1.unstack(c);
    }*/
}
impl PartialEq for CastleRights {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x
    }
}

impl Revertible<CastleRights> for CastleRights {
    fn stack(&mut self, c: &CastleRights) {
        for (index, value) in c.x.iter().enumerate() {
            self.x[index] ^= value;
        }
    }
}

impl Index<Castle> for CastleRights {
    type Output = bool;
    #[inline(always)]
    fn index(&self, index: Castle) -> &Self::Output {
        &self.x[index as usize]
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CastleData {
    pub x: [CastleRights; Player::COUNT],
}

impl CastleData {
    pub fn stack_rev(&mut self, other: &CastleData) {
        for (index, value) in other.x.iter().enumerate() {
            self.x[index].stack(value);
        }
    }
    pub fn fetch(&self, p: Player, c: Castle) -> bool {
        self.x[p as usize][c]
    }
}

pub const CASTLE_ALLOWED_ONE_SIDE: CastleRights = CastleRights { x: [true, true] };
pub const CASTLE_FORBIDDEN_ONE_SIDE: CastleRights = CastleRights { x: [false, false] };

pub const CASTLES_ALL_ALLOWED: CastleData = CastleData {
    x: [CASTLE_ALLOWED_ONE_SIDE, CASTLE_ALLOWED_ONE_SIDE],
};
pub const CASTLES_ALL_FORBIDDEN: CastleData = CastleData {
    x: [CASTLE_FORBIDDEN_ONE_SIDE, CASTLE_FORBIDDEN_ONE_SIDE],
};

pub const CASTLES_KEEP_UNCHANGED: CastleData = CASTLES_ALL_FORBIDDEN;

pub const CASTLE_FILES_SHORT: bitboard::Bitboard<GenericBB> = Bitboard(GenericBB(
    File::E.bitboard() | File::F.bitboard() | File::G.bitboard(),
));
pub const CASTLE_FILES_LONG: bitboard::Bitboard<GenericBB> = Bitboard(GenericBB(
    File::C.bitboard() | File::D.bitboard() | File::E.bitboard(),
));

pub const CASTLE_FILES_SHORT_FREE: bitboard::Bitboard<GenericBB> =
    Bitboard(GenericBB(File::F.bitboard() | File::G.bitboard()));
pub const CASTLE_FILES_LONG_FREE: bitboard::Bitboard<GenericBB> = Bitboard(GenericBB(
    File::B.bitboard() | File::C.bitboard() | File::D.bitboard(),
));
