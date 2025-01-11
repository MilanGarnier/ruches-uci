use std::{
    fmt::Debug,
    marker::PhantomData,
    mem::MaybeUninit,
    sync::{Arc, RwLock},
};

use super::position::Position;

// TODO: move in specialized perft submodule
pub type PerftCache = Cache<PerftInfo, usize, Position>;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerftInfo {
    pub nodes: u32,
    pub depth: u32,
}
impl<'a> PickMoreRelevant<'a> for PerftInfo {
    fn pick_more_relevant(x: &'a Self, y: &'a Self) -> &'a Self {
        if x.depth > y.depth { x } else { y }
    }
}

/** Transposition tables : store any position-related content.
 * Data is located in the heap. Size has to be a power of 2
 * TODO: Object will be designed for concurrent access.
 */
struct DebugData<IndexType> {
    _items: usize,                           // item counter
    _replaced: usize,                        // times when the same memory location gets written
    _updated: usize,                         // times when the same entry gets updated
    _positions: Vec<MaybeUninit<IndexType>>, // store full index to remove undetected collisions
}
impl<T> DebugData<T> {
    fn new(n: usize) -> Self {
        let mut v = Vec::with_capacity(n);
        unsafe {
            v.set_len(n);
        }
        Self {
            _items: Default::default(),
            _replaced: Default::default(),
            _updated: Default::default(),
            _positions: v,
        }
    }
}

pub struct Cache<
    X: CopyMoreRelevant + PartialEq,
    SafetyFeature: Copy + PartialEq,
    IndexType: Hashable<SafetyFeature> + PartialEq + Copy,
> {
    mask: usize, // instead of %n, we do &mask for speed
    raw: Arc<RwLock<Vec<MaybeUninit<X>>>>,
    safety: Arc<RwLock<Vec<Option<SafetyFeature>>>>,
    _index_type: PhantomData<IndexType>,
    #[cfg(debug_assertions)]
    _debug: Arc<RwLock<DebugData<IndexType>>>,
}
impl<
    X: CopyMoreRelevant + PartialEq,
    S: PartialEq + Copy,
    I: Hashable<S> + PartialEq + Debug + Copy,
> Cache<X, S, I>
{
    pub fn new(n: usize) -> Self {
        let x = Self {
            mask: compute_mask_for_size(n),
            raw: Arc::new(RwLock::new(Vec::with_capacity(n))),
            safety: Arc::new(RwLock::new(vec![None; n])),
            _index_type: PhantomData,
            #[cfg(debug_assertions)]
            _debug: Arc::new(RwLock::new(DebugData::new(n))),
        };
        unsafe {
            x.raw.try_write().unwrap().set_len(n);
        };
        x
    }

    // Notice the cache that there is a new value for a given index, it will chose itself if it is relevant
    // TODO: optimize performance, this is not clean
    pub fn push(&self, idx: &I, y: &X) {
        let a = &self.index(idx);
        match a {
            Some(x) => {
                if *y == *X::pick_more_relevant(x, y) {
                    #[cfg(debug_assertions)]
                    {
                        self._debug.try_write().unwrap()._updated += 1;
                    }
                    self.overwrite_entry(idx, y);
                }
            }
            None => {
                // add new entry
                self.overwrite_entry(idx, y);
            }
        }
    }

    #[cfg(debug_assertions)]
    pub fn print_stats(&self) {
        let elements = self.mask + 1;
        let stack = std::mem::size_of::<Self>();
        let heap = self.raw.try_write().unwrap().capacity() * (size_of::<X>() + size_of::<S>());
        let debug = self._debug.try_write().unwrap();
        log!(
            log::Level::Debug,
            "Cache ({} elements - {} + {} Bytes (static+dynamic))",
            elements,
            stack,
            heap
        );
        log!(
            log::Level::Debug,
            "\tUsage : {} ({}%)",
            debug._items,
            debug._items as f64 / elements as f64 * 100.
        );
        log!(
            log::Level::Debug,
            "\tUpdates : {}%",
            debug._updated as f64 / elements as f64 * 100.
        );
        log!(
            log::Level::Debug,
            "\tCollisions : {}%",
            (debug._replaced - debug._updated) as f64 / elements as f64 * 100.
        );
    }

    pub fn overwrite_entry(&self, idx: &I, x: &X) {
        let i = Self::compute_index(&self, idx);
        self.safety.try_write().unwrap()[i] = Some(I::safety_feature(idx));

        #[cfg(debug_assertions)]
        {
            match self.safety.try_write().unwrap()[i] {
                None => self._debug.try_write().unwrap()._items += 1,
                Some(_) => self._debug.try_write().unwrap()._replaced += 1,
            }
            let mut idx_dest_lock = self._debug.try_write().unwrap();
            let idx_dest = unsafe { idx_dest_lock._positions[i].assume_init_mut() };
            *idx_dest = *idx;
        }
        self.raw.try_write().unwrap()[i].write(*x);
    }

    fn compute_index(&self, idx: &I) -> usize {
        self.mask & I::hash(idx)
    }
}
impl<
    'a,
    X: CopyMoreRelevant + PartialEq + 'a,
    S: PartialEq + Copy,
    Idx: Hashable<S> + PartialEq + Debug + Copy,
> Cache<X, S, Idx>
{
    pub fn index(&'a self, index: &Idx) -> Option<&'a X> {
        let i = self.compute_index(index);
        match self.safety.read().unwrap()[i] {
            Some(_) => match self.safety.read().unwrap()[i] == Some(Idx::safety_feature(index)) {
                true => {
                    #[cfg(debug_assertions)]
                    {
                        let rlock = self._debug.read().unwrap();
                        let original_position = unsafe { rlock._positions[i].assume_init_ref() };
                        if original_position != index {
                            println!("A collision went undetected");
                            println!("original : {:?}", original_position);
                            println!("current : {:?}", index);
                            panic!();
                        }
                    };
                    unsafe { (self.raw.read().unwrap()[i].assume_init_ref() as *const X).as_ref() }
                }
                false => None,
            },
            None => None,
        }
    }
}

/** Pick the more relevant data for storage in a transposition table.
 * In the case of an eval, this should be the eval with the biggest depth.
 * In the case of perft, this could be the number of nodes that is the most expensive to compute
 * If there is an "equality", should return the second one
 */
pub trait PickMoreRelevant<'a> {
    fn pick_more_relevant(x: &'a Self, y: &'a Self) -> &'a Self;
}

/** Hash function, with an additional Safety function to minimize collisions */
pub trait Hashable<Safety: PartialEq> {
    fn hash(x: &Self) -> usize;
    fn safety_feature(x: &Self) -> Safety;
}

/** Automatically generated from PickMoreRelevant */
pub trait CopyMoreRelevant: for<'a> PickMoreRelevant<'a> + Copy {
    fn copy_more_relevant(x: &Self, y: &Self) -> Self {
        *Self::pick_more_relevant(x, y)
    }
}
impl<T: for<'a> PickMoreRelevant<'a> + Copy> CopyMoreRelevant for T {}

const fn compute_mask_for_size(n: usize) -> usize {
    assert!(n.is_power_of_two(), "N should be a power of 2");
    n - 1
}

#[test]
fn transposition_tables() {
    assert_eq!(compute_mask_for_size(8), 0b111);

    let t = PerftCache::new(16);

    // verify starting pos is not in table
    let r = match t.index(&Position::startingpos()) {
        None => true,
        _ => false,
    };
    assert_eq!(
        r, true,
        "Starting pos is already in an empty Cache (shouldnt)"
    );
    t.push(&Position::startingpos(), &PerftInfo {
        nodes: 20,
        depth: 1,
    });

    assert_eq!(
        *t.index(&Position::startingpos()).unwrap(),
        PerftInfo {
            nodes: 20,
            depth: 1
        },
        "Cache::push failed"
    );

    t.push(&Position::startingpos(), &PerftInfo {
        nodes: 400,
        depth: 2,
    });

    t.push(&Position::startingpos(), &PerftInfo {
        nodes: 20,
        depth: 1,
    });

    assert_eq!(
        *t.index(&Position::startingpos()).unwrap(),
        PerftInfo {
            nodes: 400,
            depth: 2
        },
        "Cache::push failed"
    );
}
