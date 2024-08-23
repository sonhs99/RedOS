use core::{arch::asm, hint::black_box};

use log::debug;

use crate::{
    device::{block::pata::set_interrupt_flag, xhc::XHC},
    float::{clear_ts, fpu_init, fpu_load, fpu_save},
    interrupt::apic::LocalAPICRegisters,
    println,
    task::{
        decrease_tick, exit, get_task_from_id, is_expired, running_task, schedule_int,
        scheduler::Schedulable, Context, SCHEDULER,
    },
};

use super::INT_FLAG;

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionStackFrame {
    instruction_pointer: u64,
    code_segment: u64,
    cpu_flags: u64,
    stack_pointer: u64,
    stack_segment: u64,
}

#[macro_export]
macro_rules! handler_without_err_code {
    ($name: ident) => {{
        #[naked]
        extern "C" fn wrapper() -> ! {
            unsafe {
                asm!(
                    "
                push rbp
                mov rbp, rsp
                push rax
                push rbx
                push rcx
                push rdx
                push rdi
                push rsi
                push r8
                push r9
                push r10
                push r11
                push r12
                push r13
                push r14
                push r15

                mov rax, ds
                push rax
                mov rax, es
                push rax
                mov rax, fs
                push rax
                mov rax, gs
                push rax

                lea rdi, [rbp + 8]
                call {func}

                pop rax
                mov gs, ax
                pop rax
                mov fs, ax
                pop rax
                mov es, ax
                pop rax
                mov ds, ax

                pop r15
                pop r14
                pop r13
                pop r12
                pop r11
                pop r10
                pop r9
                pop r8
                pop rsi
                pop rdi
                pop rdx
                pop rcx
                pop rbx
                pop rax
                pop rbp
                iretq
                ", func = sym $name, options(noreturn)
                )
            }
        }
        wrapper
    }};
}

#[macro_export]
macro_rules! handler_with_err_code {
    ($name: ident) => {{
        #[naked]
        extern "C" fn wrapper() -> ! {
            unsafe {
                asm!(
                    "
                push rbp
                mov rbp, rsp
                push rax
                push rbx
                push rcx
                push rdx
                push rdi
                push rsi
                push r8
                push r9
                push r10
                push r11
                push r12
                push r13
                push r14
                push r15

                mov rax, ds
                push rax
                mov rax, es
                push rax
                mov rax, fs
                push rax
                mov rax, gs
                push rax

                mov rsi, [rbp + 8]
                lea rdi, [rbp + 16]
                sub rsp, 8
                call {func}
                add rsp, 8

                pop rax
                mov gs, ax
                pop rax
                mov fs, ax
                pop rax
                mov es, ax
                pop rax
                mov ds, ax

                pop r15
                pop r14
                pop r13
                pop r12
                pop r11
                pop r10
                pop r9
                pop r8
                pop rsi
                pop rdi
                pop rdx
                pop rcx
                pop rbx
                pop rax
                pop rbp
                iretq
                ", func = sym $name, options(noreturn)
                )
            }
        }
        wrapper
    }};
}

#[macro_export]
macro_rules! handler_with_context{
    ($name: ident) => {{
        #[naked]
        extern "C" fn wrapper() -> ! {
            unsafe {
                asm!(
                    "
                push rbp
                push rax
                push rbx
                push rcx
                push rdx
                push rdi
                push rsi
                push r8
                push r9
                push r10
                push r11
                push r12
                push r13
                push r14
                push r15

                mov rax, ds
                push rax
                mov rax, es
                push rax
                mov rax, fs
                push rax
                mov rax, gs
                push rax

                mov ax, 0x10
                mov ds, ax
                mov es, ax
                mov fs, ax
                mov gs, ax
                mov ss, ax

                mov rdi, rsp
                call {func}

                pop rax
                mov gs, ax
                pop rax
                mov fs, ax
                pop rax
                mov es, ax
                pop rax
                mov ds, ax

                pop r15
                pop r14
                pop r13
                pop r12
                pop r11
                pop r10
                pop r9
                pop r8
                pop rsi
                pop rdi
                pop rdx
                pop rcx
                pop rbx
                pop rax
                pop rbp
                iretq
                ", func = sym $name, options(noreturn)
                )
            }
        }
        wrapper
    }};
}

pub extern "C" fn common_exception(stack_frame: &ExceptionStackFrame) {
    println!(
        "[EXCEP]: COMMON EXCEPTION at {:#X}\n{:#X?}",
        stack_frame.instruction_pointer, stack_frame
    );
    loop {}
}

pub extern "C" fn divided_by_zero(stack_frame: &ExceptionStackFrame) {
    println!("[EXCEP]: DIVIDED_BY_ZERO\n{stack_frame:#X?}");
    loop {}
}

pub extern "C" fn invalid_opcode(stack_frame: &ExceptionStackFrame) {
    if let Some(running) = running_task() {
        println!("[EXCEP]: PID={}", running.id());
        println!("[EXCEP]: INVALID_OPCODE\n{stack_frame:#X?}");
        if running.id() > 2 {
            exit();
        } else {
            loop {}
        }
    } else {
        println!("[EXCEP]: INVALID_OPCODE\n{stack_frame:#X?}");
        loop {}
    }
}

// pub extern "C" fn device_not_available(stack_frame: &ExceptionStackFrame, error_code: u64) {
//     println!("[EXCEP]: DEVICE_NOT_AVAILABLE");
//     clear_ts();
//     let current = running_task();
//     if let Some(last_id) = last_fpu_used() {
//         if last_id == current.id() {
//             return;
//         } else if let Some(last) = get_task_from_id(last_id) {
//             fpu_save(last.fpu_context());
//         }
//     }
//     if !current.fpu_used() {
//         fpu_init();
//         current.set_fpu_used();
//     } else {
//         fpu_load(current.fpu_context());
//     }
//     set_fpu_used(current.id())
// }

pub extern "C" fn page_fault(stack_frame: &ExceptionStackFrame, error_code: u64) {
    if let Some(running) = running_task() {
        println!("[EXCEP]: PID={}", running.id());
        println!("[EXCEP]: PAGE_FAULT with code {error_code}\n{stack_frame:#X?}");
        if running.id() > 1 {
            exit();
        } else {
            loop {}
        }
    } else {
        println!("[EXCEP]: PAGE_FAULT with code {error_code}\n{stack_frame:#X?}");
        loop {}
    }
}

pub extern "C" fn double_fault(stack_frame: &ExceptionStackFrame, error_code: u64) {
    println!("[EXCEP]: DOUBLE_FAULT with code {error_code}\n{stack_frame:#X?}");
    loop {}
}

pub extern "C" fn general_protection(stack_frame: &ExceptionStackFrame, error_code: u64) {
    if let Some(running) = running_task() {
        println!("[EXCEP]: PID={}", running.id());
        println!("[EXCEP]: GENERAL_PROTECTION_FAULT with code {error_code}\n{stack_frame:#X?}");
        if running.id() > 1 {
            exit();
        } else {
            loop {}
        }
    } else {
        println!("[EXCEP]: GENERAL_PROTECTION_FAULT with code {error_code}\n{stack_frame:#X?}");
        loop {}
    }
}

pub extern "C" fn break_point(stack_frame: &ExceptionStackFrame) {
    if let Some(running) = running_task() {
        println!("[EXCEP]: PID={}", running.id());
    }
    println!(
        "[EXCEP]: BREAKPOINT at {:#X}\n{:#X?}",
        stack_frame.instruction_pointer, stack_frame
    );
}

pub extern "C" fn xhc_handler(stack_frame: &ExceptionStackFrame) {
    if let Some(xhc) = XHC.get() {
        let _ = xhc.lock().process_all_event();
    }
    LocalAPICRegisters::default().end_of_interrupt().notify();
}

pub extern "C" fn apic_timer_handler(current_context: &mut Context) {
    decrease_tick();
    if is_expired() {
        // debug!("[TIMER] {:#X}", current_context.rip);
        schedule_int(current_context);
    }
    LocalAPICRegisters::default().end_of_interrupt().notify();
}

pub extern "C" fn pata1_handler(stack_frame: &ExceptionStackFrame) {
    // println!("[INTER]: PATA1");
    black_box(set_interrupt_flag(true));
    LocalAPICRegisters::default().end_of_interrupt().notify();
}

pub extern "C" fn pata2_handler(stack_frame: &ExceptionStackFrame) {
    // println!("[INTER]: PATA2");
    black_box(set_interrupt_flag(false));
    LocalAPICRegisters::default().end_of_interrupt().notify();
}

pub extern "C" fn irq_dummy_handler(stack_frame: &ExceptionStackFrame) {
    LocalAPICRegisters::default().end_of_interrupt().notify();
}
