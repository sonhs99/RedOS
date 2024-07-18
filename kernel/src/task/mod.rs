#[derive(Clone, Copy)]
#[repr(C)]
pub struct Task {
    context: Context,

    id: u64,
    flags: u64,

    stack_addr: u64,
    stack_size: u64,
}

impl Task {
    pub fn new(id: u64, flags: u64, entry_point: u64, stack_addr: u64, stack_size: u64) -> Self {
        let mut context = Context::empty();

        context.rsp = stack_addr + stack_size;
        context.rbp = stack_addr + stack_size;

        context.cs = 0x08;
        context.ss = 0x10;
        context.ds = 0x10;
        context.es = 0x10;
        context.fs = 0x10;
        context.gs = 0x10;

        context.rip = entry_point;

        context.rflags |= 0x0200;

        Self {
            context,
            id,
            flags,
            stack_addr,
            stack_size,
        }
    }

    pub fn context(&mut self) -> &Context {
        &mut self.context
    }

    pub const fn empty() -> Self {
        Self {
            context: Context::empty(),
            id: 0,
            flags: 0,
            stack_addr: 0,
            stack_size: 0,
        }
    }
}

const TASK_REGISTER_COUNT: usize = 5 + 19;

const CONTEXT_SS: usize = 23;
const CONTEXT_RSP: usize = 22;
const CONTEXT_RFLAGS: usize = 21;
const CONTEXT_CS: usize = 20;
const CONTEXT_RIP: usize = 19;
const CONTEXT_RBP: usize = 18;

const CONTEXT_DS: usize = 3;
const CONTEXT_ES: usize = 2;
const CONTEXT_FS: usize = 1;
const CONTEXT_GS: usize = 0;

#[derive(Clone, Copy, Debug)]
#[repr(C, packed(16))]
pub struct Context {
    pub gs: u64,
    pub fs: u64,
    pub es: u64,
    pub ds: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rbp: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl Context {
    pub const fn empty() -> Self {
        Self {
            gs: 0u64,
            fs: 0u64,
            es: 0u64,
            ds: 0u64,
            r15: 0u64,
            r14: 0u64,
            r13: 0u64,
            r12: 0u64,
            r11: 0u64,
            r10: 0u64,
            r9: 0u64,
            r8: 0u64,
            rsi: 0u64,
            rdi: 0u64,
            rdx: 0u64,
            rcx: 0u64,
            rbx: 0u64,
            rax: 0u64,
            rbp: 0u64,
            rip: 0u64,
            cs: 0u64,
            rflags: 0u64,
            rsp: 0u64,
            ss: 0u64,
        }
    }

    #[inline(always)]
    pub fn switch_to(&self, next: &Context) {
        context_switch(self, next)
    }
}

#[naked]
extern "sysv64" fn context_switch(current: &Context, next: &Context) {
    use core::arch::asm;

    unsafe {
        asm!(
            "
            mov qword ptr [rdi + (18 * 8)], rbp
            mov qword ptr [rdi + (17 * 8)], rax
            mov qword ptr [rdi + (16 * 8)], rbx
            mov qword ptr [rdi + (15 * 8)], rcx
            mov qword ptr [rdi + (14 * 8)], rdx
            mov qword ptr [rdi + (13 * 8)], rdi
            mov qword ptr [rdi + (12 * 8)], rsi
            mov qword ptr [rdi + (11 * 8)], r8
            mov qword ptr [rdi + (10 * 8)], r9
            mov qword ptr [rdi + ( 9 * 8)], r10
            mov qword ptr [rdi + ( 8 * 8)], r11
            mov qword ptr [rdi + ( 7 * 8)], r12
            mov qword ptr [rdi + ( 6 * 8)], r13
            mov qword ptr [rdi + ( 5 * 8)], r14
            mov qword ptr [rdi + ( 4 * 8)], r15

            mov ax, ds
            mov qword ptr [rdi + (3 * 8)], rax

            mov ax, es
            mov qword ptr [rdi + (2 * 8)], rax

            mov ax, fs
            mov qword ptr [rdi + (1 * 8)], rax

            mov ax, gs
            mov qword ptr [rdi + (0 * 8)], rax
            
            mov ax, ss
            mov qword ptr [rdi + (23 * 8)], rax

            lea rax, [rsp + 8]
            mov qword ptr [rdi + (22 * 8)], rax

            pushfq
            pop qword ptr [rdi + (21 * 8)]

            mov ax, cs
            mov qword ptr [rdi + (20 * 8)], rax

            mov rax, qword ptr [rsp]
            mov qword ptr [rdi + (19 * 8)], rax

            push qword ptr [rsi + (23 * 8)]
            push qword ptr [rsi + (22 * 8)]
            push qword ptr [rsi + (21 * 8)]
            push qword ptr [rsi + (20 * 8)]
            push qword ptr [rsi + (19 * 8)]

            mov rax, qword ptr [rsi + (3 * 8)] 
            mov ds, ax 

            mov rax, qword ptr [rsi + (2 * 8)] 
            mov es, ax 

            mov rax, qword ptr [rsi + (1 * 8)] 
            mov fs, ax 

            mov rax, qword ptr [rsi + (0 * 8)] 
            mov gs, ax 

            mov rbp, qword ptr [rsi + (18 * 8)]
            mov rax, qword ptr [rsi + (17 * 8)]
            mov rbx, qword ptr [rsi + (16 * 8)]
            mov rcx, qword ptr [rsi + (15 * 8)]
            mov rdx, qword ptr [rsi + (14 * 8)]
            mov rdi, qword ptr [rsi + (13 * 8)]
            mov  r8, qword ptr [rsi + (11 * 8)]
            mov  r9, qword ptr [rsi + (10 * 8)]
            mov r10, qword ptr [rsi + ( 9 * 8)]
            mov r11, qword ptr [rsi + ( 8 * 8)]
            mov r12, qword ptr [rsi + ( 7 * 8)]
            mov r13, qword ptr [rsi + ( 6 * 8)]
            mov r14, qword ptr [rsi + ( 5 * 8)]
            mov r15, qword ptr [rsi + ( 4 * 8)]
            
            mov rsi, qword ptr [rsi + (12 * 8)]
            iretq
            ",
            options(noreturn)
        )
    };
}
