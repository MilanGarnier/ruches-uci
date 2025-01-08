use std::fmt::{Debug, Display};

/// Bitboard is basically a u64
#[allow(non_camel_case_types)]
pub type bb64 = u64;

/// Object to handle bitboards:
/// Provides methods to iterate, represent, etc.
/// To implement it for a type, simply implement the ToBB64 trait for the specific type
///     You will get in exchange all functions within the BitboardUnsafeOps trait (as well as op overloading)
/// To provide BitboardOpsKeepProperty, implement the FromBB64 trait
/// TODO: replace the bb64 type by implementing const add, shift etc.
#[derive(Clone, Copy)]
pub struct Bitboard<T>(pub T);

pub trait ToBB64 {
    fn to_bb64(&self) -> bb64;
}
impl<T: ToBB64> ToBB64 for Bitboard<T> {
    #[inline(always)]
    fn to_bb64(&self) -> bb64 {
        self.0.to_bb64()
    }
}

impl<T: ToBB64> PartialEq for Bitboard<T> {
    #[inline(always)]
    fn eq(&self, x: &Bitboard<T>) -> bool {
        self.to_bb64() == x.to_bb64()
    }
}

/*
impl<T: ToBB64> ToBB64 for &T {
    fn to_bb64(&self) -> &bb64 {
        T::to_bb64(&self)
    }
    fn to_bb64(&self) -> bb64 {
        T::to_bb64(&self)
    }
}*/

pub trait FromBB64<T, U: ToBB64> {
    unsafe fn from_bb64_nochecks(_: &U) -> T;
    fn from_bb64(_: &U) -> Option<T>;
}

pub trait ToBB: ToBB64 {
    #[inline(always)]
    fn bb(&self) -> Bitboard<&Self> {
        return Bitboard(&self);
    }
    #[inline(always)]
    fn declass(&self) -> Bitboard<GenericBB> {
        return Bitboard(GenericBB(self.to_bb64()));
    }
}

impl<T: ToBB64> ToBB for T {}
pub trait FromBB<T: ToBB64 + FromBB64<T, U>, U: ToBB64> {
    fn from_bb(x: &U) -> Option<Bitboard<T>>;
}
impl<T: Copy + ToBB64 + FromBB64<T, U>, U: ToBB64 + Clone> FromBB<T, U> for Bitboard<T> {
    fn from_bb(x: &U) -> Option<Bitboard<T>> {
        match T::from_bb64(x) {
            None => None,
            Some(x) => Some(Bitboard(x)),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct GenericBB(pub bb64);
/// enum SpecialBB
/// enum File
/// enum Rank
/// enum Square
/// enum PackedSquare

impl ToBB64 for SpecialBB {
    #[inline(always)]
    fn to_bb64(&self) -> bb64 {
        *self as bb64
        //unsafe { std::mem::transmute(self) }
    }
}
impl ToBB64 for GenericBB {
    #[inline(always)]
    fn to_bb64(&self) -> bb64 {
        self.0 as bb64
    }
}
impl ToBB64 for Square {
    #[inline(always)]
    fn to_bb64(&self) -> bb64 {
        *self as bb64
        //unsafe { std::mem::transmute(self) }
    }
}
impl ToBB64 for Rank {
    #[inline(always)]
    fn to_bb64(&self) -> bb64 {
        *self as bb64
        //unsafe { std::mem::transmute(self) }
    }
}
impl ToBB64 for File {
    #[inline(always)]
    fn to_bb64(&self) -> bb64 {
        *self as bb64
        //unsafe { std::mem::transmute(self) }
    }
}

pub trait BitboardFastOps<T: ToBB64>: ToBB64 {
    #[inline(always)]
    fn lsu(&self) -> Bitboard<GenericBB> {
        Bitboard(GenericBB(self.to_bb64() << 8))
    }

    #[inline(always)]
    fn lsd(&self) -> Bitboard<GenericBB> {
        Bitboard(GenericBB(self.to_bb64() >> 8))
    }

    #[inline(always)]
    fn lsr(&self) -> Bitboard<GenericBB> {
        Bitboard(GenericBB((self.to_bb64() << 1) & !(0x0101010101010101)))
    }

    #[inline(always)]
    fn lsl(&self) -> Bitboard<GenericBB> {
        Bitboard(GenericBB(
            (self.to_bb64() >> 1) & !(0x0101010101010101 << 7),
        ))
    }
    #[inline(always)]
    fn fn_bitand(&self, rhs: &T) -> Bitboard<GenericBB> {
        Bitboard(GenericBB(self.to_bb64() & rhs.to_bb64()))
    }

    #[inline(always)]
    fn fn_bitor(&self, rhs: &T) -> Bitboard<GenericBB> {
        Bitboard(GenericBB(self.to_bb64() | rhs.to_bb64()))
    }

    #[inline(always)]
    fn fn_bitxor(&self, rhs: &T) -> Bitboard<GenericBB> {
        Bitboard(GenericBB(self.to_bb64() ^ rhs.to_bb64()))
    }

    #[inline(always)]
    fn fn_bitnot(&self) -> Bitboard<GenericBB> {
        Bitboard(GenericBB(!self.to_bb64()))
    }
}

impl<T: ToBB64, U: ToBB64> BitboardFastOps<T> for U {}

// Iterate over squares contained into any bitboard
impl Iterator for Bitboard<GenericBB> {
    type Item = Bitboard<Square>;

    fn next(&mut self) -> Option<Self::Item> {
        let a = self.to_bb64() & (self.to_bb64().wrapping_sub(1));
        let ex = a ^ self.to_bb64();

        self.0.0 = a;

        if ex == Bitboard(SpecialBB::Empty).to_bb64() {
            None
        } else {
            unsafe {
                Some(Bitboard(Square::from_bb64_nochecks(&Bitboard(GenericBB(
                    ex,
                )))))
            }
        }
    }
}

impl<U: ToBB64> std::ops::BitAnd<U> for Bitboard<GenericBB> {
    type Output = Bitboard<GenericBB>;
    #[inline(always)]
    fn bitand(self, rhs: U) -> Self::Output {
        self.fn_bitand(&rhs)
    }
}

impl<U: ToBB64> std::ops::BitAndAssign<U> for Bitboard<GenericBB> {
    fn bitand_assign(&mut self, rhs: U) {
        self.0.0 &= rhs.to_bb64()
    }
}

impl std::ops::BitAnd<Bitboard<File>> for Bitboard<Rank> {
    type Output = Bitboard<Square>;
    #[inline(always)]
    fn bitand(self, rhs: Bitboard<File>) -> Self::Output {
        let x = self.to_bb64() & rhs.to_bb64();
        let x = Bitboard(GenericBB(x));
        let sq = unsafe { Square::from_bb64_nochecks(&x) };
        Bitboard(sq)
    }
}
impl std::ops::BitAnd<Bitboard<Rank>> for Bitboard<File> {
    type Output = Bitboard<Square>;
    #[inline(always)]
    fn bitand(self, rhs: Bitboard<Rank>) -> Self::Output {
        rhs & self
    }
}

impl<T: ToBB64, U: ToBB64> std::ops::BitOr<U> for Bitboard<T> {
    type Output = Bitboard<GenericBB>;
    #[inline(always)]
    fn bitor(self, rhs: U) -> Self::Output {
        self.fn_bitor(&rhs)
    }
}

impl<U: ToBB64> std::ops::BitOrAssign<U> for Bitboard<GenericBB> {
    fn bitor_assign(&mut self, rhs: U) {
        self.0.0 |= rhs.to_bb64()
    }
}

impl<T: ToBB64, U: ToBB64> std::ops::BitXor<Bitboard<U>> for Bitboard<T> {
    type Output = Bitboard<GenericBB>;
    #[inline(always)]
    fn bitxor(self, rhs: Bitboard<U>) -> Self::Output {
        self.fn_bitxor(&rhs)
    }
}

impl<U: ToBB64> std::ops::BitXorAssign<U> for Bitboard<GenericBB> {
    fn bitxor_assign(&mut self, rhs: U) {
        self.0.0 ^= rhs.to_bb64()
    }
}

impl<T: ToBB64> std::ops::Not for Bitboard<T> {
    type Output = Bitboard<GenericBB>;
    #[inline(always)]
    fn not(self) -> Self::Output {
        <Bitboard<T> as BitboardFastOps<T>>::fn_bitnot(&self)
    }
}
impl<T: ToBB64> std::ops::Shl<usize> for Bitboard<T> {
    type Output = Bitboard<GenericBB>;
    #[inline(always)]
    fn shl(self, rhs: usize) -> Self::Output {
        let mut o = self.declass();
        for _i in 0..rhs {
            o = <Bitboard<GenericBB> as BitboardFastOps<T>>::lsl(&o);
        }
        o
    }
}

impl std::ops::ShlAssign<usize> for Bitboard<GenericBB> {
    fn shl_assign(&mut self, rhs: usize) {
        for _i in 0..rhs {
            self.0.0 = <Bitboard<GenericBB> as BitboardFastOps<GenericBB>>::lsl(self).to_bb64();
        }
    }
}

impl std::ops::ShrAssign<usize> for Bitboard<GenericBB> {
    fn shr_assign(&mut self, rhs: usize) {
        for _i in 0..rhs {
            self.0.0 = <Bitboard<GenericBB> as BitboardFastOps<GenericBB>>::lsr(self).to_bb64();
        }
    }
}

impl std::ops::AddAssign<usize> for Bitboard<GenericBB> {
    fn add_assign(&mut self, rhs: usize) {
        for _i in 0..rhs {
            self.0.0 = <Bitboard<GenericBB> as BitboardFastOps<GenericBB>>::lsu(self).to_bb64();
        }
    }
}

impl std::ops::SubAssign<usize> for Bitboard<GenericBB> {
    fn sub_assign(&mut self, rhs: usize) {
        for _i in 0..rhs {
            self.0.0 = <Bitboard<GenericBB> as BitboardFastOps<GenericBB>>::lsd(self).to_bb64();
        }
    }
}

impl<T: ToBB64> std::ops::Shr<usize> for Bitboard<T> {
    type Output = Bitboard<GenericBB>;
    #[inline(always)]
    fn shr(self, rhs: usize) -> Self::Output {
        let mut o = self.declass();
        for _i in 0..rhs {
            o = <Bitboard<GenericBB> as BitboardFastOps<T>>::lsr(&o);
        }
        o
    }
}

impl<T: ToBB64> std::ops::Add<usize> for Bitboard<T> {
    type Output = Bitboard<GenericBB>;
    #[inline(always)]
    fn add(self, rhs: usize) -> Self::Output {
        let mut o = self.declass();
        for _i in 0..rhs {
            o = <Bitboard<GenericBB> as BitboardFastOps<GenericBB>>::lsu(&o);
        }
        o
    }
}

impl<T: ToBB64> std::ops::Sub<usize> for Bitboard<T> {
    type Output = Bitboard<GenericBB>;
    #[inline(always)]
    fn sub(self, rhs: usize) -> Self::Output {
        let mut o = self.declass();
        for _i in 0..rhs {
            o = <Bitboard<GenericBB> as BitboardFastOps<GenericBB>>::lsd(&o);
        }
        o
    }
}

impl<T: Display + ToBB64> Display for Bitboard<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Debug for Bitboard<GenericBB> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bb<Generic>({} ~ {})", self.0.0, self)
    }
}
impl Display for Bitboard<GenericBB> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for sq in *self {
            write!(f, " {sq}")?;
        }
        write!(f, " ]")
    }
}

impl Debug for Bitboard<Square> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}
impl Debug for Bitboard<PackedSquare> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: Bitboard<Square> = self.into();
        write!(f, "{}", x)
    }
}

pub type BBSquare = Bitboard<Square>;

impl TryFrom<&str> for Bitboard<Square> {
    fn try_from(s: &str) -> Result<Self, ()> {
        let mut chars = s.chars();
        let file: Option<char> = chars.nth(0);
        let rank: Option<char> = chars.nth(0);

        let inter = match file {
            None | Some('-') => SpecialBB::Empty.declass(),
            Some(f) => File::from_char(f).declass(),
        } & match rank {
            None => SpecialBB::Empty.declass(),
            Some(f) => File::from_char(f).declass(),
        };
        // either a square or empty
        let x = Square::from_bb(&inter);
        match x {
            None => Err(()),
            Some(x) => Ok(x),
        }
    }

    type Error = ();
}

impl Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", *self)
    }
}
impl Display for Bitboard<PackedSquare> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: Bitboard<Square> = self.into();
        write!(f, "{}", x)
    }
}

impl<T: ToBB64> FromBB64<Square, T> for Square {
    unsafe fn from_bb64_nochecks(b: &T) -> Self {
        unsafe { std::mem::transmute(b.to_bb64()) }
    }

    fn from_bb64(b: &T) -> Option<Self> {
        if Bitboard(GenericBB(b.to_bb64())).count() == 1 {
            unsafe { Some(Self::from_bb64_nochecks(b)) }
        } else {
            None
        }
    }
}

impl<T: ToBB64> FromBB<Square, T> for Square {
    fn from_bb(x: &T) -> Option<Bitboard<Square>> {
        let x = GenericBB(x.to_bb64());
        match Square::from_bb64(&Bitboard(x)) {
            None => None,
            Some(x) => Some(Bitboard(x)),
        }
    }
}

///////// Boring definitions

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecialBB {
    Empty = 0,
    Full = 0xFFFFFFFFFFFFFFFF,
}

#[repr(u64)]
#[derive(Copy, Clone, PartialEq)]
pub enum File {
    A = 0x0101010101010101,
    B = (File::A as bb64) << 1,
    C = (File::B as bb64) << 1,
    D = (File::C as bb64) << 1,
    E = (File::D as bb64) << 1,
    F = (File::E as bb64) << 1,
    G = (File::F as bb64) << 1,
    H = (File::G as bb64) << 1,
}
#[repr(u64)]
#[derive(Copy, Clone, PartialEq)]
pub enum Rank {
    R1 = 0xFF,
    R2 = (Rank::R1 as bb64) << 8,
    R3 = (Rank::R2 as bb64) << 8,
    R4 = (Rank::R3 as bb64) << 8,
    R5 = (Rank::R4 as bb64) << 8,
    R6 = (Rank::R5 as bb64) << 8,
    R7 = (Rank::R6 as bb64) << 8,
    R8 = (Rank::R7 as bb64) << 8,
}

impl File {
    pub const COUNT: usize = 8;
    /*fn index(&self) -> usize {
        unsafe { *(self as *const Self as *const usize) }
    }*/
    pub const fn bitboard(&self) -> bb64 {
        match self {
            File::A => 0x0101010101010101,
            File::B => 0x0101010101010101 << 1,
            File::C => 0x0101010101010101 << 2,
            File::D => 0x0101010101010101 << 3,
            File::E => 0x0101010101010101 << 4,
            File::F => 0x0101010101010101 << 5,
            File::G => 0x0101010101010101 << 6,
            File::H => 0x0101010101010101 << 7,
        }
    }
}

impl Rank {
    pub const COUNT: usize = 8;
}

impl File {
    const fn from_char(c: char) -> Self {
        match c {
            'a' => File::A,
            'b' => File::B,
            'c' => File::C,
            'd' => File::D,
            'e' => File::E,
            'f' => File::F,
            'g' => File::G,
            'h' => File::H,
            _ => panic!(),
        }
    }
}
impl Rank {
    /*
    const fn from_char(c: char) -> Self {
        match c {
            '1' => Rank::R1,
            '2' => Rank::R2,
            '3' => Rank::R3,
            '4' => Rank::R4,
            '5' => Rank::R5,
            '6' => Rank::R6,
            '7' => Rank::R7,
            '8' => Rank::R8,
            _ => panic!(),
        }
    }*/
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Square {
    a1 = 1,
    b1 = (Square::a1 as u64) << 1,
    c1 = (Square::b1 as u64) << 1,
    d1 = (Square::c1 as u64) << 1,
    e1 = (Square::d1 as u64) << 1,
    f1 = (Square::e1 as u64) << 1,
    g1 = (Square::f1 as u64) << 1,
    h1 = (Square::g1 as u64) << 1,
    a2 = (Square::h1 as u64) << 1,
    b2 = (Square::a2 as u64) << 1,
    c2 = (Square::b2 as u64) << 1,
    d2 = (Square::c2 as u64) << 1,
    e2 = (Square::d2 as u64) << 1,
    f2 = (Square::e2 as u64) << 1,
    g2 = (Square::f2 as u64) << 1,
    h2 = (Square::g2 as u64) << 1,
    a3 = (Square::h2 as u64) << 1,
    b3 = (Square::a3 as u64) << 1,
    c3 = (Square::b3 as u64) << 1,
    d3 = (Square::c3 as u64) << 1,
    e3 = (Square::d3 as u64) << 1,
    f3 = (Square::e3 as u64) << 1,
    g3 = (Square::f3 as u64) << 1,
    h3 = (Square::g3 as u64) << 1,
    a4 = (Square::h3 as u64) << 1,
    b4 = (Square::a4 as u64) << 1,
    c4 = (Square::b4 as u64) << 1,
    d4 = (Square::c4 as u64) << 1,
    e4 = (Square::d4 as u64) << 1,
    f4 = (Square::e4 as u64) << 1,
    g4 = (Square::f4 as u64) << 1,
    h4 = (Square::g4 as u64) << 1,
    a5 = (Square::h4 as u64) << 1,
    b5 = (Square::a5 as u64) << 1,
    c5 = (Square::b5 as u64) << 1,
    d5 = (Square::c5 as u64) << 1,
    e5 = (Square::d5 as u64) << 1,
    f5 = (Square::e5 as u64) << 1,
    g5 = (Square::f5 as u64) << 1,
    h5 = (Square::g5 as u64) << 1,
    a6 = (Square::h5 as u64) << 1,
    b6 = (Square::a6 as u64) << 1,
    c6 = (Square::b6 as u64) << 1,
    d6 = (Square::c6 as u64) << 1,
    e6 = (Square::d6 as u64) << 1,
    f6 = (Square::e6 as u64) << 1,
    g6 = (Square::f6 as u64) << 1,
    h6 = (Square::g6 as u64) << 1,
    a7 = (Square::h6 as u64) << 1,
    b7 = (Square::a7 as u64) << 1,
    c7 = (Square::b7 as u64) << 1,
    d7 = (Square::c7 as u64) << 1,
    e7 = (Square::d7 as u64) << 1,
    f7 = (Square::e7 as u64) << 1,
    g7 = (Square::f7 as u64) << 1,
    h7 = (Square::g7 as u64) << 1,
    a8 = (Square::h7 as u64) << 1,
    b8 = (Square::a8 as u64) << 1,
    c8 = (Square::b8 as u64) << 1,
    d8 = (Square::c8 as u64) << 1,
    e8 = (Square::d8 as u64) << 1,
    f8 = (Square::e8 as u64) << 1,
    g8 = (Square::f8 as u64) << 1,
    h8 = (Square::g8 as u64) << 1,
}
impl Square {
    pub const COUNT: usize = 64;
}
impl Bitboard<Square> {
    pub fn to_index(&self) -> u8 {
        (self.0 as u64).trailing_zeros() as u8
    }
    pub fn from_index(x: u8) -> Bitboard<Square> {
        assert!(x < 64);
        unsafe { std::mem::transmute((1 as u64) << x) }
    }
    pub fn generic_from_index(x: u8) -> Bitboard<GenericBB> {
        Bitboard(GenericBB(1 << x))
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(non_camel_case_types)]
pub enum PackedSquare {
    a1,
    b1,
    c1,
    d1,
    e1,
    f1,
    g1,
    h1,
    a2,
    b2,
    c2,
    d2,
    e2,
    f2,
    g2,
    h2,
    a3,
    b3,
    c3,
    d3,
    e3,
    f3,
    g3,
    h3,
    a4,
    b4,
    c4,
    d4,
    e4,
    f4,
    g4,
    h4,
    a5,
    b5,
    c5,
    d5,
    e5,
    f5,
    g5,
    h5,
    a6,
    b6,
    c6,
    d6,
    e6,
    f6,
    g6,
    h6,
    a7,
    b7,
    c7,
    d7,
    e7,
    f7,
    g7,
    h7,
    a8,
    b8,
    c8,
    d8,
    e8,
    f8,
    g8,
    h8,
}
impl PackedSquare {
    pub const COUNT: usize = 64;
}
impl ToBB64 for PackedSquare {
    fn to_bb64(&self) -> bb64 {
        let x = *self as u8;
        (1 as bb64) << x
    }
}
impl From<u8> for Bitboard<PackedSquare> {
    fn from(x: u8) -> Self {
        assert!(x < 64);
        unsafe { std::mem::transmute(x) }
    }
}
impl From<&Bitboard<Square>> for Bitboard<PackedSquare> {
    fn from(value: &Bitboard<Square>) -> Self {
        Self::from(value.to_index())
    }
}
impl Into<Bitboard<Square>> for &Bitboard<PackedSquare> {
    fn into(self) -> Bitboard<Square> {
        Bitboard::<Square>::from_index(self.0 as u8)
    }
}
impl Into<Bitboard<Square>> for Bitboard<PackedSquare> {
    fn into(self) -> Bitboard<Square> {
        Bitboard::<Square>::from_index(self.0 as u8)
    }
}

#[test]
fn btype_tests() {
    assert_eq!(size_of::<Bitboard<GenericBB>>(), size_of::<u64>());
    assert_eq!(size_of::<Bitboard<Rank>>(), size_of::<u64>());
    assert_eq!(size_of::<Bitboard<File>>(), size_of::<u64>());
    assert_eq!(size_of::<Bitboard<SpecialBB>>(), size_of::<u64>());

    assert_eq!(Bitboard(File::A) & Bitboard(Rank::R3), Bitboard(Square::a3));
}
