use alloc::{collections::BTreeMap, string::String, sync::Arc};
use spin::RwLock;
use x86_64::{
    structures::paging::{
        page::PageRange,
        Page,
    },
    VirtAddr,
};


use super::*;

#[derive(Debug, Clone)]
pub struct ProcessData {
    // shared data
    pub(super) env: Arc<RwLock<BTreeMap<String, String>>>,
    pub(super) semaphores: Arc<RwLock<SemaphoreSet>>,

    // process specific data
    pub(super) stack_segment: Option<PageRange>,
    pub(super) stack_memory_usage: usize,
}

impl Default for ProcessData {
    fn default() -> Self {
        Self {
            env: Arc::new(RwLock::new(BTreeMap::new())),
            semaphores: Arc::new(RwLock::new(SemaphoreSet::default())),
            stack_segment: None,
            stack_memory_usage: 0,
        }
    }
}

impl ProcessData {
    pub fn new() -> Self {
        Self::default()
    }


    pub fn env(&self, key: &str) -> Option<String> {
        self.env.read().get(key).cloned()
    }

    pub fn set_env(self, key: &str, val: &str) -> Self {
        self.env.write().insert(key.into(), val.into());
        self
    }

    pub fn set_stack(&mut self, start: VirtAddr, size: u64) {
        let start = Page::containing_address(start);
        self.stack_segment = Some(Page::range(start, start + size));
        self.stack_memory_usage = size as usize;
    }

    pub fn is_on_stack(&self, addr: VirtAddr) -> bool {
        if let Some(stack_range) = self.stack_segment.as_ref() {
            let addr = addr.as_u64();
            let cur_stack_bot = stack_range.start.start_address().as_u64();
            trace!("Current stack bot: {:#x}", cur_stack_bot);
            trace!("Address to access: {:#x}", addr);
            addr & STACK_START_MASK == cur_stack_bot & STACK_START_MASK
        } else {
            false
        }
    }

    #[inline]
    pub fn new_sem(&mut self, key: u32, value: usize) -> bool {
        self.semaphores.write().insert(key, value)
    }

    #[inline]
    pub fn remove_sem(&mut self, key: u32) -> bool {
        self.semaphores.write().remove(key)
    }

    #[inline]
    pub fn sem_up(&mut self, key: u32) -> SemaphoreResult {
        self.semaphores.read().up(key)
    }

    #[inline]
    pub fn sem_down(&mut self, key: u32, pid: ProcessId) -> SemaphoreResult {
        self.semaphores.read().down(key, pid)
    }
}
