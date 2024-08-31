use core::arch::asm;
use core::fmt::Write;
use core::ptr::{write_unaligned, write_volatile};
use log::{debug, info};

use crate::console::CONSOLE;
use crate::interrupt::apic::APICTimerMode;
use crate::interrupt::{load_idt, set_interrupt, InterruptVector};
use crate::sync::{Mutex, OnceLock};
use crate::task::{init_task, init_task_ap};
use crate::timer::sleep;
use crate::{
    gdt,
    interrupt::{apic::LocalAPICRegisters, without_interrupts},
    page::page_table_ptr,
    timer::{wait_ms, wait_us},
};
use crate::{print, println};

static mut CORE_WAKEUP_COUNT: u32 = 0;

pub fn init_ap(num_ap: usize, stack_start: u64, stack_size: u64) -> Result<usize, &'static str> {
    let mut last_count = 0;
    without_interrupts(|| {
        unsafe {
            write_volatile(0x8004 as *mut u32, page_table_ptr());
            write_volatile(0x8008 as *mut u64, ap_entry as u64);
            write_volatile(0x8010 as *mut u32, (stack_start + stack_size) as u32);
            write_volatile(0x8014 as *mut u32, stack_size as u32 / 16);
        }
        debug!("AP Entry Point={:#X}", ap_entry as u64);

        let svr_val = LocalAPICRegisters::default().svr().read();
        LocalAPICRegisters::default().svr().write(svr_val | 0x0100);
        let int_control = LocalAPICRegisters::default().int_control();
        int_control.write(0x000C_4500, 0x0000_0000);
        debug!("ISR={:#X}", int_control.read());
        wait_ms(10);
        if int_control.read() & 0x001000 != 0 {
            return Err("Init");
        }

        int_control.write(0x000C_4608, 0x0000_0000);
        wait_us(200);
        if int_control.read() & 0x001000 != 0 {
            return Err("Startup");
        }
        debug!("ISR={:#X}", int_control.read());

        int_control.write(0x000C_4608, 0x0000_0000);
        wait_us(200);
        if int_control.read() & 0x001000 != 0 {
            return Err("Startup");
        }
        debug!("ISR={:#X}", int_control.read());
        Ok(())
    });

    while unsafe { CORE_WAKEUP_COUNT } < num_ap as u32 - 1 {
        // sleep(50);
        wait_ms(50);
    }
    Ok(unsafe { CORE_WAKEUP_COUNT as usize })
}

fn ap_entry() {
    set_interrupt(false);
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();

    gdt::load(apic_id);
    load_idt();
    init_task();

    let svr_value = LocalAPICRegisters::default().svr().read();
    LocalAPICRegisters::default().svr().write(svr_value | 0x100);

    LocalAPICRegisters::default().apic_timer().init(
        0b1011,
        false,
        APICTimerMode::Periodic,
        InterruptVector::APICTimer as u8,
    );
    set_interrupt(true);
    unsafe { asm!("lock inc dword ptr [{0}]", in(reg) (&CORE_WAKEUP_COUNT) as *const u32) };
    test_ap()
}

#[inline(never)]
fn test_ap() {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    loop {
        wait_ms(500);
        // info!("[{apic_id}] Hello, world!");
    }
}
