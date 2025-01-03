use super::Move;
use std::{fmt::Debug, mem::MaybeUninit, ops::Index};

// Pre move generation
// When computing attacks, stores them in a buffer so that they can be exploited later during move generation

// if used for move generation
pub type MoveVec = FastVec<60, Move>;

// this buffer is used to save data
pub struct FastVec<const N: usize, EntryType: Copy> {
    // max th maximum, could go lower ? not sure -> or use heap if more than 8 of them for instance
    stack: [MaybeUninit<EntryType>; N],
    heap: MaybeUninit<Vec<EntryType>>,
    counter: usize,
    already_init_heap: bool,
}
impl<const N: usize, EntryType: Copy> FastVec<N, EntryType> {
    pub fn new() -> Self {
        FastVec {
            stack: [MaybeUninit::uninit(); N],
            counter: 0,
            heap: MaybeUninit::uninit(),
            already_init_heap: false,
        }
    }
    pub fn push(&mut self, entry: EntryType) {
        if self.counter < N {
            self.stack[self.counter] = MaybeUninit::new(entry);
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
            Some(unsafe { self.stack[self.counter].assume_init() })
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
                let _r = write!(f, "{:?}", self.stack[i].assume_init());
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
            Some(unsafe { self.lvec.stack[x].assume_init_ref() })
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
            unsafe { self.stack[i].assume_init_ref() }
        }
    }
    type Output = EntryType;
}
