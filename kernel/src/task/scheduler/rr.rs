use super::Schedulable;
use crate::{collections::queue::RefQueue, task::Task};
use core::{ops::Deref, ptr::NonNull};

const PROCESSTIME_COUNT: u64 = 0x2;

pub struct RoundRobinScheduler {
    running: Option<NonNull<Task>>,
    queue: RefQueue<Task>,
    wait: RefQueue<Task>,
    process_count: u64,
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            running: None,
            queue: RefQueue::new(),
            wait: RefQueue::new(),
            process_count: PROCESSTIME_COUNT,
        }
    }
}

impl Schedulable for RoundRobinScheduler {
    fn running_task(&mut self) -> Option<&'static mut Task> {
        unsafe { self.running.map(|mut task| task.as_mut()) }
    }
    fn set_running_task(&mut self, task: &mut Task) {
        self.running = NonNull::new(task)
    }

    fn next_task(&mut self) -> Option<&'static mut Task> {
        self.queue.pop()
    }

    fn push_task(&mut self, task: &mut Task) {
        self.queue.push(task);
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
    }

    fn next_wait(&mut self) -> Option<&'static mut Task> {
        self.wait.pop()
    }

    fn change_priority(&mut self, id: u64, priority: u8) -> Result<(), ()> {
        Ok(())
    }

    fn remove_task(&mut self, task: &mut Task) -> Result<(), ()> {
        let mut res = self
            .queue
            .iter()
            .find(|curser| unsafe { curser.data().unwrap().as_ref().id() == task.id() })
            .ok_or(())?;
        res.remove();
        Ok(())
    }

    fn load(&self, task: &Task) -> usize {
        self.queue.length()
    }
}
