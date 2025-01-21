use crate::prelude::*;
use std::{fmt::Debug, marker::PhantomData, mem::MaybeUninit, ops::Index};

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
pub struct Cache<
    X: CopyMoreRelevant + PartialEq,
    SafetyFeature: PartialEq,
    IndexType: Hashable<SafetyFeature> + PartialEq + Copy,
> {
    mask: usize, // instead of %n, we do &mask for speed
    raw: Vec<Option<X>>,
    safety: Vec<MaybeUninit<SafetyFeature>>,
    null: Option<X>, // when a collision is detected

    _index_type: PhantomData<IndexType>,
    #[cfg(debug_assertions)] // item counter
    _items: usize,
    #[cfg(debug_assertions)] // times when the same memory location gets written
    _replaced: usize,
    #[cfg(debug_assertions)] // times when the same entry gets updated
    _updated: usize,
    #[cfg(debug_assertions)] // store full index to remove undetected collisions
    _positions: Vec<MaybeUninit<IndexType>>,
}
impl<X: CopyMoreRelevant + PartialEq, S: PartialEq, I: Hashable<S> + PartialEq + Debug + Copy>
    Cache<X, S, I>
{
    pub fn new(n: usize) -> Self {
        let mut x = Self {
            mask: compute_mask_for_size(n),
            raw: vec![None; n],
            safety: Vec::with_capacity(n),
            null: None,
            _index_type: PhantomData,
            #[cfg(debug_assertions)]
            _items: 0,
            #[cfg(debug_assertions)]
            _replaced: 0,
            #[cfg(debug_assertions)]
            _updated: 0,
            #[cfg(debug_assertions)]
            _positions: Vec::with_capacity(n),
        };
        unsafe {
            x.safety.set_len(n);
            #[cfg(debug_assertions)]
            x._positions.set_len(n);
        };
        x
    }

    // Notice the cache that there is a new value for a given index, it will chose itself if it is relevant
    // TODO: optimize performance, this is not clean
    pub fn push(&mut self, idx: &I, y: &X) {
        let a = &self[idx];
        match a {
            Some(x) => {
                if *y == *X::pick_more_relevant(x, y) {
                    #[cfg(debug_assertions)]
                    {
                        self._updated += 1;
                    };
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
        let heap = self.raw.capacity() * (size_of::<X>() + size_of::<S>());
        println!(
            "Cache ({} elements - {} + {} Bytes (static+dynamic))",
            elements, stack, heap
        );
        println!(
            "\tUsage : {} ({}%)",
            self._items,
            self._items as f64 / elements as f64 * 100.
        );
        println!(
            "\tUpdates : {}%",
            self._updated as f64 / elements as f64 * 100.
        );
        println!(
            "\tCollisions : {}%",
            (self._replaced - self._updated) as f64 / elements as f64 * 100.
        );
    }

    pub fn overwrite_entry(&mut self, idx: &I, x: &X) {
        let i = Self::compute_index(&self, idx);
        self.safety[i] = MaybeUninit::new(I::safety_feature(idx));

        #[cfg(debug_assertions)]
        {
            match self.raw[i] {
                None => self._items += 1,
                Some(_) => self._replaced += 1,
            }
            let idx_dest = unsafe { self._positions[i].assume_init_mut() };
            *idx_dest = *idx;
        }
        self.raw[i] = Some(*x);
    }

    fn compute_index(&self, idx: &I) -> usize {
        self.mask & I::hash(idx)
    }
}
impl<X: CopyMoreRelevant + PartialEq, S: PartialEq, Idx: Hashable<S> + PartialEq + Debug + Copy>
    Index<&Idx> for Cache<X, S, Idx>
{
    type Output = Option<X>;
    fn index(&self, index: &Idx) -> &Self::Output {
        let i = Self::compute_index(&self, index);
        match self.raw[i] {
            Some(_) => {
                match *unsafe { self.safety[i].assume_init_ref() } == Idx::safety_feature(index) {
                    true => {
                        #[cfg(debug_assertions)]
                        {
                            let original_position = unsafe { self._positions[i].assume_init_ref() };
                            if original_position != index {
                                println!("A collision went undetected");
                                println!("original : {:?}", original_position);
                                println!("current : {:?}", index);
                                panic!();
                            }
                        };
                        &self.raw[i]
                    }
                    false => &self.null,
                }
            }
            None => &self.null,
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

fn compute_mask_for_size(n: usize) -> usize {
    assert!(n.is_power_of_two(), "N should be a power of 2");
    n - 1
}

#[test]
fn transposition_tables() {
    assert_eq!(compute_mask_for_size(8), 0b111);

    let mut t = PerftCache::new(16);

    // verify starting pos is not in table
    let r = match t[&PositionSpec::startingpos()] {
        None => true,
        _ => false,
    };
    assert_eq!(
        r, true,
        "Starting pos is already in an empty Cache (shouldnt)"
    );
    t.push(&PositionSpec::startingpos(), &PerftInfo {
        nodes: 20,
        depth: 1,
    });

    assert_eq!(
        t[&PositionSpec::startingpos()].unwrap(),
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
        t[&Position::startingpos()].unwrap(),
        PerftInfo {
            nodes: 400,
            depth: 2
        },
        "Cache::push failed"
    );
}
