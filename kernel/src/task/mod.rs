mod manager;
pub mod scheduler;

use core::ptr::NonNull;

use manager::TaskManager;
use scheduler::{rr::RoundRobinScheduler, Schedulable};

use crate::{
    allocator::malloc,
    interrupt::without_interrupts,
    println,
    queue::Node,
    sync::{Mutex, OnceLock},
    KernelStack,
};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Task {
    context: Context,

    id: u64,
    flags: u64,

    next: Option<NonNull<Task>>,
    prev: Option<NonNull<Task>>,

    parent: Option<NonNull<Task>>,
    child: Option<NonNull<Task>>,
    sibling: Option<NonNull<Task>>,

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
            next: None,
            prev: None,
            parent: None,
            child: None,
            sibling: None,
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
            next: None,
            prev: None,
            parent: None,
            child: None,
            sibling: None,
        }
    }

    pub fn child(&mut self) -> &mut Option<NonNull<Task>> {
        &mut self.next
    }

    pub fn sibling(&mut self) -> &mut Option<NonNull<Task>> {
        &mut self.prev
    }

    pub fn parent(&mut self) -> &mut Option<NonNull<Task>> {
        &mut self.parent
    }
}

impl Node for Task {
    fn next(&self) -> Option<NonNull<Task>> {
        self.next
    }

    fn prev(&self) -> Option<NonNull<Task>> {
        self.prev
    }

    fn set_next(&mut self, node: Option<NonNull<Task>>) {
        self.next = node;
    }

    fn set_prev(&mut self, node: Option<NonNull<Task>>) {
        self.prev = node;
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

const STACK_SIZE: usize = 0x2000;

static TASK_MANAGER: OnceLock<Mutex<TaskManager>> = OnceLock::new();
static TASK_STACK: OnceLock<u64> = OnceLock::new();
static SCHEDULER: OnceLock<Mutex<RoundRobinScheduler>> = OnceLock::new();

pub fn schedule() {
    let _ = without_interrupts(|| {
        let mut scheduler = SCHEDULER.get()?.lock();
        let mut next_task = unsafe { scheduler.next_task()?.as_mut() };
        let mut running_task = unsafe { scheduler.running_task()?.as_mut() };
        scheduler.push_task(running_task);
        scheduler.set_running_task(next_task);
        // println!("schedule {} -> {}", running_task.id, next_task.id);
        running_task.context().switch_to(next_task.context());
        scheduler.reset_tick();
        Some(())
    });
}

pub fn schedule_int(context: &mut Context) {
    let _ = without_interrupts(|| {
        let mut scheduler = SCHEDULER.get()?.lock();
        let mut next_task = unsafe { scheduler.next_task()?.as_mut() };
        let mut running_task = unsafe { scheduler.running_task()?.as_mut() };
        // println!("schedule_int {} -> {}", running_task.id, next_task.id);
        scheduler.push_task(running_task);
        scheduler.set_running_task(next_task);
        running_task.context = *context;
        *context = next_task.context;
        scheduler.reset_tick();
        Some(())
    });
}

pub fn is_expired() -> bool {
    SCHEDULER.lock().is_expired()
}

pub fn decrease_tick() {
    SCHEDULER.lock().tick();
}

pub fn init_task() {
    let mut manager = TASK_MANAGER
        .get_or_init(|| Mutex::new(TaskManager::new()))
        .lock();
    let scheduler = SCHEDULER.get_or_init(|| Mutex::new(RoundRobinScheduler::new()));
    TASK_STACK.get_or_init(|| malloc(STACK_SIZE * 1024, 8) as u64);
    let task = manager.allocate().unwrap();
    scheduler.lock().set_running_task(task);
}

pub fn create_task(flag: u64, entry_point: u64) -> Result<(), ()> {
    let mut manager = TASK_MANAGER.lock();
    let task = manager.allocate()?;
    // println!("alloc {}", task.id);

    let stack_addr = TASK_STACK.get().unwrap() + 8192 * task.id;
    // println!("stack_addr: {:X}", stack_addr);
    *task = Task::new(task.id, flag, entry_point, stack_addr, STACK_SIZE as u64);

    SCHEDULER.lock().push_task(task);
    Ok(())
}
