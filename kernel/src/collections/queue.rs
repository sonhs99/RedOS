use core::ops::{Index, IndexMut};
use core::ptr::NonNull;

use alloc::vec;
use alloc::vec::Vec;

use super::list::{CurserIter, List, RawList, RawListIter, RawNode};
use super::Length;

pub struct Queue<T: Length> {
    buffer: T,
    put_idx: usize,
    get_idx: usize,
    last_op: bool,
}

impl<T: Length> Queue<T> {
    pub const fn new(buffer: T) -> Self {
        Self {
            buffer,
            put_idx: 0,
            get_idx: 0,
            last_op: false,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.put_idx == self.get_idx && !self.last_op
    }

    pub const fn is_full(&self) -> bool {
        self.put_idx == self.get_idx && self.last_op
    }
}

impl<T, N> Queue<T>
where
    N: Clone + Copy,
    T: Length + Index<usize, Output = N> + IndexMut<usize>,
{
    pub fn enqueue(&mut self, value: N) -> Result<(), ()> {
        if self.is_full() {
            return Err(());
        }
        self.buffer[self.put_idx] = value;
        self.put_idx = (self.put_idx + 1) % self.buffer.length();
        self.last_op = true;

        Ok(())
    }

    pub fn dequeue(&mut self) -> Result<N, ()> {
        if self.is_empty() {
            return Err(());
        }
        let value = self.buffer[self.get_idx];
        self.get_idx = (self.get_idx + 1) % self.buffer.length();
        self.last_op = false;

        Ok(value)
    }
}

pub struct RefQueue<T> {
    list: List<NonNull<T>>,
}

impl<T> RefQueue<T> {
    pub const fn new() -> Self {
        Self { list: List::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn length(&self) -> usize {
        self.list.length()
    }

    pub fn push(&mut self, data: &mut T) {
        self.list.push(NonNull::new(data).unwrap());
    }

    pub fn pop(&mut self) -> Option<&'static mut T> {
        unsafe { self.list.pop().map(|mut data| data.as_mut()) }
    }

    pub fn iter(&mut self) -> CurserIter<NonNull<T>> {
        self.list.iter()
    }
}

pub struct RawQueue<T: RawNode>(RawList<T>);

impl<T: RawNode> RawQueue<T> {
    pub const fn new() -> Self {
        Self(RawList::empty())
    }

    pub fn push(&mut self, data: &mut T) {
        self.0.push(data);
    }

    pub fn pop(&mut self) -> Option<&'static mut T> {
        self.0.pop()
    }

    pub const fn length(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> RawListIter<T> {
        self.iter()
    }
}
