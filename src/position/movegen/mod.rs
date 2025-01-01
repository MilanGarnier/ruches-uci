use std::fmt::Debug;

use customvec::{FastVec, MoveVec};
use dests::generate_next_en_passant_data;
use localvec as customvec;

use super::bitboard::{self, Bitboard, File, FromBB, GenericBB, Rank, SpecialBB, Square, ToBB};
use super::castle::{self, CASTLES_KEEP_UNCHANGED, Castle, CastleData};
use super::piece::Piece;
use super::{Player, PlayerStorage};

use super::{PieceSet, Position, UciNotation};

pub mod attacks;
mod dests;
// mod heapvec;
mod localvec;

#[derive(Copy, Clone, Debug)]
pub struct Change {
    p: Piece,
    dest: Bitboard<Square>,
    from: Bitboard<Square>,
}
impl Change {
    pub fn piece(&self) -> Piece {
        self.p
    }
    pub fn bitboard(&self) -> Bitboard<GenericBB> {
        self.dest | self.from
    }
}
impl UciNotation for Change {
    fn to_uci(&self) -> String {
        // let mut from = String::new();
        /*for sq in self.from {
            from += &sq.to_uci()
        }*/
        self.from.to_uci() + &self.dest.to_uci()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Promotion {
    pub new: Piece, // convert piece into
    pub dest: Bitboard<Square>,
    pub from: Bitboard<Square>,
}
impl UciNotation for Promotion {
    fn to_uci(&self) -> String {
        let repr = [['P', 'N', 'B', 'R', 'Q', 'K'], [
            'p', 'n', 'b', 'r', 'q', 'k',
        ]];
        let mut s = self.from.to_uci();
        let pl = match self.dest.declass() & Player::White.backrank() == SpecialBB::Empty.declass()
        {
            true => Player::White,
            false => Player::Black,
        };
        s += &self.dest.to_uci();
        s.push(repr[pl as usize][self.new as usize]);
        s
    }
}
impl Promotion {}

#[derive(Copy, Clone, Debug)]
pub enum AtomicMove {
    PieceMoved(Change),
    PiecePromoted(Promotion),
}
impl AtomicMove {
    pub fn does_affect(&self, piece: Piece) -> bool {
        match self {
            AtomicMove::PiecePromoted(prom) => (piece == Piece::Pawn) || (piece == prom.new),
            AtomicMove::PieceMoved(mv) => mv.p == piece,
        }
    }
    pub const fn dest(&self) -> Bitboard<Square> {
        match self {
            AtomicMove::PieceMoved(m) => m.dest,
            AtomicMove::PiecePromoted(p) => p.dest,
        }
    }
    pub fn src(&self) -> Bitboard<Square> {
        match self {
            AtomicMove::PieceMoved(m) => m.from,
            AtomicMove::PiecePromoted(p) => p.from,
        }
    }
}
impl UciNotation for AtomicMove {
    fn to_uci(&self) -> String {
        match self {
            AtomicMove::PieceMoved(x) => x.to_uci(),
            AtomicMove::PiecePromoted(x) => x.to_uci(),
        }
    }
}

// change in opponent pieces for capture
#[derive(Clone, Copy)]
pub struct CaptureData {
    pub piece: Piece,
    pub dst: Bitboard<Square>,
}

#[derive(Copy, Clone)]
pub struct StandardMove {
    pub mv: AtomicMove,
    pub cap: Option<CaptureData>,
    pub cas: CastleData,
}
impl StandardMove {
    pub fn is_moved(&self, piece: Piece) -> bool {
        return self.mv.does_affect(piece);
    }
    pub fn uci(&self) -> String {
        self.mv.to_uci()
    }
}

#[derive(Copy, Clone)]
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
    pub const fn is_capture(&self) -> bool {
        match self {
            PartialMove::Std(x) => match x.cap {
                None => false,
                _ => true,
            },
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
    pub fn uci(&self) -> String {
        match self {
            PartialMove::Castle(x, pl, _) => (x, pl).to_uci(),
            PartialMove::Std(x) => x.uci(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Move {
    // Full move data
    pm: PartialMove,
    fifty_mv: usize, // either 0 if this move is eligible, or the nb of half moves before it happened
    en_passant: Bitboard<GenericBB>,
}

impl Move {
    pub const fn partialmove(&self) -> PartialMove {
        self.pm
    }
    pub const fn fifty_mv(&self) -> usize {
        self.fifty_mv
    }
    pub const fn en_passant(&self) -> Bitboard<GenericBB> {
        self.en_passant
    }

    pub const fn is_capture(&self) -> bool {
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

    pub fn uci(&self) -> String {
        self.pm.uci()
    }
}

impl Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = write!(f, "{}", self.uci());
        Ok(())
    }
}

pub struct MoveIter<'a> {
    array: &'a [Option<Move>; 256],
    index: usize,
}
impl<'a> MoveIter<'a> {
    pub fn create(array: &'a [Option<Move>; 256]) -> Self {
        MoveIter { array, index: 0 }
    }
}
impl<'a> Iterator for MoveIter<'a> {
    type Item = Move;
    fn next(&mut self) -> Option<Self::Item> {
        match self.index {
            256 => None,
            _ => {
                let x = self.array[self.index];
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
fn generate_capture_data(
    meta: &AugmentedPos,
    dest: Bitboard<Square>,
    p: Piece,
) -> Option<CaptureData> {
    // TODO: #[cfg(debug_assertions)] - single byte
    /*if (dest.declass() & (meta.get_occupied()[meta.opponent()] | meta.p.en_passant))
        != SpecialBB::Empty.declass()
    */
    match which_piece_on_sq(dest, meta.semi_pos(meta.opponent())) {
        None => match p {
            // ghost capture, either en passant or nothing
            Piece::Pawn => match meta.p.en_passant & dest == SpecialBB::Empty.declass() {
                true => Some(CaptureData {
                    piece: Piece::Pawn,
                    dst: dest,
                }),
                false => None,
            },
            // {println!("En passant detected but not properly handled yet"); meta.p.pretty_print(); todo!()},
            _ => None, // only a piece going behind a pawn which just moved up 2 squares
        },
        //todo!(), // en passant !!!,
        Some(x) => Some(CaptureData {
            piece: x,
            dst: dest,
        }),
    }
}

// returns the number of atomic moves associated, typically 1 or 6
pub fn generate_non_promoting_atmove(
    src: &Bitboard<Square>,
    dest: &Bitboard<Square>,
    piece: &Piece,
) -> AtomicMove {
    /*#[cfg(debug_assertions)]
    {
        let condition = (*piece as usize == Piece::Pawn as usize)
            && (dest.declass() & (Rank::R1.declass() | Rank::R8.declass()))
                != SpecialBB::Empty.declass();
        debug_assert!(!condition);
    }*/
    AtomicMove::PieceMoved(Change {
        p: *piece,
        dest: *dest,
        from: *src,
    })
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
        Piece::King => cd.x[meta.player() as usize] = meta.p.castles.x[meta.player() as usize],
        Piece::Rook => {
            if src.declass() == (File::A.declass() & meta.player().backrank()) {
                cd.x[meta.player() as usize].x[Castle::Long as usize] =
                    meta.p.castles.x[meta.player() as usize].x[Castle::Long as usize]
            } else if src.declass() == (File::H.declass() & meta.player().backrank()) {
                cd.x[meta.player() as usize].x[Castle::Short as usize] =
                    meta.p.castles.x[meta.player() as usize].x[Castle::Short as usize]
            }
        }
        _ => (),
    }
    // capture opponent rook
    if dest.declass() == (File::A.declass() & meta.opponent().backrank()) {
        cd.x[meta.opponent() as usize].x[Castle::Long as usize] =
            meta.p.castles.x[meta.opponent() as usize].x[Castle::Long as usize]
    } else if dest.declass() == (File::H.declass() & meta.opponent().backrank()) {
        cd.x[meta.opponent() as usize].x[Castle::Short as usize] =
            meta.p.castles.x[meta.opponent() as usize].x[Castle::Short as usize]
    }

    cd
}

// return in [0/1/2]
pub fn generate_castle_move(meta: &AugmentedPos, c: castle::Castle) -> Option<Move> {
    if meta.p.castles.fetch(meta.player(), c)  // right to castle
    &&  meta.attacked[meta.opponent()] & (c.files() & meta.player().backrank()) == SpecialBB::Empty.declass()  // no attacks on path (including check)
    &&  (meta.occupied[meta.player()] | meta.occupied[meta.opponent()]) & (c.free_files() & meta.player().backrank()) == SpecialBB::Empty.declass()
    // no piece on rook&king path
    {
        // forbid current player castles
        let mut new_cas_data = CASTLES_KEEP_UNCHANGED;
        new_cas_data.x[meta.player() as usize] = meta.p.castles.x[meta.player() as usize];

        let mv = Move {
            pm: PartialMove::Castle(c, meta.player(), new_cas_data),
            fifty_mv: 0,
            en_passant: meta.p.en_passant,
        };
        Some(mv)
    } else {
        None
    }
}

fn is_legal(p: &Position, mv: Move) -> bool {
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
fn add_to_move_list(p: &AugmentedPos, m: Move, movelist: &mut MoveVec) {
    // #[cfg(debug_assertions)]
    // {
    //     println!("Considering move {:?}", m.uci());
    // }
    let pinned = (m.src().declass() & p.pinned) != SpecialBB::Empty.declass();
    // if src is pinned and moves to a destination not pinned it will be illegal anyway
    let pinned_dst = m.dest().declass() & p.pinned != SpecialBB::Empty.declass();
    let is_check = p.is_check();
    let affect_ep = !(m.en_passant == SpecialBB::Empty.declass());

    let mut edge_case = false;
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
    if !edge_case || is_legal(p.p, m) {
        movelist.push(m);
    }
}

fn generate_non_pawn_move_data(
    meta: &AugmentedPos,
    piece: &Piece,
    src: &Bitboard<Square>,
    dests: &Bitboard<GenericBB>,
    movelist: &mut MoveVec,
) {
    //println!("Generate move data for {:?} on {:?}, dests {:?}", piece, src, dests);
    for sq in *dests {
        let atomic_move = generate_non_promoting_atmove(src, &sq, piece);
        {
            let pm = PartialMove::Std(StandardMove {
                mv: atomic_move,
                cap: generate_capture_data(meta, sq, Piece::King),
                cas: generate_castle_data(meta, src, &sq, piece),
            });
            let fifty_mv = match pm.is_capture() | (*piece == Piece::Pawn) {
                true => meta.p.fifty_mv,
                _ => 0,
            };

            let en_passant = meta.p.en_passant
                ^ dests::generate_next_en_passant_data(*piece, *src, sq, meta.player());

            add_to_move_list(
                meta,
                Move {
                    pm,
                    fifty_mv,
                    en_passant,
                },
                movelist,
            );
        }
    }
}

pub fn generate_pawn_atmove(
    src: &Bitboard<Square>,
    dest: &Bitboard<Square>,
    piece: &Piece,
) -> FastVec<4, AtomicMove> {
    let is_prom = (*piece as usize == Piece::Pawn as usize)
        && (dest.declass() & (Rank::R1.declass() | Rank::R8.declass()))
            != SpecialBB::Empty.declass();
    let mut p: FastVec<4, AtomicMove> = FastVec::new();
    match is_prom {
        false => {
            p.push(generate_non_promoting_atmove(src, dest, piece));
            p
        }
        true => {
            let a = AtomicMove::PiecePromoted(Promotion {
                dest: *dest,
                from: *src,
                new: Piece::Knight,
            });
            let b = AtomicMove::PiecePromoted(Promotion {
                dest: *dest,
                from: *src,
                new: Piece::Bishop,
            });
            let c = AtomicMove::PiecePromoted(Promotion {
                dest: *dest,
                from: *src,
                new: Piece::Rook,
            });
            let d = AtomicMove::PiecePromoted(Promotion {
                dest: *dest,
                from: *src,
                new: Piece::Queen,
            });
            p.push(a);
            p.push(b);
            p.push(c);
            p.push(d);
            p
        }
    }
}

fn generate_pawn_move_data(meta: &AugmentedPos, src: &Bitboard<Square>, movelist: &mut MoveVec) {
    let blockers = meta.occupied[Player::White] | meta.occupied[Player::Black];
    let captures = meta.p.en_passant | meta.occupied[meta.opponent()];
    let dests = (attacks::generate_pawns(src.declass(), meta.turn) & captures)
        | dests::pawn_move_up_nocap(*src, meta.player(), blockers);

    for sq in dests {
        let outcomes = generate_pawn_atmove(src, &sq, &Piece::Pawn);

        let pm = PartialMove::Std(StandardMove {
            mv: outcomes[0],
            cap: generate_capture_data(meta, sq, Piece::King),
            cas: generate_castle_data(meta, src, &sq, &Piece::Pawn),
        });
        let mut m = Move {
            pm,
            fifty_mv: meta.p.fifty_mv,
            en_passant: generate_next_en_passant_data(Piece::Pawn, *src, sq, meta.turn)
                ^ meta.p.en_passant,
        };

        if outcomes.len() > 1 {
            // then outcomes = 4
            for i in 1..4 {
                add_to_move_list(meta, m, movelist);
                match &mut m.pm {
                    PartialMove::Std(x) => x.mv = outcomes[i],
                    _ => panic!(),
                }
            }
            add_to_move_list(meta, m, movelist);
        } else {
            add_to_move_list(meta, m, movelist);
        }
    }
}

// structure containing a
#[derive(Debug)]
pub struct AugmentedPos<'a> {
    p: &'a Position, // mutable to allow to simulate pseudo legal moves, but will always return it unchanged
    turn: Player,
    occupied: PlayerStorage<Bitboard<GenericBB>>,
    attacked: PlayerStorage<Bitboard<GenericBB>>,
    pinned: Bitboard<GenericBB>,
}

impl<'a> AugmentedPos<'a> {
    // would return an error if the position is illegal already
    pub fn list_issues(p: &'a Position) -> Result<MoveVec, ()> {
        let turn = Player::from_usize(p.half_move_count % 2);
        let mut a = AugmentedPos {
            p: p,
            occupied: PlayerStorage::from([SpecialBB::Empty.declass(), SpecialBB::Empty.declass()]),
            attacked: PlayerStorage::from([SpecialBB::Empty.declass(), SpecialBB::Empty.declass()]),
            pinned: SpecialBB::Empty.declass(),
            turn: turn,
        };
        a.compute_occupied();
        a.compute_pinned();

        a.compute_attacked_gen_moves()
    }

    pub fn check_legal(p: &Position) -> Result<(), ()> {
        let turn = Player::from_usize(p.half_move_count % 2);
        let mut a = AugmentedPos {
            p: p,
            occupied: PlayerStorage::from([SpecialBB::Empty.declass(), SpecialBB::Empty.declass()]),
            attacked: PlayerStorage::from([SpecialBB::Empty.declass(), SpecialBB::Empty.declass()]),
            pinned: SpecialBB::Empty.declass(),
            turn: turn,
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
    }

    // assume occupied squares and "pins" are known
    fn compute_attacked_gen_moves(&mut self) -> Result<customvec::MoveVec, ()> {
        let mut movelist = customvec::MoveVec::new();
        let blockers = self.occupied[Player::White] | self.occupied[Player::Black];

        self.attacked[self.turn.other()] =
            self.p.pos[self.turn.other()].attacks(self.turn.other(), blockers);

        // generate my attacks and my moves at the same time
        self.attacked[self.turn] = {
            let mut attacked = SpecialBB::Empty.declass();
            let mask = !self.occupied[self.turn];
            for sq in self.p.pos[self.turn][Piece::Bishop] {
                let dests = attacks::generate_bishops(sq.declass(), blockers);
                generate_non_pawn_move_data(
                    self,
                    &Piece::Bishop,
                    &sq,
                    &(dests & mask),
                    &mut movelist,
                );
                attacked = attacked | dests;
            }
            for sq in self.p.pos[self.turn][Piece::Queen] {
                let dests = attacks::generate_queens(sq.declass(), blockers);
                generate_non_pawn_move_data(
                    self,
                    &Piece::Queen,
                    &sq,
                    &(dests & mask),
                    &mut movelist,
                );
                attacked = attacked | dests;
            }
            for sq in self.p.pos[self.turn][Piece::Rook] {
                let dests = attacks::generate_rooks(sq.declass(), blockers);
                generate_non_pawn_move_data(
                    self,
                    &Piece::Rook,
                    &sq,
                    &(dests & mask),
                    &mut movelist,
                );
                attacked = attacked | dests;
            }
            for sq in self.p.pos[self.turn][Piece::Knight] {
                let dests = attacks::generate_knights(sq.declass());
                generate_non_pawn_move_data(
                    self,
                    &Piece::Knight,
                    &sq,
                    &(dests & mask),
                    &mut movelist,
                );
                attacked = attacked | dests;
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

        for pawn in self.p.pos[self.turn][Piece::Pawn] {
            generate_pawn_move_data(self, &pawn, &mut movelist);
        }

        {
            // king moves
            let king = Square::from_bb(&self.p.pos[self.turn][Piece::King]).unwrap();
            let king_dests = dests::generate_king_dests(king, self);
            generate_non_pawn_move_data(self, &Piece::King, &king, &king_dests, &mut movelist);
        }

        {
            // castles
            match generate_castle_move(self, castle::Castle::Short) {
                Some(c) => add_to_move_list(self, c, &mut movelist),
                _ => (),
            }
            match generate_castle_move(self, castle::Castle::Long) {
                Some(c) => add_to_move_list(self, c, &mut movelist),
                _ => (),
            }
        }

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
        /*#[cfg(debug_assertions)]
        {
            let p = p.clone();
            let a = AugmentedPos::list_issues(&p, r);
            match a {
                Ok(_) => true,
                Err(()) => false,
            }
        };*/
        is_king_attacked
    }
}
