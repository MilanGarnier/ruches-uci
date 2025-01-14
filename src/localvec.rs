//! FastVec is a dynamic-size vector implementation that uses a stack buffer for small sizes
//! and falls back to heap allocation when needed.
//!
//! # Features
//! - Fixed-size stack buffer of size N for fast access to small amounts of data
//! - Automatic fallback to heap allocation when stack buffer is full
//! - Implements Iterator, Debug, and Index traits
//! - Copy-only type requirements for entries
//!
//! # Examples
//! ```
//! let mut vec = FastVec::<8, u32>::new();
//! vec.push(1);
//! vec.push(2);
//! assert_eq!(vec[0], 1);
//! assert_eq!(vec.len(), 2);
//! ```
//!
//! Note: This implementation is marked as deprecated.

use std::{fmt::Debug, mem::MaybeUninit, ops::Index};

// Pre move generation
// When computing attacks, stores them in a buffer so that they can be exploited later during move generation

// this buffer is used to save data
#[deprecated]
pub struct FastVec<const N: usize, EntryType: Copy> {
    // max th maximum, could go lower ? not sure -> or use heap if more than 8 of them for instance
    stack: MaybeUninit<[MaybeUninit<EntryType>; N]>,
    heap: MaybeUninit<Vec<EntryType>>,
    counter: usize,
    already_init_heap: bool,
}
impl<const N: usize, EntryType: Copy> FastVec<N, EntryType> {
    pub fn new() -> Self {
        debug_assert_eq!(0, N & (N - 1), "FastVec size should be a power of 2");
        FastVec {
            stack: MaybeUninit::uninit(),
            heap: MaybeUninit::uninit(),
            counter: 0,
            already_init_heap: false,
        }
    }
    pub fn push(&mut self, entry: EntryType) {
        if self.counter < N {
            unsafe {
                let l = self.stack.assume_init_mut();
                l[self.counter].write(entry);
            }
            self.counter += 1;
        } else {
            match self.already_init_heap {
                false => {
                    self.heap = MaybeUninit::new(Vec::with_capacity(N));
                    self.already_init_heap = true;
                }
                true => (),
            }
            unsafe { self.heap.assume_init_mut().push(entry) };
            self.counter += 1;
        }
    }

    pub fn pop(&mut self) -> Option<EntryType> {
        if self.counter > N {
            self.counter -= 1;
            unsafe { self.heap.assume_init_mut().pop() }
        } else if self.counter >= 1 {
            self.counter -= 1;
            Some(unsafe {
                let l = self.stack.assume_init_ref();
                *l[self.counter].assume_init_ref()
            })
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.counter
    }

    pub fn iter<'a>(&'a self) -> LocalVecIterator<'a, N, EntryType> {
        LocalVecIterator {
            curr: 0,
            lvec: &self,
        }
    }
}

impl<const N: usize, EntryType: Copy + Debug> Debug for FastVec<N, EntryType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _r = write!(f, "PregenCache<{}> | [ ", N);
        for i in 0..self.counter {
            unsafe {
                let _r = write!(f, "{:?}", self.stack.assume_init_ref()[i].assume_init());
                if i < self.counter - 1 {
                    let _r = write!(f, ", ");
                }
            }
        }
        let _r = write!(f, "] + {:?} |", self.heap);
        Ok(())
    }
}

impl<const N: usize, const A: usize, EntryType: Copy + Debug + Sized> From<[EntryType; A]>
    for FastVec<N, EntryType>
{
    fn from(f: [EntryType; A]) -> Self {
        let mut s = Self::new();
        for e in &f {
            s.push(*e);
        }
        s
    }
}

pub struct LocalVecIterator<'a, const N: usize, EntryType: Copy> {
    curr: usize,
    lvec: &'a FastVec<N, EntryType>,
}

impl<'a, const N: usize, EntryType: Copy> Iterator for LocalVecIterator<'a, N, EntryType> {
    type Item = &'a EntryType;

    fn next(&mut self) -> Option<Self::Item> {
        if self.lvec.counter == self.curr {
            None
        } else if self.curr < N {
            let x = self.curr;
            self.curr += 1;
            Some(unsafe { self.lvec.stack.assume_init_ref()[x].assume_init_ref() })
        } else {
            let x = self.curr;
            self.curr += 1;
            Some(unsafe { &self.lvec.heap.assume_init_ref()[x - N] })
        }
    }
}

impl<'a, const N: usize, EntryType: Copy> Drop for FastVec<N, EntryType> {
    fn drop(&mut self) {
        if self.already_init_heap {
            unsafe {
                self.heap.assume_init_drop();
            }
        }
    }
}

impl<'a, const N: usize, EntryType: Copy> Index<usize> for FastVec<N, EntryType> {
    #[inline(always)]
    fn index(&self, i: usize) -> &EntryType {
        if i >= self.counter {
            panic!()
        }
        if i >= N {
            unsafe { &self.heap.assume_init_ref()[i - N] }
        } else {
            unsafe { self.stack.assume_init_ref()[i].assume_init_ref() }
        }
    }
    type Output = EntryType;
}
