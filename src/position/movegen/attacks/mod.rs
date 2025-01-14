//! Contains attack generation routines for each piece type.
//! The attack generation is implemented in two ways:
//! - Dynamic attack generation (`dyn_attack.rs`): Generates on-demand attacks for each piece type
//! - Static attack generation (`static_attacks.rs`): Uses pregenerated lookup tables for sliding pieces
//!
//! The module exposes a single interface through feature flags:
//! - With `static_attacks` enabled, uses the static lookup tables for bishops/rooks/queens
//! - Without `static_attacks`, uses dynamic generation for all pieces

mod dyn_attacks;

#[cfg(feature = "static_attacks")]
mod static_attacks;

#[cfg(feature = "static_attacks")]
pub use static_attacks::{generate_bishops, generate_queens, generate_rooks};

#[cfg(not(feature = "static_attacks"))]
pub use dyn_attacks::{generate_bishops, generate_queens, generate_rooks};

pub use dyn_attacks::{generate_king, generate_knights, generate_pawns};
mod tests {
    use movegen::attacks::{
        dyn_attacks::{generate_bishops, generate_queens, generate_rooks},
        generate_king, generate_knights, generate_pawns,
    };

    use crate::prelude::*;
    #[test]
    fn test_king_attacks() {
        let king = Bitboard(Square::d4);
        let expected = Bitboard(Square::c3)
            | Bitboard(Square::d3)
            | Bitboard(Square::e3)
            | Bitboard(Square::c4)
            | Bitboard(Square::e4)
            | Bitboard(Square::c5)
            | Bitboard(Square::d5)
            | Bitboard(Square::e5);

        assert_eq!(generate_king(king), expected);
    }

    #[test]
    fn test_knight_attacks() {
        let knight = Bitboard(Square::d4);
        let expected = Bitboard(Square::b3)
            | Bitboard(Square::c2)
            | Bitboard(Square::e2)
            | Bitboard(Square::f3)
            | Bitboard(Square::f5)
            | Bitboard(Square::e6)
            | Bitboard(Square::c6)
            | Bitboard(Square::b5);
        assert_eq!(generate_knights(knight.declass()), expected);
    }

    #[test]
    fn test_bishop_attacks() {
        let bishop = Bitboard(Square::d4);
        let blockers = Bitboard(Square::c5) | Bitboard(Square::e3);
        let expected = Bitboard(Square::a1)
            | Bitboard(Square::b2)
            | Bitboard(Square::c3)
            | Bitboard(Square::e5)
            | Bitboard(Square::f6)
            | Bitboard(Square::g7)
            | Bitboard(Square::h8)
            | Bitboard(Square::c5)
            | Bitboard(Square::e3);
        assert_eq!(generate_bishops(bishop.declass(), blockers), expected);
    }

    #[test]
    fn test_rook_attacks() {
        let rook = Bitboard(Square::d4);
        let blockers = Bitboard(Square::f4);
        let expected = (Rank::R4.declass() | File::D.declass())
            & !rook
            & !Bitboard(Square::g4)
            & !Bitboard(Square::h4);
        assert_eq!(generate_rooks(rook.declass(), blockers.declass()), expected);
    }

    #[test]
    fn test_queen_attacks() {
        let queen = Bitboard(Square::a1);
        let blockers = Bitboard(Square::a3) | Bitboard(Square::c3) | Bitboard(Square::c1);
        let expected = Bitboard(Square::b2)
            | Bitboard(Square::c3)
            | Bitboard(Square::a2)
            | Bitboard(Square::a3)
            | Bitboard(Square::b1)
            | Bitboard(Square::c1);
        assert_eq!(
            generate_queens(queen.declass(), blockers.declass()),
            expected
        );
    }

    #[test]
    fn test_pawn_attacks() {
        let pawn = Bitboard(Square::d4);

        // White pawn
        let expected_w = Bitboard(Square::c5) | Bitboard(Square::e5);
        assert_eq!(generate_pawns(pawn.declass(), Player::White), expected_w);

        // Black pawn
        let expected_b = Bitboard(Square::c3) | Bitboard(Square::e3);
        assert_eq!(generate_pawns(pawn.declass(), Player::Black), expected_b);
    }
}
#[cfg(test)]
mod bench {
    extern crate test;
    use std::random::random;

    use test::Bencher;

    use super::dyn_attacks;
    use crate::prelude::*;

    fn gen_random_input() -> (Bitboard<GenericBB>, Bitboard<GenericBB>) {
        let sq = (1 as u64) << (random::<u8>() % 64);
        let square = Square::from_bb(&Bitboard(GenericBB(sq))).unwrap();
        let blockers = Bitboard(GenericBB(random()));
        (square.declass(), blockers)
    }

    #[bench]
    fn bench_gen_random(b: &mut Bencher) {
        b.iter(|| test::black_box(gen_random_input()));
    }

    #[bench]
    fn bench_dynamic_queen(b: &mut Bencher) {
        b.iter(|| {
            let (square, blockers) = gen_random_input();
            test::black_box(dyn_attacks::generate_queens(square, blockers))
        });
    }

    #[cfg(feature = "static_attacks")]
    #[bench]
    fn bench_static_queen(b: &mut Bencher) {
        use super::static_attacks;
        static_attacks::STATIC_ATTACKS.ensure_init();

        b.iter(|| {
            let (square, blockers) = gen_random_input();
            test::black_box(static_attacks::generate_queens(square, blockers))
        });
    }

    #[cfg(feature = "static_attacks")]
    #[test]
    fn bench_compare_queen() {
        use super::static_attacks;
        static_attacks::STATIC_ATTACKS.ensure_init();

        let mut dyn_total = 0;
        let mut static_total = 0;
        let runs = 1000;

        for _ in 0..runs {
            let (square, blockers) = gen_random_input();

            let dyn_start = std::time::Instant::now();
            test::black_box(dyn_attacks::generate_queens(square, blockers));
            let dyn_time = dyn_start.elapsed();

            let static_start = std::time::Instant::now();
            test::black_box(static_attacks::generate_queens(square, blockers));
            let static_time = static_start.elapsed();

            dyn_total += dyn_time.as_nanos();
            static_total += static_time.as_nanos();
        }

        println!("Queen avg dynamic: {}ns", dyn_total / runs);
        println!("Queen avg static: {}ns", static_total / runs);
    }

    #[bench]
    fn bench_dynamic_bishop(b: &mut Bencher) {
        b.iter(|| {
            let (square, blockers) = gen_random_input();
            test::black_box(dyn_attacks::generate_bishops(square, blockers))
        });
    }

    #[cfg(feature = "static_attacks")]
    #[bench]
    fn bench_static_bishop(b: &mut Bencher) {
        use super::static_attacks;
        static_attacks::STATIC_ATTACKS.ensure_init();

        b.iter(|| {
            let (square, blockers) = gen_random_input();
            test::black_box(static_attacks::generate_bishops(square, blockers))
        });
    }

    #[bench]
    fn bench_dynamic_rook(b: &mut Bencher) {
        b.iter(|| {
            let (square, blockers) = gen_random_input();
            test::black_box(dyn_attacks::generate_rooks(square, blockers))
        });
    }

    #[cfg(feature = "static_attacks")]
    #[bench]
    fn bench_static_rook(b: &mut Bencher) {
        use super::static_attacks;
        static_attacks::STATIC_ATTACKS.ensure_init();

        b.iter(|| {
            let (square, blockers) = gen_random_input();
            test::black_box(static_attacks::generate_rooks(square, blockers))
        });
    }
}
