use std::random;
use std::time::{Duration, Instant};

use crate::position::UciNotation;
use crate::position::bitboard::{Bitboard, File, FromBB, GenericBB, Rank, SpecialBB, Square, ToBB};

use super::dyn_attacks;


const TS_ROOK: usize = 2 * 4096; // optimum is 2**12 = 4096
const TS_BISHOP: usize = 2 * 512; // optimum is 2**9 = 512

// long term goal is to have this structure passed in arguments to
#[derive(Debug)]
pub struct Lookup {
    at_rook: Box<AttackTable<TS_ROOK>>,
    at_bishop: Box<AttackTable<TS_BISHOP>>,
    at_knights: Box<[Bitboard<GenericBB>; 64]>,
}

impl Lookup {
    pub fn init() -> Self {
        let mut baseline = unsafe {
            let mut at_rook: Box<AttackTable<TS_ROOK>> = Box::new_uninit().assume_init();
            let mut at_bishop: Box<AttackTable<TS_BISHOP>> = Box::new_uninit().assume_init();
            {
                at_rook.as_mut().empty_in_place();
                at_bishop.as_mut().empty_in_place();
            }
            Self {
                at_rook,
                at_bishop,
                at_knights: Box::new([Bitboard(GenericBB(0)); 64]),
            }
        };
        baseline.at_rook.init(
            mask_rook,
            dyn_attacks::generate_rooks,
            MAGIC_KEYS_ROOK,
            false,
        );

        baseline.at_bishop.init(
            mask_bishop,
            dyn_attacks::generate_bishops,
            MAGIC_KEYS_BISHOP,
            false,
        );

        for sq in SpecialBB::Full.declass() {
            baseline.at_knights[sq.to_index() as usize] =
                dyn_attacks::generate_knights(sq.declass());
        }

        baseline
    }

    pub fn generate_knights(&self, bb: Bitboard<GenericBB>) -> Bitboard<GenericBB> {
        let mut b = Bitboard(GenericBB(0));
        for sq in bb {
            b = b | self.at_knights[sq.to_index() as usize];
        }
        debug_assert_eq!(b, dyn_attacks::generate_knights(bb), "src : {:?}", bb);
        b
    }

    pub fn generate_bishops(
        &self,
        p: Bitboard<GenericBB>,
        blockers: Bitboard<GenericBB>,
    ) -> Bitboard<GenericBB> {
        let mut dests = SpecialBB::Empty.declass(); // TODO: remove p without bizarre behaviour
        for s in p {
            dests = dests | (self.at_bishop[(s, blockers)] & !s)
        }
        debug_assert_eq!(
            dests,
            dyn_attacks::generate_bishops(p, blockers),
            "Bishop in {} with {} as blockers gave different outcomes : {}, {}",
            p.to_uci(),
            (blockers).to_uci(),
            dests.to_uci(),
            dyn_attacks::generate_bishops(p, blockers).to_uci()
        );
        dests
    }

    pub fn generate_rooks(
        &self,
        p: Bitboard<GenericBB>,
        blockers: Bitboard<GenericBB>,
    ) -> Bitboard<GenericBB> {
        let mut dests = SpecialBB::Empty.declass();
        for s in p {
            dests = dests | (self.at_rook[(s, blockers)] ^ s)
        }

        debug_assert_eq!(
            dests,
            dyn_attacks::generate_rooks(p, blockers),
            "Rook in {} with {} as blockers gave different outcomes : {}, {}",
            p.to_uci(),
            (blockers).to_uci(),
            dests.to_uci(),
            dyn_attacks::generate_rooks(p, blockers).to_uci()
        );
        dests
    }

    pub fn generate_queens(
        &self,
        p: Bitboard<GenericBB>,
        blockers: Bitboard<GenericBB>,
    ) -> Bitboard<GenericBB> {
        self.generate_bishops(p, blockers) | self.generate_rooks(p, blockers)
    }
}

#[derive(Clone, Debug, Copy)]
struct AttackTablePart<const N: usize> {
    key: u64,
    blocker_mask: Bitboard<GenericBB>,
    outcomes: [Bitboard<GenericBB>; N],
}

impl<const N: usize> AttackTablePart<N> {
    pub fn empty() -> Self {
        Self {
            key: 0,
            blocker_mask: Bitboard(GenericBB(0)),
            outcomes: [Bitboard(GenericBB(0)); N],
        }
    }
    pub fn empty_in_place(&mut self) {
        self.key = 0;
        self.blocker_mask = Bitboard(GenericBB(0));
        // self.outcomes = [Bitboard(GenericBB(0)); N]; // hopefully this just leads to a memset
        for i in 0..self.outcomes.len() {
            self.outcomes[i] = Bitboard(GenericBB(0));
        }
    }
    pub fn get(&self, sq: Bitboard<Square>, blockers: Bitboard<GenericBB>) -> &Bitboard<GenericBB> {
        &self.outcomes
            [Self::magic_index_dec(self.key, (sq | blockers) & self.blocker_mask) as usize]
    }

    fn from_key(
        key: u64,
        sq: Bitboard<Square>,
        blockers: Bitboard<GenericBB>,
        att_fn: AttackFn,
        verbose: bool,
    ) -> Self {
        let start_time = Instant::now();
        let mut last_print = Instant::now();
        let mut key = key;
        let mut tries = 0;
        loop {
            if !verbose && tries > 1 {
                panic!("Program launched with wrong keys")
            }
            if Instant::now() > last_print + Duration::from_secs(10) {
                last_print = Instant::now();
                let x = last_print - start_time;
                println!(
                    "Still searching for sq {} (blockers = {}) (time spent {}ms)",
                    sq.to_uci(),
                    blockers.to_uci(),
                    x.as_millis()
                )
            }
            let mut mem = [SpecialBB::Empty.declass(); N];
            let mut found = true;
            for c in Combinations::generate(blockers) {
                let index = Self::magic_index_dec(key, c);
                if mem[index as usize] != SpecialBB::Empty.declass() {
                    // collision, will have to retry with another key
                    found = false;
                    tries += 1;
                    break;
                } else {
                    let att = att_fn(sq.declass(), c);
                    /*match sq.to_uci().as_str() {
                        "c1" => {println!("pc {} , bl {} -> att {}", sq.to_uci(), c.to_uci(), att.to_uci());}
                        _ => {}
                    }*/

                    mem[index as usize] = att | sq;
                }
            }
            if found {
                if verbose || tries > 1 {
                    println!(
                        "Found key for sq {} (bl {}) - {}",
                        sq.to_uci(),
                        blockers.to_uci(),
                        key
                    );
                }
                return Self {
                    key,
                    blocker_mask: blockers,
                    outcomes: mem,
                };
            } else {
                key = random::random();
                //println!("Failed, try new key {}",key)
            }
        }
    }
    fn magic_index_dec(key: u64, blockers: Bitboard<GenericBB>) -> u64 {
        let x = blockers.0.0;
        let y = key;
        let z = x.wrapping_mul(y);
        let nb_bits = Square::from_bb(&Bitboard(GenericBB(N as u64)))
            .unwrap()
            .to_index();
        let r = z >> (64 - nb_bits);
        //println!("{},{}={} -> {}", x, y, z, r);
        r
    }
}
#[derive(Debug)]
struct AttackTable<const N: usize> {
    init: Option<()>,
    data: [AttackTablePart<N>; 64],
}

impl<const N: usize> std::ops::Index<(Bitboard<Square>, Bitboard<GenericBB>)> for AttackTable<N> {
    type Output = Bitboard<GenericBB>;
    #[inline(always)]
    fn index(&self, index: (Bitboard<Square>, Bitboard<GenericBB>)) -> &Self::Output {
        self.data[index.0.to_index() as usize].get(index.0, index.1)
    }
}

impl<const N: usize> AttackTable<N> {
    pub fn empty() -> Self {
        Self {
            init: None,
            data: [AttackTablePart::empty(); 64],
        }
    }

    pub fn empty_in_place(&mut self) {
        self.init = None;
        for i in 0..64 {
            self.data[i].empty_in_place();
        }
    }
    pub fn init(
        &mut self,
        mask_fn: MaskFn,
        att_fn: AttackFn,
        default_keys: [u64; 64],
        print_new_keys: bool,
    ) {
        if print_new_keys {
            println!("Searching for new keys...");
        }

        let mut keys = [0; 64];

        for i in 0..64 {
            let sq: Bitboard<Square> = Square::from_bb(&Bitboard(GenericBB(1 << i))).unwrap();
            let blockers: Bitboard<GenericBB> = mask_fn(sq);
            let key = default_keys[i];

            self.data[i] = AttackTablePart::from_key(key, sq, blockers, att_fn, print_new_keys);
            keys[i] = self.data[i].key;
        }

        if print_new_keys {
            println!("Keys selected : {:?}", keys);
        }
        self.init = Some(())
    }
}

pub type MaskFn = fn(s: Bitboard<Square>) -> Bitboard<GenericBB>;
pub type AttackFn =
    fn(s: Bitboard<GenericBB>, blockers: Bitboard<GenericBB>) -> Bitboard<GenericBB>;

pub fn mask_rook(s: Bitboard<Square>) -> Bitboard<GenericBB> {
    let interior =
        !(File::A.declass()) & !(File::H.declass()) & !(Rank::R1.declass()) & !(Rank::R8.declass());
    let is_inner_file =
        !(File::A.declass()) & !(File::H.declass()) & s != SpecialBB::Empty.declass();
    let is_inner_rank =
        !(Rank::R1.declass()) & !(Rank::R8.declass()) & s != SpecialBB::Empty.declass();
    match (is_inner_file, is_inner_rank) {
        (true, true) => {
            let generic_mask = SpecialBB::Full.declass();
            let blockers_mask =
                dyn_attacks::generate_rooks(s.declass(), SpecialBB::Empty.declass());
            generic_mask & blockers_mask & interior & !s
        }
        (true, false) => {
            let mut lr = s.declass();
            let mut ud = s.declass();
            for _i in 0..6 {
                lr = lr | (lr << 1) | (lr >> 1);
                ud = ud | (ud + 1) | (ud - 1);
            }
            (ud | lr) & !s & !(File::A.declass()) & !(File::H.declass())
        }
        (false, true) => {
            let mut lr = s.declass();
            let mut ud = s.declass();
            for _i in 0..6 {
                lr = lr | (lr << 1) | (lr >> 1);
                ud = ud | (ud + 1) | (ud - 1);
            }
            (ud | lr) & !s & !(Rank::R1.declass()) & !(Rank::R8.declass())
        }
        (false, false) => {
            let mut lr = s.declass();
            let mut ud = s.declass();
            for _i in 0..6 {
                lr = lr | (lr << 1) | (lr >> 1);
                ud = ud | (ud + 1) | (ud - 1);
            }
            (ud | lr) & !s
        }
    }
}

pub fn mask_bishop(s: Bitboard<Square>) -> Bitboard<GenericBB> {
    let inner = SpecialBB::Full.declass()
        & !(File::A.declass())
        & !(File::H.declass())
        & !(Rank::R1.declass())
        & !(Rank::R8.declass());
    let blockers_mask = dyn_attacks::generate_bishops(s.declass(), SpecialBB::Empty.declass());
    inner & blockers_mask & !s
}

// iterate over every combinations of a square set
struct Combinations {
    bits: Bitboard<GenericBB>,
    current: Bitboard<GenericBB>,
    over: bool,
}
impl Combinations {
    pub fn generate(x: Bitboard<GenericBB>) -> Self {
        Self {
            bits: x,
            current: SpecialBB::Empty.declass(),
            over: false,
        }
    }
}
impl Iterator for Combinations {
    type Item = Bitboard<GenericBB>;

    fn next(&mut self) -> Option<Self::Item> {
        // increment

        if self.over {
            return None;
        }

        for sq in self.bits {
            if self.current & sq == SpecialBB::Empty.declass() {
                self.current = self.current | sq;
                return Some(self.current);
            } else {
                self.current = self.current & !sq;
            }
        }
        self.over = true;
        return Some(self.current);
    }
}

const MAGIC_KEYS_BISHOP: [u64; 64] = [
    16475677061601647150,
    10682370743719558395,
    10905625644634600799,
    12313156638193159691,
    5718276587559116135,
    5429431037662602121,
    798905036554542843,
    5578744308051737278,
    9861429312443981794,
    6807435366422582103,
    14459219686422537860,
    11356183545371906149,
    17722329681391580350,
    793919143650536378,
    11428570364946074689,
    17079254861504954152,
    5159148727150707695,
    5637775089801464851,
    15254989771162901193,
    15490376742194337286,
    14458568888780975763,
    7572477080922296400,
    15805050227185148446,
    12850007204239353981,
    18367060597685013555,
    17788513854666418290,
    13929997109569560237,
    4017386623043688234,
    16630280355757153752,
    17052043688802563305,
    1880381479152363138,
    11347849059242234692,
    10518277948253691201,
    17399259791587184538,
    2498844697420397427,
    16322663556486324228,
    17915474853787320304,
    15783043388663281889,
    10234101842210622942,
    5864490836150756991,
    13642662073670409531,
    16774191073516691248,
    5448633972460076669,
    7841908345407059235,
    16817879220366911440,
    17814192418132291253,
    8006729998364352926,
    9343028813524512805,
    658861702428718516,
    2121303735751638078,
    9169839632772300902,
    6803480746047618302,
    17083820042508908834,
    16848626875706401243,
    10835751814238358717,
    13525275718487996223,
    3646513723622004269,
    18200184549262043703,
    13738702305814470778,
    6635116094085319339,
    1455636988254698553,
    8720788200682242004,
    11791968535648021303,
    4318224149314623834,
];

const MAGIC_KEYS_ROOK: [u64; 64] = [
    11543541317352298867,
    2669573113713462953,
    7320892990473378894,
    9846021556388029478,
    329913386212561529,
    1499686921204747624,
    15629097162693530677,
    14525356161394300160,
    9526896509027414509,
    3679522599952330356,
    13665010005939938645,
    16289263222064947996,
    11126417358247376154,
    12529201355030605445,
    3162581017120040398,
    1896426617692517432,
    3714221489240122450,
    7134258538384393312,
    2216855892151556023,
    14717136408222398888,
    10415656272237552980,
    13023137388553890678,
    11048208696089859163,
    8037515312046069050,
    14273051789230810998,
    17455243524572102295,
    1250079078027065884,
    9678058834781689328,
    15311069315875398891,
    4925248930769465300,
    17923488852570710787,
    9785927759673295576,
    14512199519547838797,
    350469328994630830,
    1316387552897009004,
    17064470626900700474,
    17816103418294619777,
    2217753167980218178,
    11812384239984991533,
    4764575249913700369,
    15050141290352526972,
    14922469527380846799,
    16023973628342349541,
    9150772633804300629,
    2534036305349733088,
    12816664442669276334,
    4073716996474676563,
    4301061602556484806,
    760566502692924889,
    15263214178549134374,
    1645281190178264861,
    3187081470455797754,
    5489057722724889765,
    4124532861204747793,
    2611726233246699652,
    14748855470451369044,
    1081446462013997266,
    4786578127742007102,
    10354657314539254630,
    3005584460870283338,
    8194253387356166698,
    14977014751191911887,
    17760289909127992828,
    3762411517383666422,
];

#[test]
fn static_attacks_testing() {
    // println!("[StaticAttacks] Size of lookup : {}", std::mem::size_of::<Lookup>() );

    // let l = Lookup::init();
}
