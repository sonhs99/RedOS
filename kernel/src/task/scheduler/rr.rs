use core::ptr::NonNull;

use crate::{queue::ListQueue, task::Task};

use super::Schedulable;

const PROCESSTIME_COUNT: u64 = 0x2;

pub struct RoundRobinScheduler {
    running: Option<NonNull<Task>>,
    queue: ListQueue<Task>,
    process_count: u64,
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            running: None,
            queue: ListQueue::new(),
            process_count: PROCESSTIME_COUNT,
        }
    }
}

impl Schedulable for RoundRobinScheduler {
    fn running_task(&mut self) -> Option<NonNull<Task>> {
        self.running
    }

    fn next_task(&mut self) -> Option<NonNull<Task>> {
        self.queue.pop()
    }

    fn push_task(&mut self, task: &mut Task) {
        self.queue.push(NonNull::new(task).unwrap());
    }

    fn set_running_task(&mut self, task: &mut Task) {
        self.running = NonNull::new(task)
    }

    fn tick(&mut self) {
        self.process_count -= 1;
    }

    fn reset_tick(&mut self) {
        self.process_count = PROCESSTIME_COUNT;
    }

    fn is_expired(&self) -> bool {
        self.process_count == 0
    }
}
