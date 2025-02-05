use crate::prelude::*;
use std::fmt::{Debug, Display};

use crate::piece::Piece;
use dests::{generate_king_dests, pawn_move_up_nocap};
use log::warn;

use super::Player;
use super::castle::{CASTLES_KEEP_UNCHANGED, Castle, CastleData};
use crate::bitboard::Bitboard;

use super::Position;

pub mod attacks;
mod dests;
// mod heapvec;

// Pseudo legal - simplified move -> can lead to a unknown number of states

pub trait TransitionSet<T> {}
// if fits in 32 bits, relevant data is used at runtime
// to have the legacy behaviour you could collect full moves

#[derive(Clone, Copy, Debug)]
pub enum Move {
    Normal(SimplifiedMove),
    Castle(Castle, Player),
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Move::Normal(x) => write!(f, "{}", x),
            Move::Castle(c, p) => match c {
                Castle::Short => match p {
                    Player::Black => write!(f, "e8g8"),
                    Player::White => write!(f, "e1g1"),
                },
                Castle::Long => match p {
                    Player::Black => write!(f, "e8c8"),
                    Player::White => write!(f, "e1c1"),
                },
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SimplifiedMove {
    pub src: Bitboard<PackedSquare>,
    pub dest: Bitboard<PackedSquare>,
    pub piece: Piece,
    pub hint_legal: bool,
}
impl Display for SimplifiedMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.piece == Piece::Pawn
            && self.dest.declass() & (Rank::R1.bb() | Rank::R8) != SpecialBB::Empty.declass()
        {
            warn!(
                "Promotions are not supported yet ({} -> {}), defaulting to queen.",
                self.src, self.dest
            );
            let c: char = ['P', 'N', 'B', 'R', 'Q', 'K'][Piece::Queen as usize];
            write!(f, "{}{}{}", self.src, self.dest, c)?;
        } else {
            write!(f, "{}{}", self.src, self.dest)?;
        }
        Ok(())
    }
}

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
        dest: Bitboard<Square>,
        from: Bitboard<Square>,
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
        new_p: Piece,
        cap: Option<Piece>,
        dest: &Bitboard<Square>,
        from: &Bitboard<Square>,
    ) -> Self {
        Self {
            data: Change::encode(new_p, cap, *dest, *from),
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

// dest has to point to a single square
fn generate_capture_data(meta: &AugmentedPos, dest: Bitboard<Square>, p: Piece) -> Option<Piece> {
    // TODO: #[cfg(debug_assertions)] - single byte
    /*if (dest.declass() & (meta.get_occupied()[meta.opponent()] | meta.p.en_passant))
        != SpecialBB::Empty.declass()
    */
    match meta.p.pos.get(dest) {
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

// returns the number of atomic moves associated, typically 1 or 4
pub fn generate_non_promoting_atmove(
    meta: &AugmentedPos,
    src: &Bitboard<Square>,
    dest: &Bitboard<Square>,
    piece: &Piece,
) -> AtomicMove {
    AtomicMove::PieceMoved(Change::encode(
        *piece,
        generate_capture_data(meta, *dest, *piece),
        *dest,
        *src,
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

fn iter_castle_moves<R>(cda: CastleData, m: &AugmentedPos) -> impl Iterator<Item = Move> {
    let player = m.player();
    let blockers = m.p.pos.occupied(player) | m.p.pos.occupied(player.other());
    let attacks = m.attacked[player.other() as usize];
    let x = [Castle::Short, Castle::Long]
        .iter()
        .filter_map(move |c| {
            if cda.fetch(player, *c) {
                Some(*c)
            } else {
                None
            }
        })
        .filter(move |c| {
            attacks & c.files() & player.backrank() == SpecialBB::Empty.declass()
                && blockers & c.free_files() & player.backrank() == SpecialBB::Empty.declass()
        });
    let r = x.map(move |c| Move::Castle(c, player));
    r
}

// -- prefilter legal, put pesudo legal remain
fn filter_pseudo_legal(p: &AugmentedPos, m: Move) -> Option<Move> {
    if let Move::Normal(mut m) = m {
        let pinned = (m.src.declass() & p.pinned) != SpecialBB::Empty.declass();
        // if src is pinned and moves to a destination not pinned it will be illegal anyway
        let pinned_dst = m.dest.declass() & p.pinned != SpecialBB::Empty.declass();
        let is_check = p.is_check();

        let mut edge_case = false;
        // let mut known_illegal = false;

        // if moving a pinned piece out of the pinned lines
        if pinned {
            edge_case = true;
        }

        // in check but not moving king blocking pins nor capturing source
        if is_check && !(m.piece == Piece::King) {
            edge_case = true;
            if !pinned_dst
                && (p.p.pos.occupied(p.turn.other()) & m.dest) == SpecialBB::Empty.declass()
            {
                return None;
            }
        }
        m.hint_legal = !edge_case;
        Some(Move::Normal(m))
    } else {
        Some(m)
    }
}

// structure containing a
#[derive(Debug)]
pub struct AugmentedPos<'a> {
    p: &'a Position, // mutable to allow to simulate pseudo legal moves, but will always return it unchanged
    turn: Player,
    attacked: [Bitboard<GenericBB>; 2],
    pinned: Bitboard<GenericBB>,
}

impl<'a> AugmentedPos<'a> {
    pub fn map_issues<R>(
        p: &Position,
        task: impl Fn(&Position, &Move) -> R,
        reduction: impl Fn(R, R) -> R,
    ) -> Option<R> {
        let turn = Player::from_usize((p.half_move_count % 2).into());
        let mut a = AugmentedPos {
            p,
            attacked: [SpecialBB::Empty.declass(), SpecialBB::Empty.declass()],
            pinned: SpecialBB::Empty.declass(),
            turn,
        };
        a.compute_pinned();

        let a = a.gen_moves_map(task, &reduction);
        a
    }

    pub fn check_legal(p: &Position) -> Result<(), ()> {
        let turn = Player::from_usize((p.half_move_count % 2).into());
        let mut a = AugmentedPos {
            p,
            attacked: [SpecialBB::Empty.declass(), SpecialBB::Empty.declass()],
            pinned: SpecialBB::Empty.declass(),
            turn,
        };

        let _blockers = a.p.pos.occupied(Player::White) | a.p.pos.occupied(Player::Black);

        a.attacked[a.turn as usize] = a.p.pos.generate_attacks(a.turn);
        if a.p.pos[(a.turn.other(), Piece::King)] & a.attacked[a.turn as usize]
            != SpecialBB::Empty.declass()
        {
            return Err(());
        } else {
            return Ok(());
        }
    }

    pub const fn get_attacked(&self) -> &[Bitboard<GenericBB>] {
        &self.attacked
    }
    pub const fn player(&self) -> Player {
        self.turn
    }
    pub const fn opponent(&self) -> Player {
        self.turn.other()
    }

    fn gen_moves_map<R>(
        &mut self,
        task: impl Fn(&Position, &Move) -> R,
        reduce: impl Fn(R, R) -> R,
    ) -> Option<R> {
        self.attacked[self.turn.other() as usize] = self.p.pos.generate_attacks(self.turn.other());
        self.attacked[self.turn as usize] = self.p.pos.generate_attacks(self.turn);

        let gen_dests = |piece: Piece, src: Bitboard<Square>| -> Bitboard<GenericBB> {
            let free = !self.p.pos.occupied(self.turn);
            let blockers = self.p.pos.occupied(self.turn.other()) | self.p.pos.occupied(self.turn);
            free & match piece {
                Piece::Pawn => {
                    (attacks::generate_pawns(src.declass(), self.turn)
                        & (self.p.en_passant | self.p.pos.occupied(self.turn.other())))
                        | pawn_move_up_nocap(src, self.turn, blockers)
                }
                Piece::Knight => {
                    attacks::generate_knights(src.declass()) & !self.p.pos.occupied(self.turn)
                }
                Piece::Bishop => attacks::generate_bishops(src.declass(), blockers),
                Piece::Rook => attacks::generate_rooks(src.declass(), blockers),
                Piece::Queen => attacks::generate_queens(src.declass(), blockers),
                Piece::King => generate_king_dests(src, self),
            }
        };

        use enum_iterator::all;
        let a = all::<Piece>()
            .map(|p| {
                self.p.pos[(self.turn, p)]
                    .into_iter()
                    .map(|src| {
                        gen_dests(p, src)
                            .into_iter()
                            .map(|dest| {
                                Move::Normal(SimplifiedMove {
                                    piece: p,
                                    src: src.into(),
                                    dest: dest.into(),
                                    hint_legal: false,
                                })
                            })
                            .filter_map(|m| filter_pseudo_legal(self, m))
                            .map(|m| {
                                Position::simplified_move_outcomes(*self.p, &m, &task, &reduce)
                            })
                            .filter_map(|x| x)
                            .reduce(&reduce)
                    })
                    .filter_map(|x| x)
                    .reduce(&reduce)
            })
            .filter_map(|x| x)
            .reduce(&reduce);
        //TODO: add castling
        let b = iter_castle_moves::<R>(self.p.castles, self)
            .map(|m| Position::simplified_move_outcomes(*self.p, &m, &task, &reduce))
            .filter_map(|x| x)
            .reduce(&reduce);
        match (a, b) {
            (Some(x), Some(y)) => Some(reduce(x, y)),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        }
    }

    fn compute_pinned(&mut self) {
        let opp = self.opponent();

        let king = self.p.pos[(opp.other(), Piece::King)];

        let pseudo_blockers = self.p.pos.occupied(opp);
        let sliding_attacks = attacks::generate_bishops(
            self.p.pos[(opp, Piece::Bishop)] | self.p.pos[(opp, Piece::Queen)],
            pseudo_blockers,
        ) | attacks::generate_rooks(
            self.p.pos[(opp, Piece::Rook)] | self.p.pos[(opp, Piece::Queen)],
            pseudo_blockers,
        );
        self.pinned = if sliding_attacks & king != SpecialBB::Empty.declass() {
            let trajectories = attacks::generate_queens(
                king,
                self.p.pos.occupied(opp) | self.p.pos.occupied(opp.other()),
            );
            trajectories & sliding_attacks | king
        } else {
            SpecialBB::Empty.declass()
        }
    }
    pub fn is_check(&self) -> bool {
        let x = self.p.pos()[(self.player(), Piece::King)];
        self.attacked[self.opponent() as usize] & x != SpecialBB::Empty.declass()
    }

    pub fn is_illegal(p: &Position) -> bool {
        let blockers = p.pos.occupied(Player::White) | p.pos.occupied(Player::Black);
        let relevant_attacks = attacks::generate_bishops(
            p.pos[(p.turn(), Piece::Bishop)] | p.pos[(p.turn(), Piece::Queen)],
            blockers,
        ) | attacks::generate_rooks(
            p.pos[(p.turn(), Piece::Rook)] | p.pos[(p.turn(), Piece::Queen)],
            blockers,
        ) | attacks::generate_knights(p.pos[(p.turn(), Piece::Knight)])
            | attacks::generate_pawns(p.pos[(p.turn(), Piece::Pawn)], p.turn());
        let is_king_attacked =
            p.pos[(p.turn().other(), Piece::King)] & relevant_attacks != SpecialBB::Empty.declass();
        is_king_attacked
    }
}
