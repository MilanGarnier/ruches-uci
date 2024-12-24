// same spec as local vec but only uses
// can be used to shortcircuit localvec to troublehoot performance

use super::{Bitboard, GenericBB, Move, Piece, Square};
use std::{
    fmt::Debug,
    intrinsics::{likely, unlikely},
    mem::MaybeUninit,
};

// Pre move generation
// When computing attacks, stores them in a buffer so that they can be exploited later during move generation

// up to 15 th max entries that would go in a PregenCache, but practically 7 in most realistic cases
// we could assume the cost of one heap alloc once it goes over 7
pub type MoveEntry = (Piece, Bitboard<Square>, Bitboard<GenericBB>);

// if used for move generation
pub type MoveVec = PregenCache<60, Move>;
pub type RelevantAttacksVec = PregenCache<8, MoveEntry>;

pub     type PregenCache<const N: usize, EntryType: Copy> = Vec<EntryType>;
