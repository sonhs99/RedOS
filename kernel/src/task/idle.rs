use alloc::vec::Vec;
use log::debug;

use crate::allocator::free;
use crate::interrupt::apic::LocalAPICRegisters;
use crate::interrupt::without_interrupts;
use crate::task::STACK_SIZE;

use super::Schedulable;
use super::SCHEDULER;
use super::TASK_MANAGER;

pub fn idle_task() {
    let apic_id = LocalAPICRegisters::default().local_apic_id().id() as usize;
    loop {
        without_interrupts(|| {
            let mut scheduler = SCHEDULER[apic_id].skip().lock();
            while let Some(mut task) = scheduler.next_wait() {
                match task.child() {
                    Some(_) => {
                        let mut child_task = task.child();
                        while let Some(mut child) = child_task {
                            if !child.flags().is_terminated() {
                                child.flags_mut().terminate();
                                scheduler.remove_task(child);
                                scheduler.push_wait(child);
                            }
                            child_task = child.sibling();
                        }
                        scheduler.push_wait(task);
                    }
                    None => {
                        if let Some(mut parent) = task.parent() {
                            if let Some(child) = parent.child() {
                                if child.id() == task.id() {
                                    parent.set_child(task.sibling().as_deref());
                                } else {
                                    let mut sibling = child;
                                    while let Some(brother) = sibling.sibling() {
                                        if brother.id() == task.id() {
                                            break;
                                        }
                                        sibling = brother;
                                    }
                                    if let Some(brother) = sibling.sibling() {
                                        sibling.set_sibling(brother.sibling().as_deref());
                                    } else {
                                        panic!("Task {} Not Found", task.id());
                                    }
                                }
                            }
                        }
                        free(
                            task.stack_area().0 as *mut u8,
                            task.stack_area().1 as usize,
                            STACK_SIZE,
                        );
                        TASK_MANAGER.lock().free(task);
                    }
                }
            }
        });
    }
}
