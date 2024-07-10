use x86_64::{
    registers::segmentation::{Segment, CS, SS},
    structures::gdt::{Descriptor, GlobalDescriptorTable},
};

use core::arch::asm;

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();

unsafe fn init_gdt_unsafe() {
    let code = GDT.append(Descriptor::kernel_code_segment());
    let stack = GDT.append(Descriptor::kernel_data_segment());

    GDT.load();

    CS::set_reg(code);
    SS::set_reg(stack);

    asm!(
        "mov ax, 0x10",
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        options(nostack, preserves_flags)
    );
}

pub fn init_gdt() {
    unsafe { init_gdt_unsafe() };
}
