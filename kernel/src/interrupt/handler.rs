use core::arch::asm;

use log::debug;

use crate::{
    device::xhc::XHC,
    interrupt::apic::LocalAPICRegisters,
    println,
    task::{decrease_tick, is_expired, schedule_int, scheduler::Schedulable, Context, SCHEDULER},
};

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
    println!("[EXCEP]: DIVIDED_BY_ZEOR\n{stack_frame:#X?}");
    loop {}
}

pub extern "C" fn invalid_opcode(stack_frame: &ExceptionStackFrame) {
    println!(
        "[EXCEP]: INVALID_OPCODE at {:#X}\n{:#X?}",
        stack_frame.instruction_pointer, stack_frame
    );
    loop {}
}

pub extern "C" fn page_fault(stack_frame: &ExceptionStackFrame, error_code: u64) {
    println!("[EXCEP]: PAGE_FAULT with code {error_code}\n{stack_frame:#X?}");
    loop {}
}

pub extern "C" fn double_fault(stack_frame: &ExceptionStackFrame, error_code: u64) {
    println!("[EXCEP]: DOUBLE_FAULT with code {error_code}\n{stack_frame:#X?}");
    loop {}
}

pub extern "C" fn general_protection(stack_frame: &ExceptionStackFrame, error_code: u64) {
    println!("[EXCEP]: GENERAL_PROTECTION_FAULT with code {error_code}\n{stack_frame:#X?}");
    loop {}
}

pub extern "C" fn break_point(stack_frame: &ExceptionStackFrame) {
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
        // debug!("timer");
        schedule_int(current_context);
    }
    LocalAPICRegisters::default().end_of_interrupt().notify();
}
