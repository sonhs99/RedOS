use core::ptr::NonNull;

use alloc::string::{String, ToString};

use crate::{
    float::{fpu_load, fpu_save},
    interrupt::apic::LocalAPICRegisters,
};

use super::exit_inner;

#[derive(Clone, Copy)]
pub struct TaskFlags(u64);

impl TaskFlags {
    const TASK_PRIORITY: u64 = 0xFF;
    const TASK_TERMINATE: u64 = 0x8000_0000_0000_0000;
    const TASK_THREAD: u64 = 0x4000_0000_0000_0000;
    const TASK_SYSTEM: u64 = 0x2000_0000_0000_0000;
    pub fn new() -> Self {
        Self(0)
    }
    pub fn set_priority(&mut self, priority: u8) -> &mut Self {
        self.0 = self.0 & Self::TASK_PRIORITY | priority as u64;
        self
    }

    pub const fn priority(&self) -> u8 {
        (self.0 & Self::TASK_PRIORITY) as u8
    }

    pub fn terminate(&mut self) -> &mut Self {
        self.0 |= Self::TASK_TERMINATE;
        self
    }

    pub const fn is_terminated(&self) -> bool {
        self.0 & Self::TASK_TERMINATE != 0
    }

    pub fn thread(&mut self) -> &mut Self {
        self.0 |= Self::TASK_THREAD;
        self
    }

    pub const fn is_thread(&self) -> bool {
        self.0 & Self::TASK_THREAD != 0
    }

    pub fn system(&mut self) -> &mut Self {
        self.0 |= Self::TASK_SYSTEM;
        self
    }

    pub const fn is_system_task(&self) -> bool {
        self.0 & Self::TASK_SYSTEM != 0
    }
}

#[derive(Clone)]
#[repr(C, align(16))]
pub struct Task {
    context: Context,
    fpu_context: FPUContext,

    id: u64,
    flags: TaskFlags,

    parent: Option<NonNull<Task>>,
    child: Option<NonNull<Task>>,
    sibling: Option<NonNull<Task>>,

    stack_addr: u64,
    stack_size: u64,

    memory_addr: u64,
    memory_size: u64,

    apic_id: u8,
    affinity: Option<u8>,
    name: String,
}

impl Task {
    pub fn new(
        id: u64,
        flags: TaskFlags,
        entry_point: u64,
        stack_addr: u64,
        stack_size: u64,
        memory_addr: u64,
        memory_size: u64,
        affinity: Option<u8>,
        name: &str,
    ) -> Self {
        let apic_id = LocalAPICRegisters::default().local_apic_id().id();
        let mut context = Context::empty();

        context.rsp = stack_addr + stack_size - 8;
        context.rbp = stack_addr + stack_size - 8;

        context.cs = 0x08;
        context.ss = 0x10;
        context.ds = 0x10;
        context.es = 0x10;
        context.fs = 0x10;
        context.gs = 0x10;

        context.rip = entry_point;
        context.rflags |= 0x0200;
        unsafe { *(context.rsp as *mut u64) = exit_inner as u64 };

        // unsafe { *(stack_addr as *mut u32) = 0xdeadbeef };

        let mut fpu_context = FPUContext::new();
        unsafe { *(fpu_context.get(24).cast::<u16>()) = 0x1f80 };

        Self {
            context,
            fpu_context,
            id,
            flags,
            stack_addr,
            stack_size,
            parent: None,
            child: None,
            sibling: None,
            memory_addr,
            memory_size,
            apic_id,
            affinity,
            name: name.to_string(),
        }
    }

    // pub fn corrupted(&self) -> bool {
    //     unsafe { *(self.stack_addr as *const u32) != 0xdeadbeef }
    // }

    pub const fn id(&self) -> u64 {
        self.id
    }

    pub fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    pub fn context(&mut self) -> &mut Context {
        &mut self.context
    }

    pub fn fpu_context(&mut self) -> &mut FPUContext {
        &mut self.fpu_context
    }

    pub fn apic_id(&self) -> u8 {
        self.apic_id
    }

    pub fn set_apic_id(&mut self, apic_id: u8) {
        self.apic_id = apic_id;
    }

    pub fn flags_mut(&mut self) -> &mut TaskFlags {
        &mut self.flags
    }

    pub fn flags(&self) -> &TaskFlags {
        &self.flags
    }

    pub fn affinity(&mut self) -> &mut Option<u8> {
        &mut self.affinity
    }

    pub fn memory_area(&self) -> (u64, u64) {
        (self.memory_addr, self.memory_size)
    }

    pub fn set_memory(&mut self, memory_addr: u64, memory_size: u64) {
        self.memory_addr = memory_addr;
        self.memory_size = memory_size;
    }

    pub fn stack_area(&self) -> (u64, u64) {
        (self.stack_addr, self.stack_size)
    }

    pub fn set_stack(&mut self, stack_addr: u64, stack_size: u64) {
        self.stack_addr = stack_addr;
        self.stack_size = stack_size;
    }

    pub fn child(&self) -> Option<&'static mut Task> {
        self.child.map(|mut task| unsafe { task.as_mut() })
    }

    pub fn sibling(&self) -> Option<&'static mut Task> {
        self.sibling.map(|mut task| unsafe { task.as_mut() })
    }

    pub fn parent(&self) -> Option<&'static mut Task> {
        self.parent.map(|mut task| unsafe { task.as_mut() })
    }

    pub fn set_child(&mut self, task: Option<&Task>) {
        self.child =
            task.map(|task| NonNull::new(task as *const Task as u64 as *mut Task).unwrap());
    }

    pub fn set_sibling(&mut self, task: Option<&Task>) {
        self.sibling =
            task.map(|task| NonNull::new(task as *const Task as u64 as *mut Task).unwrap());
    }

    pub fn set_parent(&mut self, task: Option<&Task>) {
        self.parent =
            task.map(|task| NonNull::new(task as *const Task as u64 as *mut Task).unwrap());
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string()
    }
}

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

#[derive(Clone, Copy)]
#[repr(C, align(16))]
pub struct FPUContext([u8; 512]);

impl FPUContext {
    pub const fn new() -> Self {
        Self([0; 512])
    }

    pub fn get(&mut self, idx: usize) -> *mut u8 {
        (&mut self.0[idx]) as *mut u8
    }

    pub fn save(&self) {
        fpu_save(self as *const _ as u64);
    }

    pub fn load(&self) {
        fpu_load(self as *const _ as u64);
    }
}
