use alloc::vec::Vec;
use core::alloc::Allocator;

pub mod list;
pub mod queue;

trait Length {
    fn length(&self) -> usize;
}

impl<T, A: Allocator> Length for Vec<T, A> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T, const N: usize> Length for [T; N] {
    fn length(&self) -> usize {
        self.len()
    }
}
