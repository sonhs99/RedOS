use core::arch::asm;

use crate::println;

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
                push rax
                push rcx
                push rdx
                push rsi
                push rdi
                push r8
                push r9
                push r10
                push r11
                mov rdi, rsp
                add rdi, 9 * 8
                call {func}
                pop r11
                pop r10
                pop r9
                pop r8
                pop rdi
                pop rsi
                pop rdx
                pop rcx
                pop rax
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
                push rax
                push rcx
                push rdx
                push rsi
                push rdi
                push r8
                push r9
                push r10
                push r11
                pop rsi
                mov rdi, rsp
                add rdi, 8*10
                sub rsp, 8
                call {func}
                add rsp, 8
                pop r11
                pop r10
                pop r9
                pop r8
                pop rdi
                pop rsi
                pop rdx
                pop rcx
                pop rax
                iretq
                ", func = sym $name, options(noreturn)
                )
            }
        }
        wrapper
    }};
}

pub fn divided_by_zero(stack_frame: &ExceptionStackFrame) {
    println!("[EXCEP]: DIVIDED_BY_ZEOR\n{stack_frame:#X?}");
    loop {}
}

pub fn invalid_opcode(stack_frame: &ExceptionStackFrame, error_code: u64) {
    println!("[EXCEP]: INVALID_OPCODE with code {error_code}\n{stack_frame:#X?}");
    loop {}
}

pub fn page_fault(stack_frame: &ExceptionStackFrame, error_code: u64) {
    println!("[EXCEP]: INVALID OPCODE with code {error_code}\n{stack_frame:#X?}");
    loop {}
}

pub fn break_point(stack_frame: &ExceptionStackFrame) {
    println!(
        "[EXCEP]: BREAKPOINT at {:#X}\n{:#X?}",
        stack_frame.instruction_pointer, stack_frame
    );
}

pub fn xhc_handler(stack_frame: &ExceptionStackFrame) {
    println!("[INTER]: xHC Interrupt");
}
