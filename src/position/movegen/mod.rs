use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::pin::Pin;

use crate::localvec::{self, FastVec, MoveVec};

use dests::{generate_next_en_passant_data, mask};
use log::Level;

use super::bitboard::{
    self, Bitboard, File, FromBB, GenericBB, PackedSquare, Rank, SpecialBB, Square, ToBB,
};
use super::castle::{self, CASTLES_KEEP_UNCHANGED, Castle, CastleData};
use super::piece::Piece;
use super::{Player, PlayerStorage};

use super::{PieceSet, Position};

pub mod attacks;
mod dests;
// mod heapvec;

#[derive(Clone, Copy, Debug)]
pub struct Change {
    p: Piece,
    cap: Option<Piece>,
    dest: Bitboard<PackedSquare>,
    from: Bitboard<PackedSquare>,
}
impl Change {
    pub fn encode(
        p: Piece,
        cap: Option<Piece>,
        dest: &Bitboard<Square>,
        from: &Bitboard<Square>,
    ) -> Self {
        Self {
            p,
            cap,
            from: Bitboard::<PackedSquare>::from(from),
            dest: Bitboard::<PackedSquare>::from(dest),
        }
    }
    pub fn piece(&self) -> Piece {
        self.p
    }
    pub fn bitboard(&self) -> Bitboard<GenericBB> {
        self.dest | self.from
        //Bitboard::<Square>::generic_from_index(self.dest)
        //    | Bitboard::<Square>::generic_from_index(self.from)
    }
    pub fn dest(&self) -> Bitboard<Square> {
        //Bitboard::<Square>::from_index(self.dest)
        self.dest.into()
    }

    pub fn from(&self) -> Bitboard<Square> {
        //Bitboard::<Square>::from_index(self.from)
        self.from.into()
    }
    pub fn cap(&self) -> Option<Piece> {
        self.cap
    }
}
impl Display for Change {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            self.from,
            self.dest //Bitboard::<Square>::from_index(self.from),
                      //Bitboard::<Square>::from_index(self.dest)
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Promotion {
    data: Change, // simply a change but interpreted in a different way
}

impl Promotion {
    pub fn encode(
        new_p: &Piece,
        cap: Option<Piece>,
        dest: &Bitboard<Square>,
        from: &Bitboard<Square>,
    ) -> Self {
        Self {
            data: Change::encode(*new_p, cap, dest, from),
        }
    }
    pub fn new_piece(&self) -> Piece {
        self.data.p
    }
    pub fn cap(&self) -> Option<Piece> {
        self.data.cap
    }
    pub fn from(&self) -> Bitboard<Square> {
        self.data.from()
    }
    pub fn dest(&self) -> Bitboard<Square> {
        self.data.dest()
    }
}
impl Display for Promotion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = ['p', 'n', 'b', 'r', 'q', 'k'];
        write!(f, "{}{}", self.data, repr[self.new_piece() as usize])
    }
}
impl Promotion {}

#[derive(Clone, Copy, Debug)]
pub enum AtomicMove {
    PieceMoved(Change),
    PiecePromoted(Promotion),
}
impl AtomicMove {
    pub fn does_affect(&self, piece: Piece) -> bool {
        match self {
            AtomicMove::PiecePromoted(prom) => {
                (piece == Piece::Pawn) || (piece == prom.new_piece())
            }
            AtomicMove::PieceMoved(mv) => mv.p == piece,
        }
    }
    pub fn dest(&self) -> Bitboard<Square> {
        match self {
            AtomicMove::PieceMoved(m) => m.dest(),
            AtomicMove::PiecePromoted(p) => p.dest(),
        }
    }
    pub fn src(&self) -> Bitboard<Square> {
        match self {
            AtomicMove::PieceMoved(m) => m.from(),
            AtomicMove::PiecePromoted(p) => p.from(),
        }
    }
    pub fn cap(&self) -> bool {
        match self {
            AtomicMove::PieceMoved(x) => !(x.cap == None),
            AtomicMove::PiecePromoted(x) => !(x.cap() == None),
        }
    }
}
impl Display for AtomicMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomicMove::PieceMoved(x) => write!(f, "{x}"),
            AtomicMove::PiecePromoted(x) => write!(f, "{x}"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct StandardMove {
    pub mv: AtomicMove,
    pub cas: CastleData,
}
impl StandardMove {
    pub fn is_moved(&self, piece: Piece) -> bool {
        return self.mv.does_affect(piece);
    }
}

impl Display for StandardMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.mv)
    }
}

#[derive(Clone, Copy)]
pub enum PartialMove {
    Std(StandardMove),
    Castle(Castle, Player, CastleData),
}

impl PartialMove {
    fn dest(&self) -> Bitboard<Square> {
        match self {
            PartialMove::Std(s) => s.mv.dest(),
            PartialMove::Castle(c, p, _cd) => {
                let dst_sq = c.king_dest_file() & p.backrank();
                Square::from_bb(&dst_sq).unwrap()
            }
        }
    }
    fn src(&self) -> Bitboard<Square> {
        match self {
            PartialMove::Std(s) => s.mv.src(),
            PartialMove::Castle(_c, p, _cd) => {
                Bitboard(File::E) & p.backrank() //*Square::from_bb(&(Bitboard(File::E) & p.backrank())).unwrap()
            }
        }
    }
    pub fn is_capture(&self) -> bool {
        match self {
            PartialMove::Std(x) => x.mv.cap(),
            _ => false,
        }
    }
    pub const fn is_promotion(&self) -> bool {
        match self {
            PartialMove::Std(x) => match x.mv {
                AtomicMove::PiecePromoted(_) => true,
                _ => false,
            },
            _ => false,
        }
    }
    pub const fn is_castle(&self) -> bool {
        match self {
            PartialMove::Castle(_, _, _) => true,
            _ => false,
        }
    }
    pub fn is_moved(&self, p: Piece) -> bool {
        match p {
            Piece::King => match self {
                PartialMove::Castle(_, _, _) => true,
                PartialMove::Std(x) => x.is_moved(p),
            },
            Piece::Rook => match self {
                PartialMove::Castle(_, _, _) => true,
                PartialMove::Std(x) => x.is_moved(p),
            },
            _ => match self {
                PartialMove::Castle(_, _, _) => false,
                PartialMove::Std(x) => x.is_moved(p),
            },
        }
    }
}

impl Display for PartialMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PartialMove::Castle(x, pl, _) => match x {
                Castle::Short => match pl {
                    Player::Black => writeln!(f, "e8g8"),
                    Player::White => writeln!(f, "e1g1"),
                },
                Castle::Long => match pl {
                    Player::Black => write!(f, "e8c8"),
                    Player::White => write!(f, "e1c1"),
                },
            },
            PartialMove::Std(x) => write!(f, "{x}"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Move<'a> {
    // Full move data
    pm: PartialMove,
    fifty_mv: u16, // either 0 if this move is eligible, or the nb of half moves before it happened
    en_passant: Bitboard<GenericBB>,
    _pos: PhantomData<&'a Position>, // move is bound to a position
}

impl<'a> Move<'a> {
    pub const fn partialmove(&'a self) -> &'a PartialMove {
        &self.pm
    }
    pub const fn fifty_mv(&self) -> u16 {
        self.fifty_mv
    }
    pub const fn en_passant(&self) -> Bitboard<GenericBB> {
        self.en_passant
    }

    pub fn is_capture(&self) -> bool {
        self.pm.is_capture()
    }
    pub const fn is_promotion(&self) -> bool {
        self.pm.is_promotion()
    }
    pub const fn is_castle(&self) -> bool {
        self.pm.is_castle()
    }
    pub fn is_moved(&self, p: Piece) -> bool {
        self.pm.is_moved(p)
    }
    pub fn dest(&self) -> Bitboard<Square> {
        self.pm.dest()
    }
    pub fn src(&self) -> Bitboard<Square> {
        self.pm.src()
    }
}
impl<'a> Display for Move<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pm)
    }
}

impl<'a> Debug for Move<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = write!(f, "{}", self);
        Ok(())
    }
}

pub struct MoveIter<'a> {
    array: &'a [Option<Move<'a>>; 256],
    index: usize,
}
impl<'a> MoveIter<'a> {
    pub fn create(array: &'a [Option<Move<'a>>; 256]) -> Self {
        MoveIter { array, index: 0 }
    }
}
impl<'a> Iterator for MoveIter<'a> {
    type Item = &'a Move<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.index {
            256 => None,
            _ => {
                let x = &self.array[self.index];
                match x {
                    Some(m) => {
                        self.index += 1;
                        Some(m)
                    }
                    None => None, // stop at first "null Move"
                                  //{ self.index += 1; self.next() }
                }
            }
        }
    }
}

fn which_piece_on_sq(sq: Bitboard<Square>, pos: &PieceSet) -> Option<Piece> {
    if (pos[Piece::Pawn] & sq) != SpecialBB::Empty.declass() {
        Some(Piece::Pawn)
    } else if (pos[Piece::Knight] & sq) != SpecialBB::Empty.declass() {
        Some(Piece::Knight)
    } else if (pos[Piece::Bishop] & sq) != SpecialBB::Empty.declass() {
        Some(Piece::Bishop)
    } else if (pos[Piece::Rook] & sq) != SpecialBB::Empty.declass() {
        Some(Piece::Rook)
    } else if (pos[Piece::Queen] & sq) != SpecialBB::Empty.declass() {
        Some(Piece::Queen)
    } else if (pos[Piece::King] & sq) != SpecialBB::Empty.declass() {
        Some(Piece::King)
    } else {
        None
    }
}

// dest has to point to a single square
fn generate_capture_data(meta: &AugmentedPos, dest: Bitboard<Square>, p: Piece) -> Option<Piece> {
    // TODO: #[cfg(debug_assertions)] - single byte
    /*if (dest.declass() & (meta.get_occupied()[meta.opponent()] | meta.p.en_passant))
        != SpecialBB::Empty.declass()
    */
    match which_piece_on_sq(dest, meta.semi_pos(meta.opponent())) {
        None => match p {
            // ghost capture, either en passant or nothing
            Piece::Pawn => match meta.p.en_passant & dest != SpecialBB::Empty.declass() {
                true => Some(Piece::Pawn),
                false => None,
            },
            // {println!("En passant detected but not properly handled yet"); meta.p.pretty_print(); todo!()},
            _ => None, // only a piece going behind a pawn which just moved up 2 squares
        },
        //todo!(), // en passant !!!,
        Some(x) => Some(x),
    }
}

// returns the number of atomic moves associated, typically 1 or 6
pub fn generate_non_promoting_atmove(
    meta: &AugmentedPos,
    src: &Bitboard<Square>,
    dest: &Bitboard<Square>,
    piece: &Piece,
) -> AtomicMove {
    AtomicMove::PieceMoved(Change::encode(
        *piece,
        generate_capture_data(meta, *dest, *piece),
        dest,
        src,
    ))
}

// cannot be called on a castle move
pub fn generate_castle_data(
    meta: &AugmentedPos,
    src: &Bitboard<Square>,
    dest: &Bitboard<Square>,
    piece: &Piece,
) -> CastleData {
    let mut cd: CastleData = CASTLES_KEEP_UNCHANGED; // eveything to false
    match piece {
        // reset castles for king/rook moves
        Piece::King => cd.copy_selection_player(meta.player(), &meta.p.castles),
        Piece::Rook => {
            if src.declass() == (File::A.declass() & meta.player().backrank()) {
                cd.copy_selection_precise(meta.player(), Castle::Long, &meta.p.castles)
            } else if src.declass() == (File::H.declass() & meta.player().backrank()) {
                cd.copy_selection_precise(meta.player(), Castle::Short, &meta.p.castles)
            }
        }
        _ => (),
    }
    // capture opponent rook
    if dest.declass() == (File::A.declass() & meta.opponent().backrank()) {
        cd.copy_selection_precise(meta.opponent(), Castle::Long, &meta.p.castles)
    } else if dest.declass() == (File::H.declass() & meta.opponent().backrank()) {
        cd.copy_selection_precise(meta.opponent(), Castle::Short, &meta.p.castles)
    }

    cd
}

// return in [0/1/2]
pub fn generate_castle_move<'a>(meta: &AugmentedPos<'a>, c: castle::Castle) -> Option<Move<'a>> {
    if meta.p.castles.fetch(meta.player(), c)  // right to castle
    &&  meta.attacked[meta.opponent()] & (c.files() & meta.player().backrank()) == SpecialBB::Empty.declass()  // no attacks on path (including check)
    &&  (meta.occupied[meta.player()] | meta.occupied[meta.opponent()]) & (c.free_files() & meta.player().backrank()) == SpecialBB::Empty.declass()
    // no piece on rook&king path
    {
        // forbid current player castles
        let mut new_cas_data = CASTLES_KEEP_UNCHANGED;
        new_cas_data.copy_selection_player(meta.player(), &meta.p.castles);

        let mv = Move {
            pm: PartialMove::Castle(c, meta.player(), new_cas_data),
            fifty_mv: 0,
            en_passant: meta.p.en_passant,
            _pos: PhantomData::default(),
        };
        Some(mv)
    } else {
        None
    }
}

fn is_legal(p: &Position, mv: &Move) -> bool {
    unsafe {
        let a = p as *const Position as *mut Position;
        let p = a.as_mut_unchecked();
        p.stack(&mv); // modifies p
        let t = !AugmentedPos::is_illegal(&p);
        p.unstack(&mv);
        t
    }
}

// WARNING modified to generate pseudolegal moves as well
// return future index
fn add_to_move_list<'a>(p: &AugmentedPos, m: Move<'a>, movelist: &mut MoveVec<'a>) {
    let pinned = (m.src().declass() & p.pinned) != SpecialBB::Empty.declass();
    // if src is pinned and moves to a destination not pinned it will be illegal anyway
    let pinned_dst = m.dest().declass() & p.pinned != SpecialBB::Empty.declass();
    let is_check = p.is_check();
    let affect_ep = !(m.en_passant == SpecialBB::Empty.declass());

    let mut edge_case = false;
    trace!("add_to_move_list({} {})", m.src(), m.dest());
    // let mut known_illegal = false;

    // if moving a pinned piece out of the pinned lines
    if pinned {
        edge_case = true;
        if !pinned_dst {
            // no way to know for sure actually :(
        }
    }

    if affect_ep && m.en_passant & p.pinned != SpecialBB::Empty.declass() {
        edge_case = true;
    }

    // in check but not moving king blocking pins nor capturing source
    if is_check && !m.partialmove().is_moved(Piece::King) {
        edge_case = true;
        if !pinned_dst && !m.is_capture() {
            // known_illegal = true;
            //return;
        }
    }
    if !edge_case || is_legal(p.p, &m) {
        movelist.push(m);
    }
}

fn non_pawn_move_iter_multiple_sources<'a>(
    meta: &AugmentedPos<'a>,
    piece: &Piece,
    src: &Bitboard<GenericBB>,
    mask: Bitboard<GenericBB>,
) -> (
    Bitboard<GenericBB>,
    impl Iterator<Item = impl Iterator<Item = Move<'a>>>,
) {
    let mut attacks = SpecialBB::Empty.declass();
    let iterator = src.map(move |s| {
        let a = non_pawn_move_iter(meta, piece, s, mask);
        attacks |= a.0;
        a.1
    });
    (attacks, iterator)
}

// Iterator for non_pawn_moves
fn non_pawn_move_iter<'a>(
    meta: &AugmentedPos<'a>,
    piece: &Piece,
    src: Bitboard<Square>,
    mask: Bitboard<GenericBB>,
) -> (Bitboard<GenericBB>, impl Iterator<Item = Move<'a>>) {
    let attacks = attacks::generate(meta, meta.turn, *piece, src);
    let dests = attacks & mask;
    let a = dests.map(move |sq| {
        let atomic_move = generate_non_promoting_atmove(meta, &src, &sq, piece);
        {
            let pm = PartialMove::Std(StandardMove {
                mv: atomic_move,
                cas: generate_castle_data(meta, &src, &sq, piece),
            });

            let fifty_mv = match pm.is_capture() | (*piece == Piece::Pawn) {
                true => meta.p.fifty_mv,
                _ => 0,
            };

            let en_passant = meta.p.en_passant
                ^ dests::generate_next_en_passant_data(*piece, src, sq, meta.player());

            Move {
                pm,
                fifty_mv,
                en_passant,
                _pos: PhantomData::default(),
            }
        }
    });
    (attacks, a)
}

pub fn generate_pawn_atmove<'a>(
    meta: &'a AugmentedPos<'a>,
    src: Bitboard<Square>,
    dest: Bitboard<Square>,
    piece: Piece,
) -> impl Iterator<Item = AtomicMove> {
    let is_prom = (piece as usize == Piece::Pawn as usize)
        && (dest.declass() & (Rank::R1.declass() | Rank::R8.declass()))
            != SpecialBB::Empty.declass();

    let iter = match is_prom {
        false => [Piece::Pawn].iter(),
        true => [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen].iter(),
    };

    iter.map(move |p| match p {
        &Piece::Pawn => generate_non_promoting_atmove(meta, &src, &dest, &piece),
        _ => AtomicMove::PiecePromoted(Promotion::encode(
            p,
            generate_capture_data(meta, dest, piece),
            &dest,
            &src,
        )),
    })
}

fn generate_pawn_move_data<'a>(
    meta: &AugmentedPos<'a>,
    src: Bitboard<Square>,
) -> impl Iterator<Item = impl Iterator<Item = Move<'a>>> {
    let blockers = meta.occupied[Player::White] | meta.occupied[Player::Black];
    let captures = meta.p.en_passant | meta.occupied[meta.opponent()];
    let dests = (attacks::generate_pawns(src.declass(), meta.turn) & captures)
        | dests::pawn_move_up_nocap(src, meta.player(), blockers);

    dests.map(move |sq| {
        let cas = generate_castle_data(meta, &src, &sq, &Piece::Pawn);
        let en_passant =
            generate_next_en_passant_data(Piece::Pawn, src, sq, meta.turn) ^ meta.p.en_passant;
        generate_pawn_atmove(meta, src, sq, Piece::Pawn).map(move |outcome| {
            let pm = PartialMove::Std(StandardMove { mv: outcome, cas });

            let m = Move {
                pm,
                fifty_mv: meta.p.fifty_mv,
                en_passant,
                _pos: PhantomData::default(),
            };

            m
        })
    })
}

// structure containing a
#[derive(Debug)]
pub struct AugmentedPos<'a> {
    p: &'a Position, // mutable to allow to simulate pseudo legal moves, but will always return it unchanged
    turn: Player,
    blockers: Bitboard<GenericBB>,
    occupied: PlayerStorage<Bitboard<GenericBB>>,
    attacked: PlayerStorage<Bitboard<GenericBB>>,
    pinned: Bitboard<GenericBB>,
}

impl<'a> AugmentedPos<'a> {
    // would return an error if the position is illegal already
    pub fn list_issues(p: &'a Position) -> Result<MoveVec<'a>, ()> {
        let turn = Player::from_usize((p.half_move_count % 2).into());
        let mut a: AugmentedPos<'a> = AugmentedPos {
            p: p,
            occupied: PlayerStorage::from([SpecialBB::Empty.declass(), SpecialBB::Empty.declass()]),
            attacked: PlayerStorage::from([SpecialBB::Empty.declass(), SpecialBB::Empty.declass()]),
            pinned: SpecialBB::Empty.declass(),
            turn: turn,
            blockers: SpecialBB::Empty.declass(),
        };
        a.compute_occupied();
        a.compute_pinned();

        a.compute_attacked_gen_moves()
    }

    pub fn check_legal(p: &Position) -> Result<(), ()> {
        let turn = Player::from_usize((p.half_move_count % 2).into());
        let mut a = AugmentedPos {
            p: p,
            occupied: PlayerStorage::from([SpecialBB::Empty.declass(), SpecialBB::Empty.declass()]),
            attacked: PlayerStorage::from([SpecialBB::Empty.declass(), SpecialBB::Empty.declass()]),
            pinned: SpecialBB::Empty.declass(),
            turn: turn,
            blockers: SpecialBB::Empty.declass(),
        };

        a.compute_occupied();

        let blockers = a.occupied[Player::White] | a.occupied[Player::Black];

        a.attacked[a.turn] = a.p.pos[a.turn].attacks(a.turn, blockers);
        if a.p.pos[a.turn.other()][Piece::King] & a.attacked[a.turn] != SpecialBB::Empty.declass() {
            return Err(());
        } else {
            return Ok(());
        }
    }

    pub const fn get_attacked(&self) -> &PlayerStorage<Bitboard<GenericBB>> {
        &self.attacked
    }
    pub const fn get_occupied(&self) -> &PlayerStorage<Bitboard<GenericBB>> {
        &self.occupied
    }
    pub const fn player(&self) -> Player {
        self.turn
    }
    pub const fn opponent(&self) -> Player {
        self.turn.other()
    }
    pub fn semi_pos(&self, p: Player) -> &PieceSet {
        &self.p.pos[p]
    }

    fn compute_occupied(&mut self) {
        self.occupied[Player::White] = self.p.pos[Player::White].occupied();
        self.occupied[Player::Black] = self.p.pos[Player::Black].occupied();
        self.blockers = self.occupied[Player::White] | self.occupied[Player::Black]
    }

    // assume occupied squares and "pins" are known
    fn compute_attacked_gen_moves(mut self) -> Result<localvec::MoveVec<'a>, ()> {
        let mut movelist: FastVec<64, Move<'a>> = localvec::MoveVec::new();
        let blockers = self.occupied[Player::White] | self.occupied[Player::Black];

        self.attacked[self.turn.other()] =
            self.p.pos[self.turn.other()].attacks(self.turn.other(), blockers);

        // generate my attacks and my moves at the same time
        self.attacked[self.turn] = {
            let mut attacked = SpecialBB::Empty.declass();

            let mask: Bitboard<GenericBB> = !self.occupied[self.turn];

            let iterator = [Piece::Bishop, Piece::Queen, Piece::Rook, Piece::Knight]
                .iter()
                .map(|piece| {
                    non_pawn_move_iter_multiple_sources(
                        &self,
                        piece,
                        &self.p.pos[self.turn][*piece],
                        mask,
                    )
                });

            for (attacks, x) in iterator {
                attacked |= attacks;
                for i in x {
                    for m in i {
                        movelist.push(m);
                    }
                }
            }

            attacked =
                attacked | attacks::generate_pawns(self.p.pos[self.turn][Piece::Pawn], self.turn);
            attacked
        };
        if self.p.pos[self.turn.other()][Piece::King] & self.attacked[self.turn]
            != SpecialBB::Empty.declass()
        {
            return Err(());
        }
        {
            let i1 =
                self.p.pos[self.turn][Piece::Pawn].map(|pawn| generate_pawn_move_data(&self, pawn));
            for i in i1 {
                for i in i {
                    for m in i {
                        add_to_move_list(&self, m, &mut movelist);
                    }
                }
            }
        }

        {
            // king moves
            let king = match Square::from_bb(&self.p.pos[self.turn][Piece::King]) {
                Some(x) => x,
                None => panic!("where is da king"),
            };
            let mask = mask(Piece::King, &self);
            let (_, i) = non_pawn_move_iter(&self, &Piece::King, king, mask);
            for m in i {
                add_to_move_list(&self, m, &mut movelist);
            }
        }

        {
            // castles

            if let Some(c) = generate_castle_move(&self, castle::Castle::Short) {
                add_to_move_list(&self, c, &mut movelist)
            }

            if let Some(c) = generate_castle_move(&self, castle::Castle::Long) {
                add_to_move_list(&self, c, &mut movelist)
            }
        }
        Pin::new(&self);

        return Ok(movelist);
    }

    // doesn't need to know attacked pieces
    fn compute_pinned(&mut self) {
        // let en_passant_pawns = (super::bitboard::lsu(self.p.en_passant) | super::bitboard::lsd(self.p.en_passant)) & (Rank::R4.bitboard() | Rank::R5.bitboard());
        // opponent pieces that cannot become SpecialBB::Empty.declass() square after my next move
        let opp = self.opponent();

        let king = self.p.pos[opp.other()][Piece::King];

        let pseudo_blockers = self.occupied[opp]; // & !(en_passant_pawns));
        let sliding_attacks = attacks::generate_bishops(
            self.p.pos[opp][Piece::Bishop] | self.p.pos[opp][Piece::Queen],
            pseudo_blockers,
        ) | attacks::generate_rooks(
            self.p.pos[opp][Piece::Rook] | self.p.pos[opp][Piece::Queen],
            pseudo_blockers,
        );
        self.pinned = if sliding_attacks & king != SpecialBB::Empty.declass() {
            // there are pins coming from the king pos, extract them by reversing propag
            let trajectories =
                attacks::generate_queens(king, self.occupied[opp] | self.occupied[opp.other()]);
            trajectories & sliding_attacks | king
        } else {
            SpecialBB::Empty.declass()
        }
    }
    pub fn is_check(&self) -> bool {
        self.attacked[self.opponent()] & self.p.pos()[self.player()][Piece::King]
            != SpecialBB::Empty.declass()
    }
    // is opponent already in check
    // this could only come from a pin that is broken
    pub fn is_illegal(p: &Position) -> bool {
        let blockers = p.pos[Player::White].occupied() | p.pos[Player::Black].occupied();
        let relevant_attacks = attacks::generate_bishops(
            p.pos[p.turn()][Piece::Bishop] | p.pos[p.turn()][Piece::Queen],
            blockers,
        ) | attacks::generate_rooks(
            p.pos[p.turn()][Piece::Rook] | p.pos[p.turn()][Piece::Queen],
            blockers,
        ) | attacks::generate_knights(p.pos[p.turn()][Piece::Knight])
            | attacks::generate_pawns(p.pos[p.turn()][Piece::Pawn], p.turn());
        let is_king_attacked =
            p.pos[p.turn().other()][Piece::King] & relevant_attacks != SpecialBB::Empty.declass();
        #[cfg(debug_assertions)]
        {
            let p = p.clone();
            let a = AugmentedPos::list_issues(&p);
            let legal_moves_next_turn = match a {
                Ok(_) => true,
                Err(()) => false,
            };
            if legal_moves_next_turn == false && is_king_attacked == false {
                log!(
                    Level::Warn,
                    "Warning : Illegal position detected with debug mode that was skipped in normal mode"
                );
            }
        };
        log!(Level::Trace, "Illegal position detected: p {p:?}");
        is_king_attacked
    }
}
