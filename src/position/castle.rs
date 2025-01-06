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

// store castle data for both players
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CastleData {
    // x: [CastleRights; Player::COUNT], // former representation, not memory efficient
    x: u8,
}

impl CastleData {
    pub fn stack_rev(&mut self, other: &CastleData) {
        /*for (index, value) in other.x.iter().enumerate() {
            self.x[index].stack(value);
        }*/
        self.x ^= other.x
    }
    pub fn fetch(&self, p: Player, c: Castle) -> bool {
        let mask: u8 = 1 << (Castle::COUNT * (p as usize) + c as usize);
        self.x & mask != 0
    }

    pub fn set(&mut self, p: Player, c: Castle, val: bool) {
        let mask: u8 = 1 << (Castle::COUNT * (p as usize) + c as usize);
        if val {
            self.x |= mask;
        } else {
            self.x &= !mask
        }
    }

    pub fn copy_selection_player(&mut self, p: Player, val: &Self) {
        let mask = (((1 as u8) << Castle::COUNT) - 1) << Castle::COUNT * (p as usize);
        self.x = (self.x & !mask) | (mask & val.x);
    }
    pub fn copy_selection_precise(&mut self, p: Player, c: Castle, val: &Self) {
        let mask: u8 = 1 << (Castle::COUNT * (p as usize) + c as usize);
        self.x = (self.x & !mask) | (mask & val.x);
    }

    pub fn hash(&self) -> usize {
        // TODO: improve speed
        /*let mut h = 0;
        for b in self.x {
            for b in b.x {
                h *= 2;
                if b {
                    h += 1;
                }
            }
        }
        h*/
        self.x as usize * 98466746843 // magic value
    }
}

pub const CASTLE_ALLOWED_ONE_SIDE: CastleRights = CastleRights { x: [true, true] };
pub const CASTLE_FORBIDDEN_ONE_SIDE: CastleRights = CastleRights { x: [false, false] };

pub const CASTLES_ALL_ALLOWED: CastleData = CastleData { x: 0xF };
pub const CASTLES_ALL_FORBIDDEN: CastleData = CastleData { x: 0x0 };

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
