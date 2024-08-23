use alloc::vec;
use alloc::vec::Vec;

use super::{Block, BlockIO};

pub struct RamDisk {
    storage: Vec<Block<512>>,
}

impl RamDisk {
    pub fn new(size: usize) -> Self {
        Self {
            storage: vec![Block::empty(); (size + 511) / 512],
        }
    }
}

impl BlockIO for RamDisk {
    fn read(&mut self, address: u32, buffer: &mut [Block<512>]) -> Result<usize, usize> {
        buffer.clone_from_slice(&self.storage[address as usize..(address as usize + buffer.len())]);
        Ok(buffer.len())
    }

    fn write(&mut self, address: u32, buffer: &[Block<512>]) -> Result<usize, usize> {
        self.storage[address as usize..(address as usize + buffer.len())].clone_from_slice(&buffer);
        Ok(buffer.len())
    }

    fn max_addr(&self) -> u32 {
        self.storage.len() as u32
    }
}
