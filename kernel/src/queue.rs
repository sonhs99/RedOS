use core::ptr::NonNull;

pub struct ArrayQueue<T, const N: usize> {
    buffer: [T; N],
    put_idx: usize,
    get_idx: usize,
    last_op: bool,
}

impl<T: Clone + Copy, const N: usize> ArrayQueue<T, N> {
    pub const fn new(init_value: T) -> Self {
        Self {
            buffer: [init_value; N],
            put_idx: 0,
            get_idx: 0,
            last_op: false,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.put_idx == self.get_idx && !self.last_op
    }

    pub const fn is_full(&self) -> bool {
        self.put_idx == self.get_idx && self.last_op
    }

    pub fn enqueue(&mut self, value: T) -> Result<(), ()> {
        if self.is_full() {
            return Err(());
        }
        self.buffer[self.put_idx] = value;
        self.put_idx = (self.put_idx + 1) % N;

        Ok(())
    }

    pub fn dequeue(&mut self) -> Result<T, ()> {
        if self.is_empty() {
            return Err(());
        }
        let value = self.buffer[self.get_idx];
        self.get_idx = (self.get_idx + 1) % N;
        Ok(value)
    }
}

pub trait Node {
    fn next(&self) -> Option<NonNull<Self>>;
    fn prev(&self) -> Option<NonNull<Self>>;
    fn set_next(&mut self, node: Option<NonNull<Self>>);
    fn set_prev(&mut self, node: Option<NonNull<Self>>);
}

pub struct ListQueue<N: Node> {
    head: Option<NonNull<N>>,
    tail: Option<NonNull<N>>,
}

impl<N: Node> ListQueue<N> {
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head == None
    }

    pub fn push(&mut self, mut node: NonNull<N>) {
        if let Some(mut prev) = self.tail {
            unsafe {
                prev.as_mut().set_next(Some(node));
                node.as_mut().set_prev(Some(prev));
                node.as_mut().set_next(None);
            }
        }
        self.tail = Some(node);
        if let None = self.head {
            self.head = Some(node);
        }
    }

    pub fn pop(&mut self) -> Option<NonNull<N>> {
        if let Some(mut node) = self.head {
            unsafe {
                let mut next_node = node.as_ref().next();
                node.as_mut().set_next(None);
                self.head = next_node;
                if let Some(mut next) = next_node {
                    next.as_mut().set_prev(None);
                } else {
                    self.tail = None;
                }
                return Some(node);
            }
        }
        None
    }
}

pub struct ListIter<N: Node> {
    ptr: Option<NonNull<N>>,
}

impl<N: Node + 'static> Iterator for ListIter<N> {
    type Item = &'static mut N;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(mut node) = self.ptr {
            let data = unsafe { node.as_mut() };
            self.ptr = data.next();
            Some(data)
        } else {
            None
        }
    }
}
