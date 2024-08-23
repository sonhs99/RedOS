use core::arch::asm;
use core::ptr::{write_unaligned, write_volatile};
use log::{debug, info};

use crate::interrupt::{load_idt, set_interrupt};
use crate::sync::{Mutex, OnceLock};
use crate::timer::sleep;
use crate::{
    gdt,
    interrupt::{apic::LocalAPICRegisters, without_interrupts},
    page::page_table_ptr,
    timer::{wait_ms, wait_us},
};

static CORE_WAKEUP_COUNT: OnceLock<Mutex<usize>> = OnceLock::new();

pub fn init_ap(num_ap: usize, stack_start: u64, stack_size: u64) -> Result<usize, &'static str> {
    let mut wakeup_count = 0;
    let mut last_wakeup_count = 0;
    without_interrupts(|| {
        CORE_WAKEUP_COUNT.get_or_init(|| Mutex::new(0));
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

    while wakeup_count < num_ap - 1 {
        // if wakeup_count != last_wakeup_count {
        //     last_wakeup_count = wakeup_count;
        //     debug!("[AP WAKEUP] wake={last_wakeup_count}");
        // }
        // sleep(50);
        wait_ms(50);
        wakeup_count = *CORE_WAKEUP_COUNT.lock();
    }
    Ok(*CORE_WAKEUP_COUNT.lock())
}

fn ap_entry() {
    set_interrupt(false);
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    gdt::load(apic_id);
    load_idt();
    // set_interrupt(true);
    *CORE_WAKEUP_COUNT.lock() += 1;
    // info!("Hello, world!");
    loop {}
}
