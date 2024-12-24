use super::Bitboard;
use super::Player;
use super::bitboard::{GenericBB, SpecialBB, Square, ToBB};

// TODO: could replace with an map
pub fn generate_king(king: Bitboard<Square>) -> Bitboard<GenericBB> {
    // let col = lsu(king) | king | lsd(king);
    // (lsl(col) | lsr(col) | col) ^ king
    let u = king + 1;
    let d = king - 1;
    let l = king << 1;
    let r = king >> 1;

    let lu = u << 1;
    let ld = d << 1;
    let ru = u >> 1;
    let rd = d >> 1;

    u | d | l | r | lu | ld | ru | rd
}

pub fn generate_knights(knights: Bitboard<GenericBB>) -> Bitboard<GenericBB> {
    let uu = knights + 2;
    let dd = knights - 2;
    let ll = knights << 2;
    let rr = knights >> 2;

    let uul = uu << 1;
    let uur = uu >> 1;

    let ddl = dd << 1;
    let ddr = dd >> 1;

    let rru = rr + 1;
    let rrd = rr - 1;

    let llu = ll + 1;
    let lld = ll - 1;

    let res = uul | uur | ddl | ddr | rru | rrd | llu | lld;

    /*#[cfg(debug_assertions)]
    println!(
        "Generate knight attacks src = {:?} , res = {:?}",
        knights, res
    );*/

    res
}

pub fn generate_rooks(
    p: Bitboard<GenericBB>,
    blockers: Bitboard<GenericBB>,
) -> Bitboard<GenericBB> {
    let mut attacked = Bitboard(SpecialBB::Empty).declass();
    let mut u = p;
    let mut d = p;
    let mut l = p;
    let mut r = p;

    let mut nb_iter = 0;
    loop {
        u = Bitboard(u & (p | !blockers)) + 1;
        d = Bitboard(d & (p | !blockers)) - 1;
        l = Bitboard(l & (p | !blockers)) << 1;
        r = Bitboard(r & (p | !blockers)) >> 1;
        attacked = attacked | u | d | l | r;
        nb_iter += 1;
        if nb_iter == 7 {
            break;
        }
    }

    /*#[cfg(debug_assertions)]
    println!("Generate rook attacks src = {:?} , res = {:?}", p, attacked);*/

    attacked
}

pub fn generate_bishops(
    p: Bitboard<GenericBB>,
    blockers: Bitboard<GenericBB>,
) -> Bitboard<GenericBB> {
    let mut attacked = Bitboard(SpecialBB::Empty).declass();
    let mut ul = p;
    let mut ur = p;
    let mut dl = p;
    let mut dr = p;
    for _i in 0..7 {
        ul = (ul & (!blockers | p)) + 1 << 1;
        ur = (ur & (!blockers | p)) + 1 >> 1;
        dl = (dl & (!blockers | p)) - 1 << 1;
        dr = (dr & (!blockers | p)) - 1 >> 1;
        attacked = attacked | ul | dr | ur | dl;
    }
    attacked
}

pub fn generate_queens(
    p: Bitboard<GenericBB>,
    blockers: Bitboard<GenericBB>,
) -> Bitboard<GenericBB> {
    generate_bishops(p, blockers) | generate_rooks(p, blockers)
}

pub fn generate_pawns(p: Bitboard<GenericBB>, pl: Player) -> Bitboard<GenericBB> {
    let l = p << 1;
    let r = p >> 1;
    match pl {
        Player::White => (l + 1) | (r + 1),
        Player::Black => (l - 1) | (r - 1),
    }
}
