use x86_64::{structures::paging::*, VirtAddr};

use crate::memory::*;

pub mod stack;

use self::stack::*;

use super::{PageTableContext, ProcessId};

type MapperRef<'a> = &'a mut OffsetPageTable<'static>;
type FrameAllocatorRef<'a> = &'a mut BootInfoFrameAllocator;

pub struct ProcessVm {
    // page table is shared by parent and child
    pub(super) page_table: PageTableContext,

    // stack is pre-process allocated
    pub(super) stack: Stack,
}

impl ProcessVm {
    pub fn new(page_table: PageTableContext) -> Self {
        Self {
            page_table,
            stack: Stack::empty(),
        }
    }

    pub fn init_proc_stack(&mut self, pid: ProcessId) -> VirtAddr {
        let offset = (pid.0 - 1) as u64 * STACK_MAX_SIZE;
        let stack_top = STACK_INIT_TOP - offset;
        let stack_bottom = STACK_INIT_BOT - offset;

        let stack_top_addr = VirtAddr::new(stack_top);
        let alloc = &mut *get_frame_alloc_for_sure();
        let mapper = &mut self.page_table.mapper();

        elf::map_pages(stack_bottom, STACK_DEF_PAGE, mapper, alloc).unwrap();

        let start = Page::containing_address(stack_top_addr);
        self.stack = Stack::new(start, STACK_DEF_PAGE);

        stack_top_addr
    }

    pub fn handle_page_fault(&mut self, addr: VirtAddr) -> bool {
        let mapper = &mut self.page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        self.stack.handle_page_fault(addr, mapper, alloc)
    }
}

impl core::fmt::Debug for ProcessVm {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("ProcessVm")
            .field("stack", &self.stack)
            .field("page_table", &self.page_table)
            .finish()
    }
}
