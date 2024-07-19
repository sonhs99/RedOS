pub mod rr;

use core::{ptr::NonNull, task::Context};

use rr::RoundRobinScheduler;

use crate::{
    interrupt::without_interrupts,
    sync::{Mutex, OnceLock},
};

use super::Task;

pub trait Schedulable {
    fn running_task(&mut self) -> Option<NonNull<Task>>;
    fn set_running_task(&mut self, task: &mut Task);
    fn next_task(&mut self) -> Option<NonNull<Task>>;
    fn push_task(&mut self, task: &mut Task);
    fn tick(&mut self);
    fn reset_tick(&mut self);
    fn is_expired(&self) -> bool;
}
