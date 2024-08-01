use core::ptr::NonNull;

use log::debug;

use crate::{allocator::malloc, queue::ListQueue};

use super::{Context, Task};

const TASKPOOL_SIZE: usize = 1024;
pub struct TaskManager {
    pool: [Task; TASKPOOL_SIZE],
    empty_queue: ListQueue<Task>,
    max_count: usize,
    use_count: usize,
    alloc_count: usize,
}

impl TaskManager {
    pub fn new() -> Self {
        let mut pool = [Task::empty(); TASKPOOL_SIZE];
        let mut empty_queue: ListQueue<Task> = ListQueue::new();
        pool.iter_mut().enumerate().for_each(|(idx, task)| {
            task.id = idx as u64;
            empty_queue.push(NonNull::new(task).unwrap())
        });

        Self {
            pool,
            empty_queue,
            use_count: 0,
            alloc_count: 0,
            max_count: TASKPOOL_SIZE,
        }
    }

    pub fn allocate(&mut self) -> Result<&mut Task, ()> {
        const TASK_SIZE: usize = size_of::<Task>();
        self.use_count += 1;
        self.alloc_count = self.alloc_count.wrapping_add(1);
        let mut task_ptr = self.empty_queue.pop().ok_or(())?;
        debug!("task_ptr = {task_ptr:?}, size = {TASK_SIZE:#X}");
        Ok(unsafe { task_ptr.as_mut() })
    }

    // pub fn allocate(&mut self) -> Result<&mut Task, ()> {
    //     const TASK_SIZE: usize = size_of::<Task>();
    //     if self.use_count >= TASKPOOL_SIZE {
    //         return Err(());
    //     }
    //     self.use_count += 1;
    //     self.alloc_count = self.alloc_count.wrapping_add(1);
    //     unsafe {
    //         if let Some(mut task) = self.empty_queue.pop() {
    //             Ok(task.as_mut())
    //         } else {
    //             let task_ptr = malloc(size_of::<Task>(), 8).cast::<Task>();
    //             debug!("task_ptr = {task_ptr:?}, size = {TASK_SIZE:#X}");
    //             if let Some(mut task) = NonNull::new(task_ptr) {
    //                 Ok(task.as_mut())
    //             } else {
    //                 Err(())
    //             }
    //         }
    //     }
    // }

    pub fn free(&mut self, task: &mut Task) {
        task.parent = None;
        task.child = None;
        task.sibling = None;
        task.context = Context::empty();
        self.empty_queue.push(NonNull::new(task).unwrap());
        self.use_count -= 1;
    }
}
