pub trait Parsing {
    type Resulting: ?Sized;
    fn from_str(s: &str) -> Self::Resulting;
}

use castle::{CASTLES_ALL_ALLOWED, CASTLES_ALL_FORBIDDEN, Castle, CastleData};

pub use movegen::{AtomicMove, AugmentedPos, Change, Move, Promotion, StandardMove};

pub mod types;
pub use types::*;
mod castle;
pub mod movegen;
mod zobrist;
use crate::prelude::*;
use crate::uci::UciOutputStream;

pub trait PositionSpec: Sized {
    fn startingpos() -> Self;
    fn empty() -> Self;

    fn pos(&self) -> &PlayerStorage;
    fn turn(&self) -> Player;

    //fn collect_outcomes<R>(self, f: impl Fn(Self) -> R) -> Result<impl FromIterator<R>, ()>;
    fn hash(&self) -> usize;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub fifty_mv: u16,
    half_move_count: u16,
    pos: PlayerStorage,
    castles: CastleData,
    en_passant: Bitboard<GenericBB>,
}

impl PositionSpec for Position {
    fn startingpos() -> Position {
        Position {
            half_move_count: 0,
            fifty_mv: 0,
            pos: PlayerStorageSpec::startingpos(),
            castles: CASTLES_ALL_ALLOWED,
            en_passant: SpecialBB::Empty.declass(),
        }
    }
    fn empty() -> Self {
        Self {
            half_move_count: 0,
            fifty_mv: 0,
            pos: PlayerStorageSpec::empty(),
            castles: CASTLES_ALL_FORBIDDEN,
            en_passant: SpecialBB::Empty.declass(),
        }
    }

    fn pos(&self) -> &PlayerStorage {
        &self.pos
    }

    fn turn(&self) -> Player {
        match self.half_move_count % 2 {
            0 => Player::White,
            _ => Player::Black,
        }
    }

    fn hash(&self) -> usize {
        self.pos.zobrist()
    }
}

impl Position {
    fn simplified_move_outcomes<R>(
        mut self,
        ch: &Move,
        task: impl Fn(&Self, &Move) -> R,
        reduce: impl Fn(R, R) -> R,
    ) -> Option<R> {
        match ch {
            Move::Normal(ch) => {
                log::trace!("listing outcomes for {}-{}", ch.src, ch.dest);
                let turn = self.turn();

                // en passant case
                let en_passant =
                    (ch.piece == Piece::Pawn) && (ch.dest.declass() == self.en_passant);

                // promotion case
                let promotion = (ch.piece == Piece::Pawn)
                    && (ch.dest.declass() & (self.turn().other().backrank()))
                        != SpecialBB::Empty.declass();

                // can be improved
                let en_passant_change = self.en_passant
                    | if ch.piece == Piece::Pawn {
                        if (ch.dest - 2) == ch.src.declass() {
                            ch.dest - 1 // TODO: OP directly on u8
                        } else if (ch.dest + 2) == ch.src.declass() {
                            ch.dest + 1
                        } else {
                            SpecialBB::Empty.declass()
                        }
                    } else {
                        SpecialBB::Empty.declass()
                    };

                if en_passant {
                    if !ch.hint_legal {
                        // TODO: add legal checking
                    }
                    // toggle ennemy pawn
                    let en_passant_target_square = match turn.other() {
                        Player::Black => (ch.dest.declass() & self.en_passant) - 1,
                        Player::White => (ch.dest.declass() & self.en_passant) + 1,
                    }
                    .into_iter()
                    .next()
                    .unwrap();
                    self.pos
                        .remove_piece(turn.other(), Piece::Pawn, en_passant_target_square);
                } else {
                    match self.pos.get((turn.other(), ch.dest.into())) {
                        Some(cap) => {
                            self.pos.remove_piece(turn.other(), cap, ch.dest.into());
                        }
                        None => (),
                    }
                }

                let cda_old = self.castles.clone();

                if ch.piece == Piece::Rook {
                    // moving rook away
                    if (ch.src.declass() & (turn.backrank() & File::A.bb()))
                        != SpecialBB::Empty.declass()
                    {
                        self.castles.set(turn, Castle::Long, false);
                    }
                    if (ch.src.declass() & (turn.backrank() & File::H.bb()))
                        != SpecialBB::Empty.declass()
                    {
                        self.castles.set(turn, Castle::Short, false);
                    }
                }
                if ch.piece == Piece::King {
                    // moving king away
                    self.castles.set(turn, Castle::Short, false);
                    self.castles.set(turn, Castle::Long, false);
                }

                // capturing other player's rook
                if (ch.dest.declass() & (turn.other().backrank() & File::A.bb()))
                    != SpecialBB::Empty.declass()
                {
                    self.castles.set(turn.other(), Castle::Long, false);
                }
                if (ch.dest.declass() & (turn.other().backrank() & File::H.bb()))
                    != SpecialBB::Empty.declass()
                {
                    self.castles.set(turn.other(), Castle::Short, false);
                }

                //// preparations done, now inspecting

                self.en_passant ^= en_passant_change;
                self.fifty_mv += 1;
                self.half_move_count += 1;

                self.pos
                    .move_piece(turn, ch.piece, ch.src.into(), ch.dest.into());
                let res = if promotion {
                    if ch.hint_legal
                        || self.pos.generate_attacks(turn.other()) & self.pos[(turn, Piece::King)]
                            == SpecialBB::Empty.declass()
                    {
                        log::info!("-- legal promotion detected");
                        self.pos.remove_piece(turn, Piece::Pawn, ch.dest.into());
                        let peek = |&p| -> R {
                            self.pos.add_new_piece(turn, p, ch.dest.into());
                            let r = task(&self, &Move::Normal(*ch));
                            self.pos.remove_piece(turn, p, ch.dest.into());
                            r
                        };
                        let mapped = [Piece::Queen, Piece::Bishop, Piece::Rook, Piece::Knight]
                            .iter()
                            .map(peek);
                        mapped.reduce(reduce)
                    } else {
                        log::info!("-- filtered out");
                        self.pos.remove_piece(turn, Piece::Pawn, ch.dest.into());
                        None
                    }
                } else if ch.hint_legal
                    || self.pos.generate_attacks(turn.other()) & self.pos[(turn, Piece::King)]
                        == SpecialBB::Empty.declass()
                {
                    let result = Some(task(&self, &Move::Normal(*ch)));
                    result
                } else {
                    None
                };

                self.pos
                    .move_piece(turn, ch.piece, ch.dest.into(), ch.src.into());
                self.castles = cda_old;
                self.en_passant ^= en_passant_change;
                self.fifty_mv -= 1; // TODO: fifty mv rule
                self.half_move_count -= 1;

                // Clean state
                if en_passant {
                    // toggle ennemy pawn
                    let en_passant_target_square = match turn.other() {
                        Player::Black => (ch.dest.declass() & self.en_passant) + 1,
                        Player::White => (ch.dest.declass() & self.en_passant) - 1,
                    }
                    .into_iter()
                    .next()
                    .unwrap();

                    self.pos
                        .add_new_piece(turn.other(), Piece::Pawn, en_passant_target_square);
                }

                res
            }
            Move::Castle(c, p) => {
                // Castle moves are filtered before, no need to check legality
                match (c, p) {
                    (Castle::Short, Player::White) => {
                        self.pos
                            .move_piece(*p, Piece::King, Square::e1.bb(), Square::g1.bb());
                        self.pos
                            .move_piece(*p, Piece::Rook, Square::h1.bb(), Square::f1.bb());
                    }
                    (Castle::Long, Player::White) => {
                        self.pos
                            .move_piece(*p, Piece::King, Square::e1.bb(), Square::c1.bb());
                        self.pos
                            .move_piece(*p, Piece::Rook, Square::a1.bb(), Square::d1.bb());
                    }
                    (Castle::Short, Player::Black) => {
                        self.pos
                            .move_piece(*p, Piece::King, Square::e8.bb(), Square::g8.bb());
                        self.pos
                            .move_piece(*p, Piece::Rook, Square::h8.bb(), Square::f8.bb());
                    }
                    (Castle::Long, Player::Black) => {
                        self.pos
                            .move_piece(*p, Piece::King, Square::e8.bb(), Square::c8.bb());
                        self.pos
                            .move_piece(*p, Piece::Rook, Square::a8.bb(), Square::d8.bb());
                    }
                };
                let cda_save = self.castles.clone();
                // secretly hoping for compiler to optimize this (these are just bitwise ops)
                self.castles.set(*p, Castle::Short, false);
                self.castles.set(*p, Castle::Long, false);

                let en_passant_change = self.en_passant;
                self.en_passant = SpecialBB::Empty.declass();
                self.fifty_mv += 1;
                self.half_move_count += 1;

                let r = task(&self, &Move::Castle(*c, *p));

                self.half_move_count -= 1;
                self.fifty_mv -= 1;
                self.en_passant = en_passant_change;

                self.castles.copy_selection_player(*p, &cda_save);

                match (c, p) {
                    (Castle::Short, Player::White) => {
                        self.pos.move_piece(
                            Player::White,
                            Piece::King,
                            Square::g1.bb(),
                            Square::e1.bb(),
                        );
                        self.pos.move_piece(
                            Player::White,
                            Piece::Rook,
                            Square::f1.bb(),
                            Square::h1.bb(),
                        );
                    }
                    (Castle::Long, Player::White) => {
                        self.pos.move_piece(
                            Player::White,
                            Piece::King,
                            Square::c1.bb(),
                            Square::e1.bb(),
                        );
                        self.pos.move_piece(
                            Player::White,
                            Piece::Rook,
                            Square::d1.bb(),
                            Square::a1.bb(),
                        );
                    }
                    (Castle::Short, Player::Black) => {
                        self.pos.move_piece(
                            Player::Black,
                            Piece::King,
                            Square::g8.bb(),
                            Square::e8.bb(),
                        );
                        self.pos.move_piece(
                            Player::Black,
                            Piece::Rook,
                            Square::f8.bb(),
                            Square::h8.bb(),
                        );
                    }
                    (Castle::Long, Player::Black) => {
                        self.pos.move_piece(
                            Player::Black,
                            Piece::King,
                            Square::c8.bb(),
                            Square::e8.bb(),
                        );
                        self.pos.move_piece(
                            Player::Black,
                            Piece::Rook,
                            Square::d8.bb(),
                            Square::a8.bb(),
                        );
                    }
                };
                Some(r)
            }
        }
    }

    // very unoptimized, should not be called when we can access the move as &mv
    pub fn getmove(&mut self, uci: &str) -> Result<Option<Move>, ()> {
        let gather_value = |x: Option<Move>, y| x.or(y);

        let a = AugmentedPos::map_issues(
            self,
            |_p, m| match format!("{m}") == uci {
                true => Some(*m),
                false => None,
            },
            gather_value,
        );
        // Here, receiving Option<Option<>> would mean that
        // AugmentedPos::map_issues can fail at two different levels:
        // - outer Option: failed position exploration (filtered out by rules)
        // - inner Option: match with query failed
        // AugmentedPos now can collapse both Options as:
        // - Some(None) would mean all legal positions explored but none matching uci
        // - Some(Some()) would mean all legal positions explored with one matching uci
        // - None would mean no legal positions explored (invalid state)
        Ok(match a {
            Some(x) => x,
            None => None,
        })
    }
    pub fn playmove(self, uci: &str) -> Result<Option<Position>, ()> {
        let gather_value = |x: Option<Position>, y| x.or(y);

        let a = AugmentedPos::map_issues(
            &self,
            |p, m| match format!("{m}") == uci {
                true => Some(*p),
                false => None,
            },
            gather_value,
        );
        // Here, receiving Option<Option<>> would mean that
        // AugmentedPos::map_issues can fail at two different levels:
        // - outer Option: failed position exploration (filtered out by rules)
        // - inner Option: match with query failed
        // AugmentedPos now can collapse both Options as:
        // - Some(None) would mean all legal positions explored but none matching uci
        // - Some(Some()) would mean all legal positions explored with one matching uci
        // - None would mean no legal positions explored (invalid state)
        Ok(match a {
            Some(x) => x,
            None => None,
        })
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
                    pos.pos.add_new_piece(
                        player,
                        piece,
                        Bitboard(GenericBB(1 << sq_index))
                            .into_iter()
                            .next()
                            .unwrap(),
                    );
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

        let repr = [['♟', '♞', '♝', '♜', '♛', '♚'], [
            '♙', '♘', '♗', '♖', '♕', '♔',
        ]];
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
                            if self.pos[(pl, pc)] & sq != SpecialBB::Empty.declass() {
                                printed = true;
                                s = format!("{s}{}", repr[pl as usize][pc as usize]);
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
        log::info!("{:#?}", self);
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use test::Bencher;

    use crate::{NullUciStream, Position, PositionSpec};

    #[cfg(feature = "perft")]
    #[bench]
    fn perft_startpos_3(b: &mut Bencher) {
        use crate::uci::NullUciStream;
        let mut a = super::Position::startingpos();
        b.iter(|| {
            assert_eq!(a.perft_top::<NullUciStream>(std::hint::black_box(3)), 8902);
        });
    }

    /*#[test]
    fn zobrist() {
        let mut a = super::Position::startingpos();
        let ml = super::AugmentedPos::list_issues(&a).unwrap();
        let initial_hash = a.hash();
        for m in ml.iter() {
            a.stack(m);
            assert_ne!(
                initial_hash,
                a.hash(),
                "Hash collision detected playing a single move (should have changed)"
            );
            a.unstack(m);
        }
        assert_eq!(
            initial_hash,
            a.hash(),
            "Hash has been altered in issue exploration phase"
        );
    }*/

    #[test]
    fn captures_en_passant() {
        let p = Position::from_fen("7k/8/8/8/1p6/8/P7/7K", "w", "-", "-", "0", "0");
        let p = p.playmove("a2a4").unwrap().unwrap();
        assert_eq!(p.half_move_count, 1);
        assert_eq!(p.fifty_mv, 0);
        let mut p = p.playmove("b4a3").unwrap().unwrap();
        assert_eq!(p.half_move_count, 2);
        assert_eq!(p.fifty_mv, 0);
        assert_eq!(p.perft_top::<NullUciStream>(1), 3);
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
        //let x = p.getmove("a7a8q").unwrap().unwrap();
        //p.stack(&x);
        //assert_eq!(
        //    p.perft_top::<NullUciStream>(1),
        //    2,
        //    "Failed promotion to queen"
        //); // king in check
    }
}

#[test]
fn pawn_up() {
    perft_test_batch(
        "KP vs k",
        &[1, 5, 15, 96, 574, 4184, 23973, 181758, 1151913],
        "k7/8/8/8/8/8/P7/7K",
        "w",
        "-",
        "-",
        "0",
        "0",
    )
}

#[test]
fn knight_up() {
    perft_test_batch(
        "KP vs k",
        &[1, 6, 18, 162, 932, 9116, 50004, 533415],
        "k7/8/8/8/8/8/N7/7K",
        "w",
        "-",
        "-",
        "0",
        "0",
    )
}

#[test]
fn bishop_up() {
    perft_test_batch(
        "KB vs k",
        &[1, 10, 29, 363, 1986, 26104, 140746, 1937534],
        "k7/8/8/8/8/8/B7/7K",
        "w",
        "-",
        "-",
        "0",
        "0",
    )
}

#[test]
fn random_opening() {
    perft_test_batch(
        "Random Opening",
        &[1, 30, 1449, 43690, 1983559, 60712083],
        "r3k2r/ppp2ppp/2n1bn2/2b1p3/4P3/2N2N2/PPPP1PPP/R1B1KB1R",
        "w",
        "KQkq",
        "-",
        "0",
        "1",
    )
}

#[test]
fn perft_startpos_extensive() {
    perft_test_batch(
        "Startpos",
        &[1, 20, 400, 8902, 197281],
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR",
        "w",
        "KQkq",
        "-",
        "0",
        "1",
    );
}
#[cfg(test)]
fn perft_test_batch(
    name: &str,
    depths: &[usize],
    fen: &str,
    turn: &str,
    castles: &str,
    en_passant: &str,
    hf: &str,
    fm: &str,
) {
    for i in 0..depths.len() {
        perft_test(name, i, depths[i], fen, turn, castles, en_passant, hf, fm);
    }
}

#[cfg(test)]
fn perft_test(
    name: &str,
    depth: usize,
    expected: usize,
    fen: &str,
    turn: &str,
    castles: &str,
    en_passant: &str,
    hf: &str,
    fm: &str,
) {
    let mut p = Position::from_fen(fen, turn, castles, en_passant, hf, fm);
    assert_eq!(
        p.perft_top::<UciOut<std::io::Sink>>(depth),
        expected,
        "[Failed Perft [ d {depth} | {name:?} ] ({} {} {} {} {} {}).",
        fen.to_string(),
        turn.to_string(),
        castles.to_string(),
        en_passant.to_string(),
        hf.to_string(),
        fm.to_string()
    );
}
