pub mod idle;
mod manager;
pub mod scheduler;

use core::{ptr::NonNull, usize};

use log::{debug, error};
use manager::TaskManager;
use scheduler::{prr::PriorityRoundRobinScheduler, rr::RoundRobinScheduler, Schedulable};

use crate::{
    allocator::malloc,
    float::{clear_ts, fpu_load, fpu_save, set_ts},
    interrupt::{apic::LocalAPICRegisters, without_interrupts},
    println,
    queue::Node,
    sync::{Mark, Mutex, OnceLock},
};

#[derive(Clone, Copy)]
pub struct TaskFlags(u64);

impl TaskFlags {
    const TASK_PRIORITY: u64 = 0xFF;
    const TASK_TERMINATE: u64 = 0x8000_0000_0000_0000;
    const TASK_THREAD: u64 = 0x4000_0000_0000_0000;
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
}

#[derive(Clone, Copy)]
#[repr(C, align(16))]
pub struct Task {
    context: Context,
    fpu_context: FPUContext,

    id: u64,
    flags: TaskFlags,

    next: Option<NonNull<Task>>,
    prev: Option<NonNull<Task>>,

    parent: Option<NonNull<Task>>,
    child: Option<NonNull<Task>>,
    sibling: Option<NonNull<Task>>,

    stack_addr: u64,
    stack_size: u64,

    memory_addr: u64,
    memory_size: u64,

    apic_id: u8,
    affinity: Option<u8>,
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
            next: None,
            prev: None,
            parent: None,
            child: None,
            sibling: None,
            memory_addr,
            memory_size,
            apic_id,
            affinity,
        }
    }

    // pub fn corrupted(&self) -> bool {
    //     unsafe { *(self.stack_addr as *const u32) != 0xdeadbeef }
    // }

    pub const fn id(&self) -> u64 {
        self.id
    }

    pub fn context(&mut self) -> &Context {
        &mut self.context
    }

    pub const fn empty() -> Self {
        Self {
            context: Context::empty(),
            fpu_context: FPUContext::new(),
            id: 0,
            flags: TaskFlags(0),
            stack_addr: 0,
            stack_size: 0,
            next: None,
            prev: None,
            parent: None,
            child: None,
            sibling: None,
            memory_addr: 0,
            memory_size: 0,
            apic_id: 0,
            affinity: None,
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

const STACK_SIZE: usize = 0x2000;

static TASK_MANAGER: OnceLock<Mutex<TaskManager>> = OnceLock::new();
pub static SCHEDULER: [OnceLock<Mark<Mutex<PriorityRoundRobinScheduler>>>; 16] =
    [const { OnceLock::new() }; 16];

pub fn schedule() {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    let _ = without_interrupts(|| {
        let mut scheduler = SCHEDULER[apic_id as usize].skip().lock();
        let mut next_task = unsafe { scheduler.next_task()?.as_mut() };
        let mut running_task = unsafe { scheduler.running_task()?.as_mut() };

        scheduler.set_running_task(next_task);
        scheduler.reset_tick();

        running_task.fpu_context.save();
        next_task.fpu_context.load();

        if running_task.flags.is_terminated() {
            scheduler.push_wait(running_task);
            drop(scheduler);
        } else {
            // scheduler.push_task(running_task);
            drop(scheduler);
            push_task_load_balance(running_task);
        }
        // println!("schedule {} -> {}", running_task.id, next_task.id);

        running_task.context().switch_to(next_task.context());
        Some(())
    });
}

pub fn schedule_int(context: &mut Context) {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    let _ = without_interrupts(|| {
        let mut scheduler = SCHEDULER[apic_id as usize].skip().lock();
        let mut next_task = unsafe { scheduler.next_task()?.as_mut() };
        let mut running_task = unsafe { scheduler.running_task()?.as_mut() };

        scheduler.set_running_task(next_task);
        scheduler.reset_tick();

        running_task.fpu_context.save();
        running_task.context = *context;
        next_task.fpu_context.load();
        *context = next_task.context;

        if running_task.flags.is_terminated() {
            scheduler.push_wait(running_task);
            drop(scheduler);
        } else {
            // scheduler.push_task(running_task);
            drop(scheduler);
            push_task_load_balance(running_task);
        }

        Some(())
    });
}

fn push_task_load_balance(task: &mut Task) {
    let target_id = match task.affinity {
        Some(id) => id,
        None => {
            let mut min_id = task.apic_id;
            let mut min = usize::MAX;
            for (id, sched) in SCHEDULER
                .iter()
                .enumerate()
                .filter_map(|(id, sched)| sched.get().map(|sched| (id as u8, sched.skip())))
            {
                if id == task.apic_id {
                    continue;
                }
                let load = sched.without_lock().load(&task);
                if load < min {
                    min = load;
                    min_id = id;
                }
            }
            min_id
        }
    };
    // debug!(
    //     "[SCHED] pid={:3}, {} -> {} {}",
    //     task.id,
    //     task.apic_id,
    //     target_id,
    //     task.flags.priority()
    // );
    task.apic_id = target_id;
    SCHEDULER[target_id as usize].skip().lock().push_task(task);
}

pub fn is_expired() -> bool {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id() as usize;
    SCHEDULER[apic_id].skip().lock().is_expired()
}

pub fn decrease_tick() {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id() as usize;
    SCHEDULER[apic_id].skip().lock().tick();
}

pub fn init_task() {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id() as usize;
    {
        let mut manager = TASK_MANAGER
            .get_or_init(|| Mutex::new(TaskManager::new()))
            .lock();
        let mut scheduler = SCHEDULER[apic_id as usize]
            .get_or_init(|| Mark::new(Mutex::new(PriorityRoundRobinScheduler::new())));
        let task = manager.allocate().unwrap();
        scheduler.skip().lock().set_running_task(task);
        task.flags = *TaskFlags::new().set_priority(0);
        task.affinity = Some(apic_id as u8);
    }
}

pub fn create_task(
    flag: TaskFlags,
    affinity: Option<u8>,
    entry_point: u64,
    memory_addr: u64,
    memory_size: u64,
) -> Result<(), ()> {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    without_interrupts(|| {
        let mut manager = TASK_MANAGER.lock();
        let mut scheduler = SCHEDULER[apic_id as usize].skip().lock();
        let task = manager.allocate()?;
        let parent_task = scheduler.running_task();

        let stack_addr = malloc(STACK_SIZE, STACK_SIZE) as u64;

        let (memory_addr, memory_size) = if flag.is_thread() {
            unsafe {
                (
                    parent_task.unwrap().as_mut().memory_addr,
                    parent_task.unwrap().as_mut().memory_size,
                )
            }
        } else {
            (memory_addr, memory_size)
        };
        *task = Task::new(
            task.id,
            flag,
            entry_point,
            stack_addr,
            STACK_SIZE as u64,
            memory_addr,
            memory_size,
            affinity,
        );
        // debug!("stack_addr = {stack_addr:#X}");

        task.parent = parent_task;
        if let Some(mut parent) = parent_task {
            let parent = unsafe { parent.as_mut() };
            task.sibling = parent.child;
            parent.child = NonNull::new(task);
        }

        scheduler.push_task(task);
        Ok(())
    })
}

pub fn end_task(id: u64) {
    without_interrupts(|| {
        let mut manager = TASK_MANAGER.lock();
        let task = match manager.get(id) {
            Some(task) => task,
            None => {
                error!("Task {id} Not Found");
                for task_ in manager.iter() {
                    debug!("id={}, Task {}, flag={}", id, task_.id, id == task_.id);
                }
                loop {}
            }
        };
        let mut cap_scheduler = None;
        while true {
            let apic_id = task.apic_id;
            let scheduler = SCHEDULER[task.apic_id as usize].skip().lock();
            if task.apic_id == apic_id {
                cap_scheduler = Some(scheduler);
                break;
            }
        }
        let mut scheduler = cap_scheduler.unwrap();
        // let task = manager.get(id).unwrap();
        task.flags.terminate();
        if unsafe { { scheduler.running_task().unwrap() }.as_mut() }.id == id {
            drop(manager);
            drop(scheduler);
            schedule();
            loop {}
        } else {
            scheduler.remove_task(task);
            scheduler.push_wait(task);
        }
    })
}

pub fn exit() {
    let running_task = running_task().unwrap();
    end_task(running_task.id);
}

fn exit_inner() {
    use core::arch::asm;
    unsafe { asm!("sub rsp, 8") };
    let running_task = running_task().unwrap();
    end_task(running_task.id);
}

pub fn running_task() -> Option<&'static mut Task> {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    without_interrupts(|| unsafe {
        Some(
            SCHEDULER[apic_id as usize]
                .skip()
                .lock()
                .running_task()?
                .clone()
                .as_mut(),
        )
    })
}

pub fn get_task_from_id(id: u64) -> Option<&'static mut Task> {
    TASK_MANAGER.lock().get(id)
}
