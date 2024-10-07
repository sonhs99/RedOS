pub mod idle;
mod manager;
pub mod scheduler;
pub mod tcb;

use core::{ptr::NonNull, usize};

use alloc::vec::Vec;
use log::{debug, error};
use manager::TaskManager;
use scheduler::{prr::PriorityRoundRobinScheduler, rr::RoundRobinScheduler, Schedulable};

use crate::{
    allocator::malloc,
    float::{clear_ts, fpu_load, fpu_save, set_ts},
    interrupt::{apic::LocalAPICRegisters, without_interrupts},
    println,
    sync::{Mark, Mutex, OnceLock},
};

pub use tcb::{Context, FPUContext, Task, TaskFlags};

const STACK_SIZE: usize = 0x2000;

static TASK_MANAGER: OnceLock<Mutex<TaskManager>> = OnceLock::new();
pub static SCHEDULER: [OnceLock<Mark<Mutex<PriorityRoundRobinScheduler>>>; 16] =
    [const { OnceLock::new() }; 16];

pub fn schedule() {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    let _ = without_interrupts(|| {
        let mut scheduler = SCHEDULER[apic_id as usize].skip().lock();
        let mut next_task = scheduler.next_task()?;
        let mut running_task = scheduler.running_task()?;

        scheduler.set_running_task(next_task);
        scheduler.reset_tick();

        running_task.fpu_context().save();
        next_task.fpu_context().load();

        if running_task.flags_mut().is_terminated() {
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
        let mut next_task = scheduler.next_task()?;
        let mut running_task = scheduler.running_task()?;

        scheduler.set_running_task(next_task);
        scheduler.reset_tick();

        running_task.fpu_context().save();
        *running_task.context() = *context;
        next_task.fpu_context().load();
        *context = *next_task.context();

        if running_task.flags_mut().is_terminated() {
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
    let target_id = match task.affinity() {
        Some(id) => *id,
        None => {
            let mut min_id = task.apic_id();
            let mut min = usize::MAX;
            for (id, sched) in SCHEDULER
                .iter()
                .enumerate()
                .filter_map(|(id, sched)| sched.get().map(|sched| (id as u8, sched.skip())))
            {
                if id == task.apic_id() {
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
    task.set_apic_id(target_id);
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
    let mut manager = TASK_MANAGER
        .get_or_init(|| Mutex::new(TaskManager::new()))
        .lock();
    let mut scheduler = SCHEDULER[apic_id as usize]
        .get_or_init(|| Mark::new(Mutex::new(PriorityRoundRobinScheduler::new())));
    let task = manager.allocate().unwrap();
    scheduler.skip().lock().set_running_task(task);
    *task.flags_mut() = *TaskFlags::new().set_priority(0).system();
    *task.affinity() = Some(apic_id as u8);
    task.set_name("idle");
}

pub fn create_task(
    name: &str,
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
        let mut parent_task = scheduler.running_task();

        let stack_addr = malloc(STACK_SIZE, STACK_SIZE) as u64;

        let (memory_addr, memory_size) = if flag.is_thread() {
            unsafe { parent_task.as_ref().unwrap().memory_area() }
        } else {
            (memory_addr, memory_size)
        };
        *task = Task::new(
            task.id(),
            flag,
            entry_point,
            stack_addr,
            STACK_SIZE as u64,
            memory_addr,
            memory_size,
            affinity,
            name,
        );
        // debug!("stack_addr = {stack_addr:#X}");

        if let Some(ref mut parent) = parent_task {
            task.set_sibling(parent.child().as_deref());
            parent.set_child(Some(task));
            // debug!(
            //     "P={},C={},S={:?}",
            //     parent.id(),
            //     task.id(),
            //     task.sibling()
            //         .and_then(|task| unsafe { Some(task.as_ref().id()) })
            // );
        }
        task.set_parent(parent_task.as_deref());

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
                let mut list: Vec<_> = manager.iter().map(|task| task.id()).collect();
                list.sort();
                panic!("Task {id} Not Found\nList: {list:?}");
            }
        };
        let mut cap_scheduler = None;
        while true {
            let apic_id = task.apic_id();
            let scheduler = SCHEDULER[task.apic_id() as usize].skip().lock();
            if task.apic_id() == apic_id {
                cap_scheduler = Some(scheduler);
                break;
            }
        }
        let mut scheduler = cap_scheduler.unwrap();
        // let task = manager.get(id).unwrap();
        task.flags_mut().terminate();
        if scheduler.running_task().unwrap().id() == id {
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
    end_task(running_task.id());
}

fn exit_inner() {
    use core::arch::asm;
    unsafe { asm!("sub rsp, 8") };
    let running_task = running_task().unwrap();
    end_task(running_task.id());
}

pub fn running_task() -> Option<&'static mut Task> {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id();
    without_interrupts(|| unsafe { SCHEDULER[apic_id as usize].skip().lock().running_task() })
}

pub fn get_task_from_id(id: u64) -> Option<&'static mut Task> {
    TASK_MANAGER.lock().get(id)
}
