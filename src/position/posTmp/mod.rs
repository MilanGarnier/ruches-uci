pub trait UciNotation {
    fn to_uci(&self) -> String;
}
pub trait Parsing {
    type Resulting: ?Sized;
    fn from_str(s: &str) -> Self::Resulting;
}

use bitboard::{BBSquare, Bitboard, File, FromBB, GenericBB, Rank, SpecialBB, Square, ToBB};

use castle::{CASTLES_ALL_FORBIDDEN, Castle};
use movegen::static_attacks::Lookup;
use movegen::{
    AtomicMove, AugmentedPos, CaptureData, Change, Move, PartialMove, Promotion, StandardMove,
};

pub mod bitboard;
mod castle;
pub mod movegen;
pub mod piece;

use crate::position::castle::{CASTLES_ALL_ALLOWED, CastleData};
use crate::position::piece::Piece;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Player {
    White,
    Black,
}

impl Player {
    pub const COUNT: usize = 2;
    pub const fn other(&self) -> Player {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
        }
    }
    pub const fn backrank(&self) -> Bitboard<Rank> {
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

#[derive(Clone, Copy, Debug)]
pub struct PieceSet {
    pawns: Bitboard<GenericBB>,
    knights: Bitboard<GenericBB>,
    bishops: Bitboard<GenericBB>,
    rooks: Bitboard<GenericBB>,
    queens: Bitboard<GenericBB>,
    king: Bitboard<GenericBB>, // TODO: move to squares
}
#[derive(Clone, Copy, Debug)]
pub struct PlayerStorage<T> {
    black: T,
    white: T,
}
impl<T: Clone> PlayerStorage<T> {
    #[inline(always)]
    pub fn from(x: [T; 2]) -> Self {
        Self {
            white: x[0].clone(),
            black: x[1].clone(),
        }
    }
}

impl PieceSet {
    fn occupied(&self) -> Bitboard<GenericBB> {
        self.pawns | self.bishops | self.king | self.knights | self.queens | self.rooks
    }
    fn attacks(
        &self,
        player: Player,
        blockers: Bitboard<GenericBB>,
        runtime: &Lookup,
    ) -> Bitboard<GenericBB> {
        movegen::dyn_attacks::generate_pawns(self[Piece::Pawn], player)
            | movegen::dyn_attacks::generate_knights(self[Piece::Knight])
            | runtime.generate_bishops(self[Piece::Bishop] | self[Piece::Queen], blockers)
            | runtime.generate_rooks(self[Piece::Rook] | self[Piece::Queen], blockers)
            | movegen::dyn_attacks::generate_king(Square::from_bb(&self[Piece::King]).unwrap())
    }
}

impl std::ops::Index<Piece> for PieceSet {
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

impl std::ops::IndexMut<Piece> for PieceSet {
    #[inline(always)]
    fn index_mut(&mut self, index: Piece) -> &mut Self::Output {
        match index {
            Piece::Pawn => &mut self.pawns,
            Piece::King => &mut self.king,
            Piece::Bishop => &mut self.bishops,
            Piece::Rook => &mut self.rooks,
            Piece::Knight => &mut self.knights,
            Piece::Queen => &mut self.queens,
        }
    }
}

type PieceData = PlayerStorage<PieceSet>;

impl<T> std::ops::Index<Player> for PlayerStorage<T> {
    type Output = T;
    #[inline(always)]
    fn index(&self, index: Player) -> &Self::Output {
        match index {
            Player::Black => &self.black,
            Player::White => &self.white,
        }
    }
}
impl<T> std::ops::IndexMut<Player> for PlayerStorage<T> {
    #[inline(always)]
    fn index_mut(&mut self, index: Player) -> &mut Self::Output {
        match index {
            Player::Black => &mut self.black,
            Player::White => &mut self.white,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub fifty_mv: usize,
    half_move_count: usize,
    pos: PieceData,
    castles: CastleData,
    en_passant: Bitboard<GenericBB>,
}

impl PieceSet {
    fn startingpos(p: Player) -> Self {
        PieceSet {
            pawns: Piece::Pawn.startingpos(p),
            knights: Piece::Knight.startingpos(p),
            bishops: Piece::Bishop.startingpos(p),
            rooks: Piece::Rook.startingpos(p),
            queens: Piece::Queen.startingpos(p),
            king: Piece::King.startingpos(p),
        }
    }
    pub fn empty() -> Self {
        PieceSet {
            pawns: SpecialBB::Empty.declass(),
            knights: SpecialBB::Empty.declass(),
            bishops: SpecialBB::Empty.declass(),
            rooks: SpecialBB::Empty.declass(),
            queens: SpecialBB::Empty.declass(),
            king: SpecialBB::Empty.declass(),
        }
    }
}
impl PieceData {
    pub fn startingpos() -> Self {
        PieceData {
            white: PieceSet::startingpos(Player::White),
            black: PieceSet::startingpos(Player::Black),
        }
    }
    pub fn empty() -> Self {
        PieceData {
            white: PieceSet::empty(),
            black: PieceSet::empty(),
        }
    }
}

impl Position {
    pub fn startingpos() -> Position {
        Position {
            half_move_count: 0,
            fifty_mv: 0,
            pos: PieceData::startingpos(),
            castles: CASTLES_ALL_ALLOWED,
            en_passant: SpecialBB::Empty.declass(),
        }
    }
    pub fn empty() -> Self {
        Self {
            half_move_count: 0,
            fifty_mv: 0,
            pos: PieceData::empty(),
            castles: CASTLES_ALL_FORBIDDEN,
            en_passant: SpecialBB::Empty.declass(),
        }
    }
    pub fn pos(&self) -> &PieceData {
        &self.pos
    }
    pub fn turn(&self) -> Player {
        match self.half_move_count % 2 {
            0 => Player::White,
            _ => Player::Black,
        }
    }

    fn stack_capdata_rev(&mut self, cd: &CaptureData, pl: Player) {
        if cd.piece == Piece::Pawn
            && cd.dst.declass() & self.en_passant != Bitboard(SpecialBB::Empty).declass()
        {
            // toggle ennemy pawn
            self.pos[pl.other()][Piece::Pawn] = self.pos[pl.other()][Piece::Pawn] ^ {
                match pl.other() {
                    Player::Black => (cd.dst.declass() & self.en_passant) - 1,
                    Player::White => (cd.dst.declass() & self.en_passant) + 1,
                }
            }
        }
        self.pos[pl][cd.piece] = self.pos[pl][cd.piece] ^ cd.dst
    }

    fn stack_change_rev(&mut self, ch: &Change, pl: Player) {
        // en passant case
        if ch.piece() == Piece::Pawn
            && ch.bitboard() & self.en_passant != Bitboard(SpecialBB::Empty).declass()
        {
            // toggle ennemy pawn
            self.pos[pl.other()][Piece::Pawn] = self.pos[pl.other()][Piece::Pawn] ^ {
                match pl.other() {
                    Player::Black => (ch.bitboard() & self.en_passant) - 1,
                    Player::White => (ch.bitboard() & self.en_passant) + 1,
                }
            }
        }
        self.pos[pl][ch.piece()] = self.pos[pl][ch.piece()] ^ ch.bitboard()
    }
    fn stack_prom_rev(&mut self, pr: &Promotion) {
        let turn = self.turn();
        self.pos[turn][pr.new] = self.pos[turn][pr.new] ^ pr.dest;
        self.pos[turn][Piece::Pawn] = self.pos[turn][Piece::Pawn] ^ pr.from;
    }
    fn stack_atomic_rev(&mut self, amv: &AtomicMove) {
        match amv {
            AtomicMove::PieceMoved(ch) => self.stack_change_rev(ch, self.turn()),
            AtomicMove::PiecePromoted(pr) => self.stack_prom_rev(pr),
        }
    }
    fn stack_std_rev(&mut self, smv: &StandardMove) {
        match smv.cap {
            None => (),
            Some(ch) => self.stack_capdata_rev(&ch, self.turn().other()),
        }
        self.castles.stack_rev(&smv.cas);
        self.stack_atomic_rev(&smv.mv);
    }
    fn stack_castle_rev(&mut self, cs: &Castle) {
        let turn = self.turn();
        match cs {
            Castle::Short => {
                self.pos[turn][Piece::King] = self.pos[turn][Piece::King]
                    ^ turn.backrank().declass() & (File::E.declass() | File::G.declass());
                self.pos[turn][Piece::Rook] = self.pos[turn][Piece::Rook]
                    ^ turn.backrank().declass() & (File::H.declass() | File::F.declass());
            }
            Castle::Long => {
                self.pos[turn][Piece::King] = self.pos[turn][Piece::King]
                    ^ turn.backrank().declass() & (File::E.declass() | File::C.declass());
                self.pos[turn][Piece::Rook] = self.pos[turn][Piece::Rook]
                    ^ turn.backrank().declass() & (File::A.declass() | File::D.declass());
            }
        }
    }
    fn stack_partial_rev(&mut self, pmv: &PartialMove) {
        match pmv {
            PartialMove::Std(smv) => self.stack_std_rev(smv),
            PartialMove::Castle(cs, _, cda) => {
                self.stack_castle_rev(cs);
                self.castles.stack_rev(cda);
            }
        }
    }

    pub fn stack(&mut self, mv: &Move) {
        self.stack_partial_rev(&mv.partialmove());
        self.fifty_mv = self.fifty_mv ^ mv.fifty_mv();
        self.en_passant = self.en_passant ^ mv.en_passant();
        self.half_move_count += 1;
    }

    pub fn unstack(&mut self, mv: &Move) {
        self.half_move_count -= 1;
        self.fifty_mv = self.fifty_mv ^ mv.fifty_mv();
        self.en_passant = self.en_passant ^ mv.en_passant();
        self.stack_partial_rev(&mv.partialmove());
    }

    /*pub fn play_pseudolegal(&self, mv: &Move) -> Self {
        let mut pos = self.clone();
        pos.stack(mv);
        pos
    }*/

    // very unoptimized, should not be called when we can access the move as &mv
    pub fn getmove(&mut self, uci: &str, r: &Lookup) -> Result<Option<Move>, ()> {
        let meta = AugmentedPos::list_issues(self, r);
        let moves = match meta {
            Err(()) => return Err(()),
            Ok(meta) => meta,
        };
        for m in moves.iter() {
            if m.uci() == uci {
                return Ok(Some(*m));
            }
        }
        Ok(None)
    }

    pub fn perft_top(&mut self, depth: usize, pregen: &Lookup) -> usize {
        match depth {
            0 => 0,
            _ => {
                let r = AugmentedPos::list_issues(self, pregen);
                let ml = match r {
                    Err(()) => return 0,
                    Ok(ml) => ml,
                };

                let mut sum = 0;
                for m in ml.iter() {
                    self.stack(m);
                    let count = self.perft_rec(depth - 1, pregen);
                    println!("{}: {}", m.uci(), count);
                    sum += count;
                    self.unstack(m);
                }
                sum
            }
        }
    }

    fn perft_rec(&mut self, depth: usize, pregen: &Lookup) -> usize {
        match depth {
            0 => {
                let a = AugmentedPos::list_issues(&self, pregen);
                match a {
                    Ok(_) => 1,
                    Err(()) => 0,
                }
            }
            1 => {
                let r = AugmentedPos::list_issues(self, pregen);
                match r {
                    Err(()) => 0,
                    Ok(ml) => ml.len(),
                }
            }
            _ => {
                let r = AugmentedPos::list_issues(self, pregen);
                let ml = match r {
                    Err(()) => return 0,
                    Ok(ml) => ml,
                };

                let mut sum = 0;
                for m in ml.iter() {
                    self.stack(m);
                    let count = self.perft_rec(depth - 1, pregen);
                    self.unstack(m);
                    sum += count;
                }
                sum
            }
        }
    }

    // extract fen, knowing it is the first element in the iterator
    pub fn extract_fen(words: &mut std::str::SplitWhitespace<'_>) -> Option<Self> {
        Self::parse_fen(
            words.nth(0),
            words.nth(0),
            words.nth(0),
            words.nth(0),
            words.nth(0),
            words.nth(0),
        )
    }

    pub fn parse_fen(
        a: Option<&str>,
        b: Option<&str>,
        c: Option<&str>,
        d: Option<&str>,
        e: Option<&str>,
        f: Option<&str>,
    ) -> Option<Self> {
        match (a, b, c, d, e, f) {
            (a, b, c, d, None, None) => Self::parse_fen(a, b, c, d, Some("0"), Some("1")),
            (a, b, c, d, e, None) => Self::parse_fen(a, b, c, d, e, Some("1")),
            (_, _, _, _, None, Some(_)) => panic!("Rewrite this"),
            (None, _, _, _, _, _)
            | (_, None, _, _, _, _)
            | (_, _, None, _, _, _)
            | (_, _, _, None, _, _) => None,
            (
                Some(fen),
                Some(turn),
                Some(castles),
                Some(en_passant),
                Some(hf_mv_until_100),
                Some(full_moves),
            ) => Some(Position::from_fen(
                fen,
                turn,
                castles,
                en_passant,
                hf_mv_until_100,
                full_moves,
            )),
        }
    }

    pub fn from_fen(
        fen: &str,
        turn: &str,
        castles: &str,
        en_passant: &str,
        hf_mv_until_100: &str,
        full_moves: &str,
    ) -> Self {
        let mut sq_index = 64 - 8; // start at top square
        let mut pos: Self = Self::empty();

        for c in fen.chars().into_iter() {
            match c {
                '1'..'9' => sq_index += (c as usize) - ('0' as usize),
                '/' => sq_index -= 16,
                p => {
                    let (player, piece) = Piece::from_notation(p).unwrap();
                    pos.pos[player][piece] =
                        pos.pos[player][piece] | Bitboard(GenericBB(1 << sq_index));
                    sq_index = sq_index + 1;
                }
            }
        }
        debug_assert_eq!(sq_index, 8);

        let full_moves = full_moves
            .parse::<usize>()
            .expect("Incorrect fen + bad handling = exception :/ [full moves]");
        let turn = match turn {
            "w" => Player::White,
            "b" => Player::Black,
            _ => panic!("Incorrect turn parameter in fen description ({})", turn),
        };
        pos.half_move_count = 2 * full_moves + turn as usize;
        pos.fifty_mv = hf_mv_until_100
            .parse::<usize>()
            .expect("Incorrect input for fifty move rule");

        for c in castles.chars() {
            match c {
                '-' => break,
                'K' => pos.castles.x[Player::White as usize].x[Castle::Short as usize] = true,
                'Q' => pos.castles.x[Player::White as usize].x[Castle::Long as usize] = true,
                'k' => pos.castles.x[Player::Black as usize].x[Castle::Short as usize] = true,
                'q' => pos.castles.x[Player::Black as usize].x[Castle::Long as usize] = true,
                _ => panic!("Incorrect castling rights in fen description ({})", castles),
            }
        }

        pos.en_passant = match BBSquare::from_str(en_passant) {
            None => SpecialBB::Empty.declass(),
            Some(x) => x.declass(),
        };

        pos
    }
}

////// Print functions

impl Position {
    // TODO: replace with fen interpretation / or other
    pub fn pretty_print(&self) {
        debug_assert_eq!(File::G.declass() & Rank::R5, Square::g5.declass());

        let repr = PlayerStorage::from([['♟', '♞', '♝', '♜', '♛', '♚'], [
            '♙', '♘', '♗', '♖', '♕', '♔',
        ]]);
        println!("┏━━━┯━━━┯━━━┯━━━┯━━━┯━━━┯━━━┯━━━┓");
        // dirty, but anyway

        for rank in 0..8 {
            print!("┃");
            for file in 0..8 {
                print!(" ");
                let bb_sq = Bitboard(GenericBB(1 << (8 * (7 - rank) + file)));
                let mut printed = false;
                // only one in bb_sq but this is for safety
                for sq in bb_sq {
                    for pl in 0..2 {
                        for pc in 0..Piece::COUNT {
                            let pl = Player::from_usize(pl).other();
                            let pc = Piece::from_usize(pc).unwrap();
                            if self.pos[pl][pc] & sq != SpecialBB::Empty.declass() {
                                printed = true;
                                print!("{}", repr[pl][pc as usize]);
                                break;
                            }
                        }
                    }
                }
                if !printed {
                    print!(" ");
                }
                print!(" ");
                if file != 7 {
                    print!("│");
                }
            }
            println!("┃{}", 7 - rank + 1);
            if rank != 7 {
                println!("┠───┼───┼───┼───┼───┼───┼───┼───┨");
            }
        }
        println!("┗━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┛");
        println!("  a   b   c   d   e   f   g   h");
        //println!("Debug : {:#?}", AugmentedPos::create(&mut self.clone()));
    }
}

use crate::eval::{self, Eval, EvalState};
impl Position {
    /*#[cfg(debug_assertions)]
    fn assert_squares_occupied_only_once(&self) {
        let mut occupied = 0;
        for bb_array in self.pos.iter() {
            for bb in bb_array.pieces.iter() {
                let xor = occupied ^ bb;
                let or = occupied | bb;
                if (xor != or) {
                    bb_print(occupied);
                    bb_print(*bb);
                    bb_print(or);
                    bb_print(xor);
                    self.pretty_print();
                    debug_assert_eq!(xor, or);
                }

                occupied = xor;
            }
        }
    }*/
    pub fn eval_minimax(
        &mut self,
        depth: usize,
        eval_fn: &eval::EvalFun,
        pregen: &Lookup,
    ) -> Result<EvalState, ()> {
        //#[cfg(debug_assertions)]
        //self.assert_squares_occupied_only_once();
        match depth {
            // TODO: add quiescent search for depth 1
            0 => {
                let a = AugmentedPos::list_issues(&self, pregen);
                match a {
                    Err(()) => Err(()),
                    Ok(_) => Ok(EvalState::new(&eval_fn(self))),
                }
            }
            _ => {
                let turn = self.turn();
                let movelist = AugmentedPos::list_issues(self, pregen)?;
                //let is_check = !meta.is_check();

                let mut best_eval = EvalState::new(&Eval::m0(turn.other()));
                let mut best_move = None;
                let mut explored = 0;
                for m in movelist.iter() {
                    self.stack(m);
                    let eval = self.eval_minimax(depth - 1, eval_fn, pregen);
                    self.unstack(m);
                    match eval {
                        Err(()) => continue,
                        Ok(eval) => {
                            if (EvalState::pick_best_for(turn, &best_eval, &eval)) == 1 {
                                best_eval = eval;
                                best_move = Some(m);
                            }
                            if explored == 0 {
                                best_move = Some(m)
                            }
                            explored += 1;
                        }
                    }
                }
                // if there were no legal moves and no check, set to draw instead of M0
                if explored == 0
                /*&& is_check*/
                {
                    best_eval = EvalState::new(&Eval::draw());
                    Ok(best_eval)
                } else if explored != 0 {
                    let m = best_move.unwrap();
                    best_eval.nest(&m);
                    Ok(best_eval)
                } else {
                    Ok(best_eval)
                }
            }
        }
    }
}
