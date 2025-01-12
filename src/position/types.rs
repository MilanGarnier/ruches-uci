use crate::position::Bitboard;
use crate::position::BitboardSpec;
use crate::position::GenericBB;
use crate::position::Piece;
use crate::position::Player;
use crate::position::Square;
use crate::prelude::*;
use std::ops::Index;

pub use pieceset::{DynPieceSet, PieceSet, PieceSetTr};

// Check if an item exists : TODO -> make an Iterator for it
pub trait Get<'a, I, O: Sized> {
    type Input = I;
    type Output = O;
    fn get(&'a self, index: I) -> Self::Output;
}

pub trait PlayerStorageSpec<'a>: Sized + Index<(Player, Piece)>
where
    Self: Sized,
    Self: Get<'a, Bitboard<Square>, Option<Piece>>,
    Self: Get<'a, (Player, Bitboard<Square>), Option<Piece>>,
    <Self as Index<(Player, Piece)>>::Output: BitboardSpec + IntoIterator,
{
    fn startingpos() -> Self;
    fn empty() -> Self;

    fn generate_attacks(&self, pl: Player) -> Bitboard<GenericBB>;
    fn get_pieceset(&self, index: Player) -> DynPieceSet;
    fn edit<T>(&mut self, index: Player, task: impl Fn(&mut DynPieceSet) -> T) -> T;
    fn map_reduce<R>(
        &self,
        task: impl Fn((Player, Piece, Bitboard<Square>)) -> R,
        reduce: impl Fn(R, R) -> R,
    ) -> Option<R>;

    fn zobrist(&self) -> usize {
        self.white().hash() ^ self.black().hash()
    }
    fn white_mut(&mut self) -> &mut PieceSet<WhiteS>;
    fn black_mut(&mut self) -> &mut PieceSet<BlackS>;

    fn white(&self) -> &PieceSet<WhiteS>;
    fn black(&self) -> &PieceSet<BlackS>;

    fn occupied(&self, player: Player) -> Bitboard<GenericBB> {
        match player {
            Player::Black => self.black().occupied(),
            Player::White => self.white().occupied(),
        }
    }

    fn add_new_piece(&mut self, pl: Player, index: Piece, sq: Bitboard<Square>) {
        match pl {
            Player::White => self.white_mut().add_new_piece(index, sq),
            Player::Black => self.black_mut().add_new_piece(index, sq),
        }
    }
    fn remove_piece(&mut self, pl: Player, index: Piece, sq: Bitboard<Square>) {
        match pl {
            Player::White => self.white_mut().remove_piece(index, sq),
            Player::Black => self.black_mut().remove_piece(index, sq),
        }
    }
    fn move_piece(
        &mut self,
        pl: Player,
        index: Piece,
        src: Bitboard<Square>,
        dest: Bitboard<Square>,
    );
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerStorage {
    pub black: PieceSet<BlackS>,
    pub white: PieceSet<WhiteS>,
}

//// Internal defs

mod pieceset {
    pub enum DynPieceSet<'a> {
        White(&'a PieceSet<WhiteS>),
        Black(&'a PieceSet<BlackS>),
    }

    use std::{marker::PhantomData, ops::Index};

    use crate::{
        position::zobrist::{ZOBRIST_SEED, zobrist_hash_square},
        prelude::*,
    };
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct PieceSet<T: ColorTr> {
        pawns: Bitboard<GenericBB>,
        knights: Bitboard<GenericBB>,
        bishops: Bitboard<GenericBB>,
        rooks: Bitboard<GenericBB>,
        queens: Bitboard<GenericBB>,
        king: Bitboard<GenericBB>, // TODO: move to squares

        occupied: Bitboard<GenericBB>,
        hash: usize,
        _side: PhantomData<T>,
    }
    pub trait PieceSetTr: Sized + Index<Piece>
    where
        <Self as Index<Piece>>::Output: BitboardSpec,
    {
        fn startingpos() -> Self;
        fn empty() -> Self;

        fn occupied(&self) -> Bitboard<GenericBB>;
        fn hash(&self) -> usize;
        fn attacks(&self, blockers: Bitboard<GenericBB>) -> Bitboard<GenericBB>;

        fn add_new_piece(&mut self, index: Piece, sq: Bitboard<Square>);
        fn remove_piece(&mut self, index: Piece, sq: Bitboard<Square>);
        fn move_piece(&mut self, index: Piece, src: Bitboard<Square>, dest: Bitboard<Square>);

        fn get_square(&self, square: Bitboard<Square>) -> Option<Piece>;
    }

    impl<T: ColorTr> Index<Piece> for PieceSet<T> {
        type Output = Bitboard<GenericBB>;
        #[inline(always)]
        fn index(&self, index: Piece) -> &Self::Output {
            match index {
                Piece::Pawn => &self.pawns,
                Piece::King => &self.king,
                Piece::Bishop => &self.bishops,
                Piece::Rook => &self.rooks,
                Piece::Knight => &self.knights,
                Piece::Queen => &self.queens,
            }
        }
    }

    impl<T: ColorTr> PieceSetTr for PieceSet<T> {
        fn occupied(&self) -> Bitboard<GenericBB> {
            self.occupied
        }
        fn hash(&self) -> usize {
            self.hash
        }

        fn attacks(&self, blockers: Bitboard<GenericBB>) -> Bitboard<GenericBB> {
            movegen::attacks::generate_pawns(self[Piece::Pawn], T::side())
                | movegen::attacks::generate_knights(self[Piece::Knight])
                | movegen::attacks::generate_bishops(
                    self[Piece::Bishop] | self[Piece::Queen],
                    blockers,
                )
                | movegen::attacks::generate_rooks(self[Piece::Rook] | self[Piece::Queen], blockers)
                | movegen::attacks::generate_king(Square::from_bb(&self[Piece::King]).unwrap())
        }

        fn add_new_piece(&mut self, index: Piece, sq: Bitboard<Square>) {
            if cfg!(debug_assertions) && self.occupied() & sq != SpecialBB::Empty.declass() {
                log::warn!("Tried adding piece where it is not authorized");
            }
            self.edit(index, sq);
        }

        fn remove_piece(&mut self, index: Piece, sq: Bitboard<Square>) {
            if cfg!(debug_assertions) && self.occupied() & sq == SpecialBB::Empty.declass() {
                log::warn!("Tried removing piece where it is not authorized {index:?} - {sq}");
            };
            self.edit(index, sq);
        }

        fn move_piece(&mut self, index: Piece, src: Bitboard<Square>, dest: Bitboard<Square>) {
            self.remove_piece(index, src);
            self.add_new_piece(index, dest);
        }

        fn startingpos() -> Self {
            let p = T::side();
            PieceSet {
                pawns: Piece::Pawn.startingpos(p),
                knights: Piece::Knight.startingpos(p),
                bishops: Piece::Bishop.startingpos(p),
                rooks: Piece::Rook.startingpos(p),
                queens: Piece::Queen.startingpos(p),
                king: Piece::King.startingpos(p),
                occupied: p.backrank() | (p.backrank() + 1) | (p.backrank() - 1),
                hash: enum_iterator::all::<Piece>()
                    .map(|piece| -> usize {
                        {
                            let bb = piece.startingpos(p);
                            let mut hash = 0;
                            for sq in bb {
                                hash ^= ZOBRIST_SEED[sq.to_index() as usize][piece as usize]
                                    [p as usize];
                            }
                            hash
                        }
                    })
                    .reduce(|x, y| -> usize { x ^ y })
                    .unwrap(),
                _side: PhantomData::default(),
            }
        }

        fn empty() -> Self {
            PieceSet {
                pawns: SpecialBB::Empty.declass(),
                knights: SpecialBB::Empty.declass(),
                bishops: SpecialBB::Empty.declass(),
                rooks: SpecialBB::Empty.declass(),
                queens: SpecialBB::Empty.declass(),
                king: SpecialBB::Empty.declass(),
                occupied: SpecialBB::Empty.declass(),
                hash: 0,
                _side: PhantomData::default(),
            }
        }

        fn get_square(&self, square: Bitboard<Square>) -> Option<Piece> {
            enum_iterator::all::<Piece>()
                .find(|p| -> bool { self[*p] & square != SpecialBB::Empty.declass() })
        }
    }

    // private methods
    impl<T: ColorTr> PieceSet<T> {
        fn index_mut(&mut self, index: Piece) -> &mut Bitboard<GenericBB> {
            match index {
                Piece::Pawn => &mut self.pawns,
                Piece::King => &mut self.king,
                Piece::Bishop => &mut self.bishops,
                Piece::Rook => &mut self.rooks,
                Piece::Knight => &mut self.knights,
                Piece::Queen => &mut self.queens,
            }
        }
        fn edit(&mut self, index: Piece, sq: Bitboard<Square>) {
            let a = self.index_mut(index);
            *a ^= sq;
            self.occupied ^= sq;
            self.hash ^= zobrist_hash_square(sq, index, T::side());
        }
    }
}

impl Index<(Player, Piece)> for PlayerStorage {
    type Output = Bitboard<GenericBB>;

    fn index(&self, (player, piece): (Player, Piece)) -> &Self::Output {
        match player {
            Player::Black => &self.black[piece],
            Player::White => &self.white[piece],
        }
    }
}

impl<'a> Get<'a, Player, DynPieceSet<'a>> for PlayerStorage {
    fn get(&'a self, player: Player) -> DynPieceSet<'a> {
        match player {
            Player::Black => DynPieceSet::Black(&self.black),
            Player::White => DynPieceSet::White(&self.white),
        }
    }
}

impl Get<'_, (Player, Bitboard<Square>), Option<Piece>> for PlayerStorage {
    fn get(&self, (player, square): (Player, Bitboard<Square>)) -> Option<Piece> {
        let a: DynPieceSet = self.get(player);
        match a {
            DynPieceSet::Black(x) => x.get_square(square),
            DynPieceSet::White(x) => x.get_square(square),
        }
    }
}

impl Get<'_, (Piece, Bitboard<Square>), Option<Piece>> for PlayerStorage {
    fn get(&self, (piece, square): (Piece, Bitboard<Square>)) -> Option<Piece> {
        let p1 = self.black.get_square(square);
        let p2 = self.white.get_square(square);
        p1.or(p2).take_if(|x| *x == piece)
    }
}

impl Get<'_, Bitboard<Square>, Option<Piece>> for PlayerStorage {
    fn get(&self, index: Bitboard<Square>) -> Self::Output {
        let a = |pl| -> Option<Piece> {
            <Self as Get<(Player, Bitboard<Square>), Option<Piece>>>::get(self, (pl, index))
        };
        let b = enum_iterator::all::<Player>().find_map(a);
        b
    }
}

impl<'a> PlayerStorageSpec<'a> for PlayerStorage {
    fn get_pieceset(&self, index: Player) -> DynPieceSet {
        match index {
            Player::Black => DynPieceSet::Black(&self.black),
            Player::White => DynPieceSet::White(&self.white),
        }
    }

    fn generate_attacks(&self, pl: Player) -> Bitboard<GenericBB> {
        match pl {
            Player::Black => self.black.attacks(self.white.occupied()),
            Player::White => self.white.attacks(self.black.occupied()),
        }
    }

    fn edit<T>(&mut self, index: Player, task: impl Fn(&mut DynPieceSet) -> T) -> T {
        match index {
            Player::Black => {
                let mut set = DynPieceSet::Black(&mut self.black);
                task(&mut set)
            }
            Player::White => {
                let mut set = DynPieceSet::White(&self.white);
                task(&mut set)
            }
        }
    }

    fn startingpos() -> Self {
        Self {
            white: PieceSet::startingpos(),
            black: PieceSet::startingpos(),
        }
    }
    fn empty() -> Self {
        Self {
            white: PieceSet::empty(),
            black: PieceSet::empty(),
        }
    }

    fn map_reduce<R>(
        &self,
        task: impl Fn((Player, Piece, Bitboard<Square>)) -> R,
        reduce: impl Fn(R, R) -> R,
    ) -> Option<R> {
        let enum_squares = |(player, piece)| {
            self[(player, piece)]
                .into_iter()
                .map(move |sq| (player, piece, sq))
        };
        use enum_iterator::all;
        let triple_iter = all::<Player>()
            .zip(all::<Piece>())
            .flat_map(enum_squares)
            .map(task);
        let reduced = triple_iter.reduce(reduce);
        reduced
    }

    fn zobrist(&self) -> usize {
        self.black.hash() ^ self.white.hash()
    }

    fn white(&self) -> &PieceSet<WhiteS> {
        &self.white
    }

    fn black(&self) -> &PieceSet<BlackS> {
        &self.black
    }

    fn white_mut(&mut self) -> &mut PieceSet<WhiteS> {
        &mut self.white
    }

    fn black_mut(&mut self) -> &mut PieceSet<BlackS> {
        &mut self.black
    }
    #[warn(soft_unstable)]
    fn move_piece(
        &mut self,
        pl: Player,
        index: Piece,
        src: Bitboard<Square>,
        dest: Bitboard<Square>,
    ) {
        log::warn!("Called unstable function in move generation.");
        match pl {
            Player::Black => self.black.move_piece(index, src, dest),
            Player::White => self.white.move_piece(index, src, dest),
        }
    }
}
