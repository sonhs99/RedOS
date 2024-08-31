pub mod prr;
pub mod rr;

use core::{ptr::NonNull, task::Context};

use prr::PriorityRoundRobinScheduler;
use rr::RoundRobinScheduler;

use super::Task;

pub trait Schedulable {
    fn running_task(&mut self) -> Option<NonNull<Task>>;
    fn set_running_task(&mut self, task: &mut Task);

    fn next_task(&mut self) -> Option<NonNull<Task>>;
    fn push_task(&mut self, task: &mut Task);
    fn load(&self, task: &Task) -> usize;

    // Wait Queue
    fn next_wait(&mut self) -> Option<NonNull<Task>>;
    fn push_wait(&mut self, task: &mut Task);

    // Priority
    fn change_priority(&mut self, id: u64, priority: u8) -> Result<(), ()>;
    fn remove_task(&mut self, task: &mut Task) -> Result<(), ()>;

    // Preemptive Schedule
    fn tick(&mut self);
    fn reset_tick(&mut self);
    fn is_expired(&self) -> bool;
}
