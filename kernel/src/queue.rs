pub struct Queue<T, const N: usize> {
    buffer: [T; N],
    put_idx: usize,
    get_idx: usize,
    last_op: bool,
}

impl<T: Clone + Copy, const N: usize> Queue<T, N> {
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
