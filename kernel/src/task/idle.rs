use log::debug;

use crate::interrupt::without_interrupts;

use super::Schedulable;
use super::SCHEDULER;
use super::TASK_MANAGER;

pub fn idle_task() {
    loop {
        without_interrupts(|| {
            let mut scheduler = SCHEDULER.lock();
            while let Some(mut wait_task) = scheduler.next_wait() {
                let task = unsafe { wait_task.as_mut() };
                if task.child != None {
                    // debug!("child");
                    let mut child_task = task.child;
                    while let Some(mut child) = child_task {
                        let child = unsafe { child.as_mut() };
                        if !child.flags.is_terminated() {
                            debug!("Task {} is terminated", child.id);
                            child.flags.terminate();
                            scheduler.remove_task(child);
                            scheduler.push_wait(child);
                        }
                        child_task = child.sibling;
                    }
                    scheduler.push_wait(task);
                } else {
                    // debug!("parent");
                    if let Some(mut parent) = task.parent {
                        let parent = unsafe { parent.as_mut() };
                        if parent.child == Some(wait_task) {
                            parent.child = task.sibling;
                        } else {
                            let mut sibling = parent.child;
                            while let Some(mut next_task) = sibling {
                                let next = unsafe { next_task.as_mut() };
                                if next.id == task.id {
                                    break;
                                }
                                sibling = next.sibling;
                            }
                            let brother = unsafe { sibling.unwrap().as_mut() };
                            brother.sibling = task.sibling;
                        }
                    }
                    debug!("Task {} is ended", task.id);
                    TASK_MANAGER.lock().free(task);
                }
            }
        });
    }
}
