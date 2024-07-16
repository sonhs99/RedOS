pub type InterruptHandler = extern "C" fn() -> !;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Entry {
    pointer_low: u16,
    gdt_selector: u16,
    options: EntryOptions,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct EntryOptions(u16);

impl Entry {
    pub fn new(gdt_selector: u16, handler: InterruptHandler) -> Self {
        let pointer = handler as u64;
        Entry {
            pointer_low: pointer as u16,
            gdt_selector,
            options: EntryOptions::new(),
            pointer_middle: (pointer >> 16) as u16,
            pointer_high: (pointer >> 32) as u32,
            reserved: 0,
        }
    }

    pub const fn blank() -> Self {
        Entry {
            pointer_low: 0,
            gdt_selector: 0,
            options: EntryOptions::new(),
            pointer_middle: 0,
            pointer_high: 0,
            reserved: 0,
        }
    }

    pub fn set_option(&mut self, option: EntryOptions) {
        self.options = option;
    }
}

impl EntryOptions {
    pub const fn new() -> Self {
        Self(0x8E00)
    }

    pub fn set_present(mut self, present: bool) -> Self {
        const FLAG: u16 = 0x8000;
        self.0 = (self.0 & !FLAG) | ((present as u16) & FLAG);
        self
    }

    pub fn disable_interrupt(mut self, disable: bool) -> Self {
        const FLAG: u16 = 0x0100;
        self.0 = (self.0 & !FLAG) | ((disable as u16) & FLAG);
        self
    }

    pub fn set_dpl(mut self, dpl: u16) -> Self {
        const FLAG: u16 = 0x6000;
        let flag = dpl << 13;
        self.0 = (self.0 & !FLAG) | (flag & FLAG);
        self
    }

    pub fn set_stack_index(mut self, index: u16) -> Self {
        const FLAG: u16 = 0x0007;
        self.0 = (self.0 & !FLAG) | (index & FLAG);
        self
    }
}

pub struct EntryTable([Entry; 256]);

#[repr(C, packed)]
struct EntryTablePointer {
    limit: u16,
    offset: u64,
}

impl EntryTable {
    pub const fn new() -> Self {
        Self([Entry::blank(); 256])
    }

    pub fn set_handler(&mut self, index: u8, handler: InterruptHandler) -> &mut Entry {
        self.0[index as usize] = Entry::new(0x08, handler);
        &mut self.0[index as usize]
    }

    pub fn load(&self) {
        use core::arch::asm;
        use core::mem::size_of;

        let pointer = EntryTablePointer {
            limit: (size_of::<Self>() - 1) as u16,
            offset: self as *const _ as u64,
        };

        let ptr = &pointer as *const _ as u64;

        unsafe { asm!("lidt [{p}]", p = in(reg) ptr) };
    }
}
