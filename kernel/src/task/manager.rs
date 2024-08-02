use super::{Context, Task};
use crate::{allocator::malloc, queue::ListQueue};
use core::ptr::NonNull;
use hashbrown::HashMap;
use log::debug;

const TASKPOOL_SIZE: usize = 1024;
pub struct TaskManager {
    empty_queue: ListQueue<Task>,
    task_map: HashMap<u64, NonNull<Task>>,
    max_count: usize,
    use_count: usize,
    alloc_count: usize,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            empty_queue: ListQueue::new(),
            task_map: HashMap::new(),
            use_count: 0,
            alloc_count: 0,
            max_count: TASKPOOL_SIZE,
        }
    }

    pub fn allocate(&mut self) -> Result<&'static mut Task, ()> {
        const TASK_SIZE: usize = size_of::<Task>();
        if self.use_count >= TASKPOOL_SIZE {
            return Err(());
        }
        self.use_count += 1;
        self.alloc_count = self.alloc_count.wrapping_add(1);
        let task = unsafe {
            if let Some(mut task) = self.empty_queue.pop() {
                task.as_mut()
            } else {
                let task_ptr = malloc(size_of::<Task>(), 8).cast::<Task>();
                debug!("task_ptr = {task_ptr:?}, size = {TASK_SIZE:#X}");
                if let Some(mut task) = NonNull::new(task_ptr) {
                    task.as_mut()
                } else {
                    return Err(());
                }
            }
        };
        task.id = self.alloc_count as u64;
        self.task_map.insert(task.id, NonNull::new(task).unwrap());
        Ok(task)
    }

    pub fn free(&mut self, task: &mut Task) {
        task.parent = None;
        task.child = None;
        task.sibling = None;
        task.context = Context::empty();
        self.empty_queue.push(NonNull::new(task).unwrap());
        self.task_map.remove(&task.id);
        self.use_count -= 1;
    }

    pub fn get(&mut self, id: u64) -> Option<&'static mut Task> {
        // let task_ptr = self.task_map.get(&id);
        // debug!("task_ptr = {task_ptr:?}");
        Some(unsafe { self.task_map.get_mut(&id)?.as_mut() })
    }
}
