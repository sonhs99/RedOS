use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::ops::{Deref, DerefMut};
use core::panic;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct Mutex<T> {
    inner: UnsafeCell<T>,
    status: AtomicBool,
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

#[derive(Debug)]
pub enum MutexError {
    Poisoned,
}

pub struct OnceLock<T> {
    inner: UnsafeCell<Option<T>>,
}

unsafe impl<T> Send for Mutex<T> {}
unsafe impl<T> Sync for Mutex<T> {}

unsafe impl<T: Send> Send for OnceLock<T> {}
unsafe impl<T: Send> Sync for OnceLock<T> {}

impl<T> Mutex<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
            status: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        while !self.status.swap(true, Ordering::AcqRel) {
            spin_loop();
        }
        MutexGuard { mutex: self }
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.inner.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.inner.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.status.store(false, Ordering::Release);
    }
}

impl<T> OnceLock<T> {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    pub fn get(&self) -> Option<&T> {
        unsafe { &*self.inner.get() }.as_ref()
    }

    pub fn try_insert(&self, value: T) -> Result<&T, (&T, T)> {
        if let Some(old) = self.get() {
            Err((old, value))
        } else {
            let new = unsafe { &mut *self.inner.get() };
            Ok(new.insert(value))
        }
    }

    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if let Some(value) = self.get() {
            return value;
        }
        #[cold]
        fn outlined_call<F, T>(f: F) -> T
        where
            F: FnOnce() -> T,
        {
            f()
        }
        let value = outlined_call(f);
        if let Ok(res) = self.try_insert(value) {
            return res;
        } else {
            panic!();
        }
    }
}

impl<T> Deref for OnceLock<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.get().unwrap()
    }
}
