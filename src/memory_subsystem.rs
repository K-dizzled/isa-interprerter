use crate::instruction::WriteOperation;
use std::collections::{HashMap, VecDeque};

pub struct Memory {
    pub data: HashMap<String, usize>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn load(&self, addr: &str) -> usize {
        *self.data.get(addr).unwrap_or(&0)
    }

    pub fn store(&mut self, addr: &str, value: usize) {
        self.data.insert(addr.to_string(), value);
    }
}

impl std::fmt::Display for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut keys: Vec<&String> = self.data.keys().collect();
        keys.sort();
        for key in keys {
            writeln!(f, "{}: {}", key, self.data[key])?;
        }
        Ok(())
    }
}

pub trait MemorySubsystem {
    fn store(&mut self, addr: &str, value: usize, thread_id: usize);
    fn load(&self, addr: &str, thread_id: usize) -> usize;
    fn propagate(&mut self, thread_id: usize);
}

pub struct SCMemorySubsystem {
    pub memory: Memory,
}

impl SCMemorySubsystem {
    pub fn new() -> Self {
        Self {
            memory: Memory::new(),
        }
    }
}

impl MemorySubsystem for SCMemorySubsystem {
    fn store(&mut self, addr: &str, value: usize, _thread_id: usize) {
        self.memory.store(addr, value);
    }
    fn load(&self, addr: &str, _thread_id: usize) -> usize {
        self.memory.load(addr)
    }
    fn propagate(&mut self, _thread_id: usize) {}
}

pub struct Buffer {
    operations: VecDeque<WriteOperation>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            operations: VecDeque::new(),
        }
    }

    pub fn load(&self, addr: &str) -> Option<usize> {
        self.operations
            .iter()
            .rev()
            .find(|op| op.addr == addr)
            .map(|op| op.value)
    }

    pub fn push(&mut self, operation: WriteOperation) {
        self.operations.push_back(operation);
    }

    pub fn propagate(&mut self) -> Option<WriteOperation> {
        self.operations.pop_front()
    }
}

pub struct TSOMemorySubsystem {
    pub memory: Memory,
    pub buffers: HashMap<usize, Buffer>,
}

impl TSOMemorySubsystem {
    pub fn new() -> Self {
        Self {
            memory: Memory::new(),
            buffers: HashMap::new(),
        }
    }
}

impl MemorySubsystem for TSOMemorySubsystem {
    fn store(&mut self, addr: &str, value: usize, thread_id: usize) {
        self.buffers
            .entry(thread_id)
            .or_insert(Buffer::new())
            .push(WriteOperation::new(addr.to_string(), value));
    }

    fn load(&self, addr: &str, thread_id: usize) -> usize {
        self.buffers
            .get(&thread_id)
            .and_then(|buffer| buffer.load(addr))
            .unwrap_or_else(|| self.memory.load(addr))
    }

    fn propagate(&mut self, thread_id: usize) {
        let write = self.buffers.get_mut(&thread_id).unwrap().propagate();
        if let Some(write) = write {
            self.memory.store(&write.addr, write.value);
        }
    }
}
