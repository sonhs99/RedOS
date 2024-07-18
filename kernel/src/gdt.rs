use log::debug;
use x86_64::{
    registers::segmentation::{Segment, CS, SS},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

use core::arch::asm;

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut TSS: TaskStateSegment = TaskStateSegment::new();

static INT_STACK: [u64; 0x10000] = [0; 0x10000];

unsafe fn init_gdt_unsafe() {
    TSS.interrupt_stack_table[0] = VirtAddr::new(INT_STACK.as_ptr() as u64 + 0x100000);

    let code = GDT.append(Descriptor::kernel_code_segment());
    let stack = GDT.append(Descriptor::kernel_data_segment());
    GDT.append(Descriptor::tss_segment(&TSS));
    GDT.load();

    CS::set_reg(code);
    SS::set_reg(stack);

    debug!("Stack Address={:#X}", INT_STACK.as_ptr() as u64);

    asm!(
        "
        mov ax, 0x10
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax

        mov ax, 0x18
        ltr ax",
        options(nostack, preserves_flags)
    );
}

pub fn init_gdt() {
    unsafe { init_gdt_unsafe() };
}
