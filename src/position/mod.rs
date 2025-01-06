pub trait UciNotation {
    fn to_uci(&self) -> String;
}
pub trait Parsing {
    type Resulting: ?Sized;
    fn from_str(s: &str) -> Self::Resulting;
}

use bitboard::{BBSquare, Bitboard, File, FromBB, GenericBB, Rank, SpecialBB, Square, ToBB};

use castle::{CASTLES_ALL_ALLOWED, CASTLES_ALL_FORBIDDEN, Castle, CastleData};
use movegen::{AtomicMove, AugmentedPos, Change, Move, PartialMove, Promotion, StandardMove};
use zobrist::{zobrist_hash_bitboard, zobrist_hash_square};

pub mod bitboard;
mod castle;
pub mod movegen;
pub mod piece;
pub mod zobrist;

use piece::Piece;

use crate::uci::UciOutputStream;

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
    #[inline(always)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PieceSet {
    pawns: Bitboard<GenericBB>,
    knights: Bitboard<GenericBB>,
    bishops: Bitboard<GenericBB>,
    rooks: Bitboard<GenericBB>,
    queens: Bitboard<GenericBB>,
    king: Bitboard<GenericBB>, // TODO: move to squares
}
#[derive(Clone, Copy, Debug, PartialEq)]
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
    fn attacks(&self, player: Player, blockers: Bitboard<GenericBB>) -> Bitboard<GenericBB> {
        movegen::attacks::generate_pawns(self[Piece::Pawn], player)
            | movegen::attacks::generate_knights(self[Piece::Knight])
            | movegen::attacks::generate_bishops(self[Piece::Bishop] | self[Piece::Queen], blockers)
            | movegen::attacks::generate_rooks(self[Piece::Rook] | self[Piece::Queen], blockers)
            | movegen::attacks::generate_king(Square::from_bb(&self[Piece::King]).unwrap())
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub fifty_mv: u16,
    half_move_count: u16,
    pos: PieceData,
    castles: CastleData,
    en_passant: Bitboard<GenericBB>,
    zobrist: usize,
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
            zobrist: zobrist::zobrist_hash_playerstorage(&PieceData::startingpos()),
        }
    }
    pub fn empty() -> Self {
        Self {
            half_move_count: 0,
            fifty_mv: 0,
            pos: PieceData::empty(),
            castles: CASTLES_ALL_FORBIDDEN,
            en_passant: SpecialBB::Empty.declass(),
            zobrist: zobrist::zobrist_hash_playerstorage(&PieceData::empty()),
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

    fn stack_change_rev(&mut self, ch: &Change, pl: Player) {
        // en passant case
        if ch.piece() == Piece::Pawn
            && ch.cap() == Some(Piece::Pawn)
            && ch.bitboard() & self.en_passant != Bitboard(SpecialBB::Empty).declass()
        {
            // toggle ennemy pawn
            self.pos[pl.other()][Piece::Pawn] = self.pos[pl.other()][Piece::Pawn] ^ {
                match pl.other() {
                    Player::Black => (ch.bitboard() & self.en_passant) - 1,
                    Player::White => (ch.bitboard() & self.en_passant) + 1,
                }
            }
        } else {
            match ch.cap() {
                Some(cap) => {
                    self.pos[pl.other()][cap] ^= ch.dest();
                    self.zobrist ^= zobrist_hash_bitboard(ch.dest().declass(), cap, pl.other());
                }
                None => (),
            }
        }
        self.pos[pl][ch.piece()] ^= ch.bitboard();

        self.zobrist ^= zobrist_hash_bitboard(ch.bitboard(), ch.piece(), pl);
    }
    fn stack_prom_rev(&mut self, pr: &Promotion) {
        let turn = self.turn();

        self.pos[turn][pr.new_piece()] ^= pr.dest();
        self.pos[turn][Piece::Pawn] ^= pr.from();

        match pr.cap() {
            Some(cap) => {
                self.pos[turn.other()][cap] ^= pr.dest();
                self.zobrist ^= zobrist_hash_bitboard(pr.dest().declass(), cap, turn.other());
            }
            None => (),
        }

        self.zobrist ^= zobrist_hash_square(pr.dest(), pr.new_piece(), turn);
        self.zobrist ^= zobrist_hash_square(pr.from(), Piece::Pawn, turn);
    }
    fn stack_atomic_rev(&mut self, amv: &AtomicMove) {
        match amv {
            AtomicMove::PieceMoved(ch) => self.stack_change_rev(ch, self.turn()),
            AtomicMove::PiecePromoted(pr) => self.stack_prom_rev(pr),
        }
    }
    fn stack_std_rev(&mut self, smv: &StandardMove) {
        self.castles.stack_rev(&smv.cas);
        self.stack_atomic_rev(&smv.mv);
    }
    fn stack_castle_rev(&mut self, cs: &Castle) {
        let turn = self.turn();
        match cs {
            Castle::Short => {
                let king_ch = turn.backrank().declass() & (File::E.declass() | File::G.declass());
                let rook_ch = turn.backrank().declass() & (File::H.declass() | File::F.declass());

                self.pos[turn][Piece::King] ^= king_ch;
                self.pos[turn][Piece::Rook] ^= rook_ch;

                self.zobrist ^= zobrist_hash_bitboard(king_ch, Piece::King, turn);
                self.zobrist ^= zobrist_hash_bitboard(rook_ch, Piece::Rook, turn);
            }
            Castle::Long => {
                let king_ch = turn.backrank().declass() & (File::E.declass() | File::C.declass());
                let rook_ch = turn.backrank().declass() & (File::A.declass() | File::D.declass());

                self.pos[turn][Piece::King] ^= king_ch;
                self.pos[turn][Piece::Rook] ^= rook_ch;

                self.zobrist ^= zobrist_hash_bitboard(king_ch, Piece::King, turn);
                self.zobrist ^= zobrist_hash_bitboard(rook_ch, Piece::Rook, turn);
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
        self.fifty_mv ^= mv.fifty_mv();
        self.en_passant ^= mv.en_passant();
        self.half_move_count += 1;
    }

    pub fn unstack(&mut self, mv: &Move) {
        self.half_move_count -= 1;
        self.fifty_mv ^= mv.fifty_mv();
        self.en_passant ^= mv.en_passant();
        self.stack_partial_rev(&mv.partialmove());
    }

    // very unoptimized, should not be called when we can access the move as &mv
    pub fn getmove(&mut self, uci: &str) -> Result<Option<Move>, ()> {
        let meta = AugmentedPos::list_issues(self);
        let moves = match meta {
            Err(()) => return Err(()),
            Ok(meta) => meta,
        };
        for m in moves.iter() {
            if format!("{m}") == uci {
                return Ok(Some(*m));
            }
        }
        Ok(None)
    }
    #[cfg(feature = "perft")]
    pub fn perft_top<O: UciOutputStream>(&mut self, depth: usize) -> usize {
        use crate::uci::UciResponse;

        let mut cache = match depth {
            0..=3 => PerftCache::new(1),
            4..=5 => PerftCache::new(1024 * 1024),
            6.. => PerftCache::new(4 * 1024 * 1024),
        };
        match depth {
            0 => 0,
            _ => {
                let r = AugmentedPos::list_issues(self);
                let ml = match r {
                    Err(()) => return 0,
                    Ok(ml) => ml,
                };

                let mut sum = 0;
                for m in ml.iter() {
                    self.stack(m);
                    let count = self.perft_rec(depth - 1, 1, &mut cache);
                    O::send_response(UciResponse::Raw(format!("{}: {}", m, count).as_str()))
                        .unwrap();
                    sum += count;
                    self.unstack(m);
                }
                #[cfg(debug_assertions)]
                cache.print_stats();
                sum
            }
        }
    }

    fn perft_rec(&mut self, depth: usize, depth_in: usize, cache: &mut PerftCache) -> usize {
        match depth {
            0 => {
                let a = AugmentedPos::list_issues(&self);
                match a {
                    Ok(_) => 1,
                    Err(()) => 0,
                }
            }
            1 => {
                let r = AugmentedPos::list_issues(self);
                match r {
                    Err(()) => 0,
                    Ok(ml) => ml.len(),
                }
            }
            _ => {
                // minimum depth to have transpositions happening
                if depth_in >= 4 && depth >= 2 {
                    match cache[&self] {
                        Some(x) => {
                            //println!("Found entry");
                            if x.depth as usize == depth {
                                //println!("Found transposition {:?}", x);
                                return x.nodes as usize;
                            } else {
                            }
                        }
                        None => (),
                    }
                };
                let r = AugmentedPos::list_issues(self);
                let ml = match r {
                    Err(()) => return 0,
                    Ok(ml) => ml,
                };

                let mut sum = 0;
                for m in ml.iter() {
                    self.stack(m);
                    let count = self.perft_rec(depth - 1, depth_in + 1, cache);

                    self.unstack(m);
                    sum += count;
                }
                if depth_in >= 4 && depth >= 2 {
                    cache.push(&self, &PerftInfo {
                        depth: depth as u32,
                        nodes: sum as u32,
                    });
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
            .parse::<u16>()
            .expect("Incorrect fen + bad handling = exception :/ [full moves]");
        let turn = match turn {
            "w" => Player::White,
            "b" => Player::Black,
            _ => panic!("Incorrect turn parameter in fen description ({})", turn),
        };
        pos.half_move_count = 2 * full_moves + turn as u16;
        pos.fifty_mv = hf_mv_until_100
            .parse::<u16>()
            .expect("Incorrect input for fifty move rule");

        for c in castles.chars() {
            match c {
                '-' => break,
                'K' => pos.castles.set(Player::White, Castle::Short, true),
                'Q' => pos.castles.set(Player::White, Castle::Long, true),
                'k' => pos.castles.set(Player::Black, Castle::Short, true),
                'q' => pos.castles.set(Player::Black, Castle::Long, true),
                _ => panic!("Incorrect castling rights in fen description ({})", castles),
            }
        }

        pos.en_passant = match BBSquare::try_from(en_passant) {
            Err(()) => SpecialBB::Empty.declass(),
            Ok(x) => x.declass(),
        };

        pos
    }
}

////// Print functions

impl Position {
    // TODO: replace with fen interpretation / or other
    pub fn pretty_print<O: UciOutputStream>(&self) {
        debug_assert_eq!(File::G.declass() & Rank::R5, Square::g5.declass());

        let repr = PlayerStorage::from([['♟', '♞', '♝', '♜', '♛', '♚'], [
            '♙', '♘', '♗', '♖', '♕', '♔',
        ]]);
        O::send_response(crate::uci::UciResponse::Debug(
            "┏━━━┯━━━┯━━━┯━━━┯━━━┯━━━┯━━━┯━━━┓ ",
        ))
        .unwrap();
        // dirty, but anyway

        for rank in 0..8 {
            let mut s = format!("┃");
            for file in 0..8 {
                s = format!("{s} ");
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
                                s = format!("{s}{}", repr[pl][pc as usize]);
                                break;
                            }
                        }
                    }
                }
                if !printed {
                    s = format!("{s} ");
                }
                s = format!("{s} ");
                if file != 7 {
                    s = format!("{s}│");
                }
            }
            s = format!("{s}┃{}", 7 - rank + 1);
            O::send_response(crate::uci::UciResponse::Debug(s.as_str())).unwrap();
            if rank != 7 {
                O::send_response(crate::uci::UciResponse::Debug(
                    "┠───┼───┼───┼───┼───┼───┼───┼───┨ ",
                ))
                .unwrap();
            }
        }
        O::send_response(crate::uci::UciResponse::Debug(
            "┗━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┷━━━┛ ",
        ))
        .unwrap();
        O::send_response(crate::uci::UciResponse::Debug(
            "  a   b   c   d   e   f   g   h  ",
        ))
        .unwrap();
        //println!("Debug : {:#?}", AugmentedPos::create(&mut self.clone()));
    }
}

use super::tt::{PerftCache, PerftInfo};
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
}

#[cfg(test)]
mod tests {
    extern crate test;
    use std::io::Stdout;

    use test::Bencher;

    use crate::{
        position::{Player, movegen::AugmentedPos},
        uci::{NullUciStream, UciOut, UciOutputStream},
    };

    use super::Position;

    #[cfg(feature = "perft")]
    #[bench]
    fn perft_startpos(b: &mut Bencher) {
        use crate::uci::NullUciStream;

        let mut a = super::Position::startingpos();

        assert_eq!(a.perft_top::<NullUciStream>(1), 20);
        assert_eq!(a.perft_top::<NullUciStream>(2), 400);
        assert_eq!(a.perft_top::<NullUciStream>(3), 8902);

        let mut a = super::Position::startingpos();
        b.iter(|| {
            assert_eq!(a.perft_top::<NullUciStream>(3), 8902);
        });
    }

    #[test]
    fn zobrist() {
        let mut a = super::Position::startingpos();
        let ml = super::AugmentedPos::list_issues(&a).unwrap();
        let initial_hash = super::zobrist::zobrist_hash_playerstorage(&a.pos);
        for m in ml.iter() {
            a.stack(m);
            assert_ne!(
                initial_hash, a.zobrist,
                "Hash collision detected playing a single move (should have changed)"
            );
            a.unstack(m);
        }
        assert_eq!(
            initial_hash, a.zobrist,
            "Hash has been altered in issue exploration phase"
        );
    }

    #[test]
    fn captures_knight() {
        let mut p = Position::from_fen("7k/p7/8/1N6/8/8/8/7K", "w", "-", "-", "0", "0");
        let x = p.getmove("b5a7").expect("Did not find capture").unwrap();
        p.stack(&x);
        assert_eq!(p.half_move_count, 1);
        assert_eq!(p.fifty_mv, 0);
        assert_eq!(p.turn(), Player::Black);
        assert_eq!(AugmentedPos::list_issues(&p).unwrap().len(), 3);
    }

    #[test]
    fn captures_en_passant() {
        let mut p = Position::from_fen("7k/8/8/8/1p6/8/P7/7K", "w", "-", "-", "0", "0");
        let x = p.getmove("a2a4").unwrap().unwrap();
        p.stack(&x);
        assert_eq!(p.half_move_count, 1);
        assert_eq!(p.fifty_mv, 0);
        let x = p.getmove("b4a3").unwrap().unwrap();
        p.stack(&x);
        assert_eq!(p.half_move_count, 2);
        assert_eq!(p.fifty_mv, 0);
        assert_eq!(AugmentedPos::list_issues(&p).unwrap().len(), 3);
    }

    #[test]
    fn promotion() {
        let mut p = Position::from_fen("7k/P7/8/8/8/8/8/7K", "w", "-", "-", "0", "0");
        assert_eq!(
            p.perft_top::<NullUciStream>(1),
            4 + 3,
            "Failed counting moves in promoting position."
        ); // 4 pieces possible + 3 king moves
        //p.perft_top::<UciOut<Stdout>>(1);
        let x = p.getmove("a7a8q").unwrap().unwrap();
        p.stack(&x);
        assert_eq!(
            p.perft_top::<NullUciStream>(1),
            2,
            "Failed promotion to queen"
        ); // king in check
    }
}
