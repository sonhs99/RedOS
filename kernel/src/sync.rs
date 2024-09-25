use core::arch::asm;
use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::ops::{Deref, DerefMut};
use core::panic;
use core::sync::atomic::{AtomicBool, Ordering};

use log::debug;

use crate::font::write_ascii;
use crate::graphic::{get_graphic, PixelColor};
use crate::interrupt::apic::LocalAPICRegisters;
use crate::interrupt::{asm, set_interrupt};

pub struct StaticCell<T> {
    inner: UnsafeCell<T>,
}

impl<T> StaticCell<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
        }
    }

    pub fn get(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}

const COLOR: [PixelColor; 4] = [
    PixelColor::White,
    PixelColor::Red,
    PixelColor::Blue,
    PixelColor::Green,
];

pub struct Mark<T> {
    inner: UnsafeCell<T>,
    id: UnsafeCell<u8>,
    pid: UnsafeCell<usize>,
}

pub struct MarkGuard<'a, T> {
    mark: &'a Mark<T>,
}

impl<T> Mark<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
            id: UnsafeCell::new(0xFF),
            pid: UnsafeCell::new(0),
        }
    }

    pub fn mark(&self, index: u8) -> MarkGuard<T> {
        let apic_id = LocalAPICRegisters::default().local_apic_id().id();
        let id = unsafe { *self.id.get() };
        let byte = if id == 0xFF { b' ' } else { b'0' + id };
        write_ascii(
            800 + (index as u64) * 8,
            0,
            byte,
            PixelColor::Black,
            Some(COLOR[(apic_id % 4) as usize]),
            &mut get_graphic().lock(),
        );
        unsafe { *self.id.get() = apic_id };
        MarkGuard { mark: self }
    }

    pub fn skip(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }
}

impl<'a, T> Deref for MarkGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mark.inner.get() }
    }
}

impl<'a, T> Drop for MarkGuard<'a, T> {
    fn drop(&mut self) {
        // let apic_id = LocalAPICRegisters::default().local_apic_id().id();
        // write_ascii(800, 0, b'0' + apic_id, PixelColor::White, PixelColor::Black);
        unsafe { *self.mark.id.get() = 0xFF };
    }
}

unsafe impl<T: Send> Send for StaticCell<T> {}
unsafe impl<T: Send> Sync for StaticCell<T> {}

pub struct Mutex<T> {
    inner: UnsafeCell<T>,
    status: UnsafeCell<u8>,
    id: UnsafeCell<u8>,
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
    interrupt: bool,
}

#[derive(Debug)]
pub enum MutexError {
    Poisoned,
}

pub struct OnceLock<T> {
    inner: UnsafeCell<Option<T>>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

unsafe impl<T> Send for OnceLock<T> {}
unsafe impl<T> Sync for OnceLock<T> {}

unsafe impl<T: Send + Sync> Send for Mark<T> {}
unsafe impl<T: Send + Sync> Sync for Mark<T> {}

impl<T> Mutex<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
            status: UnsafeCell::new(0),
            id: UnsafeCell::new(0xFF),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        let previous = set_interrupt(false);
        let apic_id = LocalAPICRegisters::default().local_apic_id().id();
        let ptr = self.status.get();
        let id_ptr = self.id.get();
        if !test_and_set(ptr, 0, 1) {
            if unsafe { *id_ptr != apic_id } {
                while !test_and_set(ptr, 0, 1) {
                    while unsafe { *ptr != 0 } {
                        spin_loop();
                    }
                }
            }
        }

        unsafe { *id_ptr = apic_id };
        MutexGuard {
            mutex: self,
            interrupt: previous,
        }
    }

    pub fn without_lock(&self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }

    pub fn flag(&self) -> u8 {
        unsafe { *self.status.get() }
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
        set_interrupt(false);
        unsafe { *self.mutex.status.get() = 0 };
        unsafe { *self.mutex.id.get() = 0xFF };
        set_interrupt(self.interrupt);
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

#[naked]
fn test_and_set(ptr: *mut u8, compare: u8, source: u8) -> bool {
    unsafe {
        asm!(
            "
        mov rax, rsi
        lock cmpxchg byte ptr [rdi], dl
        je 5f
        mov rax, 0
        ret
    5:
        mov rax, 1
        ret
        ",
            options(noreturn)
        )
    };
}
