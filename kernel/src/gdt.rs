use log::debug;
use x86_64::{
    registers::segmentation::{Segment, CS, SS},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

use core::arch::asm;

use crate::sync::OnceLock;

static mut GDT: GlobalDescriptorTable<35> = GlobalDescriptorTable::empty();
static mut TSS: [TaskStateSegment; 16] = [TaskStateSegment::new(); 16];

static STACK_FRAME: OnceLock<(u64, u64)> = OnceLock::new();

unsafe fn init_gdt_unsafe(stack_start: u64, stack_size: u64) {
    STACK_FRAME.get_or_init(|| (stack_start, stack_size));
    let code = GDT.append(Descriptor::kernel_code_segment());
    let stack = GDT.append(Descriptor::kernel_data_segment());
    let part_stack_size = stack_size / 16;
    for i in 0..16 {
        TSS[i].interrupt_stack_table[0] =
            VirtAddr::new(stack_start + stack_size - part_stack_size * (i as u64));
        TSS[i].iomap_base = 0xFFFF;
        GDT.append(Descriptor::tss_segment(&TSS[i]));
    }
    GDT.load();

    CS::set_reg(code);

    asm!(
        "
        mov ax, 0x10
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        mov ss, ax

        mov ax, 0x18
        ltr ax",
        options(nostack, preserves_flags)
    );
}

pub fn init_gdt(stack_start: u64, stack_size: u64) {
    unsafe { init_gdt_unsafe(stack_start, stack_size) };
}

pub fn stack_frame() -> (u64, u64) {
    let (stack_start, stack_size) = STACK_FRAME.get().unwrap();
    (*stack_start, *stack_size)
    // let size_of_frame = stack_size / 16;
    // stack_start + stack_size - size_of_frame * id as u64
}

pub fn load(id: u8) {
    unsafe {
        GDT.load();

        CS::set_reg(SegmentSelector(0x08));

        asm!(
            "
        mov ax, 0x10
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        mov ss, ax
        ",
            options(nostack, preserves_flags)
        );
        asm!("ltr ax", in("ax") 0x18 + 16 * id as u16);
    }
}
