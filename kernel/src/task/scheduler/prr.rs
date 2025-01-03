use alloc::vec::Vec;
use log::debug;
use x86_64::instructions::interrupts::without_interrupts;

use super::Schedulable;
use crate::{
    collections::{list::RawNode, queue::RawQueue},
    task::{Task, TASK_MANAGER},
};
use core::ptr::NonNull;

const PROCESSTIME_COUNT: u64 = 0x4;
const NUM_OF_PRIORITY: usize = 4;
const PRIORITY_SIZE: usize = u8::MAX as usize / NUM_OF_PRIORITY + 1;

pub struct PriorityRoundRobinScheduler {
    running: Option<NonNull<Task>>,
    queues: [RawQueue<Task>; NUM_OF_PRIORITY],
    wait: RawQueue<Task>,
    execute: [usize; NUM_OF_PRIORITY],
    process_count: u64,
    current_execute: usize,
}

impl PriorityRoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            running: None,
            queues: [const { RawQueue::new() }; NUM_OF_PRIORITY],
            wait: RawQueue::new(),
            execute: [0; NUM_OF_PRIORITY],
            process_count: PROCESSTIME_COUNT,
            current_execute: 0,
        }
    }

    fn get_priority(priority: u8) -> usize {
        priority as usize / PRIORITY_SIZE
    }
}

impl Schedulable for PriorityRoundRobinScheduler {
    fn running_task(&mut self) -> Option<&'static mut Task> {
        unsafe { self.running.map(|mut task| task.as_mut()) }
    }
    fn set_running_task(&mut self, task: &mut Task) {
        self.running = NonNull::new(task)
    }

    fn next_task(&mut self) -> Option<&'static mut Task> {
        for _ in 0..2 {
            for priority in 0..NUM_OF_PRIORITY {
                if self.execute[priority] < self.queues[priority].length() {
                    self.execute[priority] += 1;
                    return self.queues[priority].pop();
                } else {
                    self.execute[priority] = 0;
                }
            }
        }
        None
    }

    fn push_task(&mut self, task: &mut Task) {
        let priority = task.flags().priority();
        let queue_idx = Self::get_priority(priority);
        self.queues[queue_idx].push(task);
    }

    fn tick(&mut self) {
        if self.process_count != 0 {
            self.process_count -= 1;
        }
    }

    fn reset_tick(&mut self) {
        self.process_count = PROCESSTIME_COUNT;
    }

    fn is_expired(&self) -> bool {
        self.process_count == 0
    }

    fn push_wait(&mut self, task: &mut Task) {
        self.wait.push(task);
        // debug!("wait = {} push", self.wait.length());
    }

    fn next_wait(&mut self) -> Option<&'static mut Task> {
        let task = self.wait.pop();
        // debug!("wait = {} pop", self.wait.length());
        task
    }

    fn change_priority(&mut self, id: u64, priority: u8) -> Result<(), ()> {
        without_interrupts(|| {
            let mut manager = TASK_MANAGER.lock();
            let task = manager.get(id).ok_or(())?;
            if unsafe { self.running.unwrap().as_mut().id() } != id {
                self.remove_task(task);
            }
            task.flags_mut().set_priority(priority);
            self.push_task(task);
            Ok(())
        })
    }

    fn remove_task(&mut self, task: &mut Task) -> Result<(), ()> {
        let priority = task.flags().priority();
        let queue_idx = Self::get_priority(priority);
        let mut node = self.queues[queue_idx]
            .iter()
            .find(|t| unsafe { t.id() == task.id() })
            .ok_or(())?;
        node.remove();
        Ok(())
    }

    fn load(&self, task: &Task) -> usize {
        let priority = Self::get_priority(task.flags().priority());
        self.queues[priority].length()
    }
}
