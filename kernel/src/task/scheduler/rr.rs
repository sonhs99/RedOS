use super::Schedulable;
use crate::{queue::ListQueue, task::Task};
use core::ptr::NonNull;

const PROCESSTIME_COUNT: u64 = 0x2;

pub struct RoundRobinScheduler {
    running: Option<NonNull<Task>>,
    queue: ListQueue<Task>,
    wait: ListQueue<Task>,
    process_count: u64,
    last_fpu_used: Option<u64>,
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            running: None,
            queue: ListQueue::new(),
            wait: ListQueue::new(),
            process_count: PROCESSTIME_COUNT,
            last_fpu_used: None,
        }
    }
}

impl Schedulable for RoundRobinScheduler {
    fn running_task(&mut self) -> Option<NonNull<Task>> {
        self.running
    }
    fn set_running_task(&mut self, task: &mut Task) {
        self.running = NonNull::new(task)
    }

    fn next_task(&mut self) -> Option<NonNull<Task>> {
        self.queue.pop()
    }

    fn push_task(&mut self, task: &mut Task) {
        self.queue.push(NonNull::new(task).unwrap());
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
        self.wait.push(NonNull::new(task).unwrap());
    }

    fn next_wait(&mut self) -> Option<NonNull<Task>> {
        self.wait.pop()
    }

    fn change_priority(&mut self, id: u64, priority: u8) -> Result<(), ()> {
        Ok(())
    }

    fn remove_task(&mut self, task: &mut Task) -> Result<(), ()> {
        self.queue.remove(NonNull::new(task).ok_or(())?);
        Ok(())
    }

    fn last_fpu_used(&self) -> Option<u64> {
        self.last_fpu_used
    }

    fn set_fpu_used(&mut self, id: u64) {
        self.last_fpu_used = Some(id);
    }
}
