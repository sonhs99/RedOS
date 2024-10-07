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
    // debug!("[IDLE] IDLE Task Started");
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
                                // debug!("Task {} is terminated", child.id());
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
                            // let mut vec: Vec<u64> = Vec::new();
                            // let mut iter = parent.child();
                            // while let Some(next) = iter {
                            //     vec.push(next.id());
                            //     iter = next.sibling();
                            // }
                            if let Some(child) = parent.child() {
                                if child.id() == task.id() {
                                    // debug!("{} -> {}: {:?}", parent.id(), child.id(), vec);
                                    parent.set_child(task.sibling().as_deref());
                                } else {
                                    let mut sibling = child;
                                    // debug!("id={},Head={}", task.id(), unsafe {
                                    //     sibling1.unwrap().as_mut().id()
                                    // });
                                    while let Some(brother) = sibling.sibling() {
                                        // debug!("list : {}", next.id());
                                        if brother.id() == task.id() {
                                            break;
                                        }
                                        sibling = brother;
                                    }
                                    if let Some(brother) = sibling.sibling() {
                                        // debug!("{} -> {}: {:?}", parent.id(), brother.id(), vec);
                                        sibling.set_sibling(brother.sibling().as_deref());
                                    } else {
                                        panic!("Task {} Not Found", task.id());
                                    }
                                }
                            }
                        }
                        // debug!("Task {} is ended", task.id);
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
