use log::debug;

use crate::interrupt::without_interrupts;

use super::Schedulable;
use super::SCHEDULER;
use super::TASK_MANAGER;

pub fn idle_task() {
    debug!("IDLE: IDLE Task Started");
    loop {
        without_interrupts(|| {
            let mut scheduler = SCHEDULER.lock();
            while let Some(mut wait_task) = scheduler.next_wait() {
                let task = unsafe { wait_task.as_mut() };
                if task.child != None {
                    let mut child_task = task.child;
                    while let Some(mut child) = child_task {
                        let child = unsafe { child.as_mut() };
                        if !child.flags.is_terminated() {
                            // debug!("Task {} is terminated", child.id);
                            child.flags.terminate();
                            scheduler.remove_task(child);
                            scheduler.push_wait(child);
                        }
                        child_task = child.sibling;
                    }
                    scheduler.push_wait(task);
                } else {
                    if let Some(mut parent) = task.parent {
                        let parent = unsafe { parent.as_mut() };
                        if parent.child == Some(wait_task) {
                            parent.child = task.sibling;
                        } else {
                            let mut sibling1 = parent.child;
                            let mut sibling2 = unsafe { sibling1.unwrap().as_mut().sibling };
                            while let Some(mut next_task) = sibling2 {
                                let next = unsafe { next_task.as_mut() };
                                // debug!("list : {}", next.id);
                                if next.id == task.id {
                                    break;
                                }
                                sibling1 = sibling2;
                                sibling2 = next.sibling;
                            }
                            if sibling2.is_some() {
                                let brother = unsafe { sibling1.unwrap().as_mut() };
                                // debug!("{} -> {} -> {:?}", brother.id, task.id, task.sibling);
                                brother.sibling = task.sibling;
                            } else {
                                debug!("Not Found {}", task.id);
                                loop {}
                            }
                        }
                    }
                    // debug!("Task {} is ended", task.id);
                    TASK_MANAGER.lock().free(task);
                }
            }
        });
    }
}
