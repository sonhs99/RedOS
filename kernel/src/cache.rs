use core::{ptr::NonNull, slice};

use alloc::vec;
use alloc::{boxed::Box, vec::Vec};
use log::debug;

use crate::queue::{ListQueue, Node};

pub enum CacheState<'a, T> {
    Dirty(u64, &'a Vec<T>),
    Clean,
}

#[derive(Clone)]
pub struct CacheEntry<T> {
    tag: u64,
    dirty: bool,
    data: Vec<T>,
    next: Option<NonNull<Self>>,
    prev: Option<NonNull<Self>>,
}

impl<T: Default + Clone> CacheEntry<T> {
    pub fn new(size: usize) -> Self {
        Self {
            tag: 0,
            dirty: false,
            data: vec![T::default(); size],
            next: None,
            prev: None,
        }
    }

    pub const fn tag(&self) -> u64 {
        self.tag
    }

    pub fn set_tag(&mut self, tag: u64) {
        self.tag = tag;
    }
}

impl<T> Node for CacheEntry<T> {
    fn next(&self) -> Option<NonNull<Self>> {
        self.next
    }

    fn prev(&self) -> Option<NonNull<Self>> {
        self.prev
    }

    fn set_next(&mut self, node: Option<NonNull<Self>>) {
        self.next = node;
    }

    fn set_prev(&mut self, node: Option<NonNull<Self>>) {
        self.prev = node;
    }
}

pub struct Cache<T> {
    pool: Vec<CacheEntry<T>>,
    queue: ListQueue<CacheEntry<T>>,
}

impl<T: Clone + Default + 'static> Cache<T> {
    pub fn new(size: usize, length: usize) -> Self {
        let mut me = Self {
            pool: vec![CacheEntry::new(size); length],
            queue: ListQueue::new(),
        };
        me.init();
        me
    }

    fn init(&mut self) {
        for node in self.pool.iter_mut() {
            self.queue.push(NonNull::new(node).unwrap());
        }
    }

    pub fn read_from_cache(&mut self, tag: u64) -> Result<&Vec<T>, ()> {
        for node in self.queue.iter() {
            if node.tag == tag {
                let node_ptr = NonNull::new(node).unwrap();
                self.queue.remove(node_ptr);
                self.queue.push(node_ptr);
                return Ok(&node.data);
            }
        }
        Err(())
    }

    pub fn write_to_cache(&mut self, tag: u64, value: &Vec<T>) -> Result<(), ()> {
        for node in self.queue.iter() {
            if node.tag == tag {
                node.dirty = true;
                debug!("[CACHE] write tag={}", node.tag);
                node.data.clone_from(value);

                let node_ptr = NonNull::new(node).unwrap();
                self.queue.remove(node_ptr);
                self.queue.push(node_ptr);
                return Ok(());
            }
        }
        Err(())
    }

    pub fn allocate_cache(
        &mut self,
        tag: u64,
        value: &Vec<T>,
        mut burst_fn: impl FnMut(u64, &Vec<T>),
    ) -> Result<(), ()> {
        let mut new_cache_ptr = self.queue.pop().unwrap();
        self.queue.push(new_cache_ptr);

        let new_cache = unsafe { new_cache_ptr.as_mut() };

        if new_cache.dirty {
            burst_fn(new_cache.tag, &new_cache.data);
        }

        debug!("[CACHE] alloc tag={}", tag);

        new_cache.dirty = false;
        new_cache.tag = tag;
        new_cache.data.clone_from(value);
        Ok(())
    }

    pub fn flush(&mut self, mut burst_fn: impl FnMut(u64, &Vec<T>)) {
        for cache in self.pool.iter_mut() {
            if cache.dirty {
                burst_fn(cache.tag, &cache.data);
            }
            cache.dirty = false;
        }
    }
}
