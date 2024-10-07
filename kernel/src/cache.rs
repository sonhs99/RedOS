use core::{ptr::NonNull, slice};

use alloc::vec;
use alloc::{boxed::Box, vec::Vec};
use log::debug;

use crate::collections::queue::RefQueue;

pub enum CacheState<'a, T> {
    Dirty(u64, &'a Vec<T>),
    Clean,
}

#[derive(Clone)]
pub struct CacheEntry<T> {
    tag: u64,
    dirty: bool,
    data: Vec<T>,
}

impl<T: Default + Clone> CacheEntry<T> {
    pub fn new(size: usize) -> Self {
        Self {
            tag: 0,
            dirty: false,
            data: vec![T::default(); size],
        }
    }

    pub const fn tag(&self) -> u64 {
        self.tag
    }

    pub fn set_tag(&mut self, tag: u64) {
        self.tag = tag;
    }
}

pub struct Cache<T> {
    pool: Vec<CacheEntry<T>>,
    queue: RefQueue<CacheEntry<T>>,
}

impl<T: Clone + Default + 'static> Cache<T> {
    pub fn new(size: usize, length: usize) -> Self {
        let mut me = Self {
            pool: vec![CacheEntry::new(size); length],
            queue: RefQueue::new(),
        };
        me.init();
        me
    }

    fn init(&mut self) {
        for node in self.pool.iter_mut() {
            self.queue.push(node);
        }
    }

    pub fn read_from_cache(&mut self, tag: u64) -> Result<&Vec<T>, ()> {
        for mut curser in self.queue.iter() {
            let node = unsafe { curser.data().ok_or(())?.as_mut() };
            if node.tag == tag {
                curser.remove();
                self.queue.push(node);
                return Ok(&node.data);
            }
        }
        Err(())
    }

    pub fn write_to_cache(&mut self, tag: u64, value: &Vec<T>) -> Result<(), ()> {
        for mut curser in self.queue.iter() {
            let node = unsafe { curser.data().ok_or(())?.as_mut() };
            if node.tag == tag {
                node.dirty = true;
                debug!("[CACHE] write tag={}", node.tag);
                node.data.clone_from(value);

                curser.remove();
                self.queue.push(node);
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
        let mut new_cache = self.queue.pop().ok_or(())?;
        self.queue.push(new_cache);

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
