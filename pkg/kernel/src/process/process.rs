use super::ProcessId;
use super::*;
use crate::memory::gdt::get_selector;
use crate::memory::{self, *};
use crate::utils::{Registers, RegistersValue};
use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::intrinsics::copy_nonoverlapping;
use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::registers::rflags::RFlags;
use x86_64::structures::idt::{InterruptStackFrame, InterruptStackFrameValue};
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::page::PageRange;
use x86_64::structures::paging::*;
use x86_64::VirtAddr;

pub struct Process {
    pid: ProcessId,
    regs: RegistersValue,
    name: String,
    parent: ProcessId,
    status: ProgramStatus,
    ticks_passed: usize,
    children: Vec<ProcessId>,
    stack_frame: InterruptStackFrameValue,
    page_table_addr: (PhysFrame, Cr3Flags),
    page_table: Option<OffsetPageTable<'static>>,
    proc_data: ProcessData,
}

#[derive(Clone, Debug)]
pub struct ProcessData {
    env: BTreeMap<String, String>,
    stack_segment: Option<PageRange>,
    pub stack_memory_usage: usize,
}

impl Default for ProcessData {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessData {
    pub fn new() -> Self {
        let env = BTreeMap::new();
        let stack_segment = None;
        Self {
            env,
            stack_segment,
            stack_memory_usage: 0,
        }
    }

    pub fn set_env(mut self, key: &str, val: &str) -> Self {
        self.env.insert(key.into(), val.into());
        self
    }

    pub fn set_stack(&mut self, start: u64, size: u64) {
        let start = Page::containing_address(VirtAddr::new(start));
        self.stack_segment = Some(Page::range(start, start + size));
        self.stack_memory_usage = size as usize;
    }
}

impl Process {
    pub fn pid(&self) -> ProcessId {
        self.pid
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn tick(&mut self) {
        self.ticks_passed += 1;
    }

    pub fn status(&self) -> ProgramStatus {
        self.status
    }

    pub fn pause(&mut self) {
        self.status = ProgramStatus::Ready;
    }

    pub fn resume(&mut self) {
        self.status = ProgramStatus::Running;
    }

    pub fn block(&mut self) {
        self.status = ProgramStatus::Blocked;
    }

    pub fn set_page_table_with_cr3(&mut self) {
        self.page_table_addr = Cr3::read();
    }

    pub fn page_table_addr(&self) -> PhysFrame {
        self.page_table_addr.0
    }

    pub fn is_running(&self) -> bool {
        self.status == ProgramStatus::Running
    }

    pub fn env(&self, key: &str) -> Option<String> {
        self.proc_data.env.get(key).cloned()
    }

    pub fn set_env(&mut self, key: &str, val: &str) {
        self.proc_data.env.insert(key.into(), val.into());
    }

    pub fn save(&mut self, regs: &mut Registers, sf: &mut InterruptStackFrame) {
        self.regs = unsafe { regs.as_mut().as_ptr().read() };
        self.stack_frame = unsafe { sf.as_mut().read() };
        self.status = ProgramStatus::Ready;
    }

    pub fn restore(&mut self, regs: &mut Registers, sf: &mut InterruptStackFrame) {
        unsafe {
            regs.as_mut().as_mut_ptr().write(self.regs);
            sf.as_mut().write(self.stack_frame);
            Cr3::write(self.page_table_addr.0, self.page_table_addr.1)
        }
        self.status = ProgramStatus::Running;
    }

    pub fn init_stack_frame(&mut self, entry: VirtAddr, stack_top: VirtAddr) {
        self.stack_frame.stack_pointer = stack_top;
        self.stack_frame.instruction_pointer = entry;
        self.stack_frame.cpu_flags =
            (RFlags::IOPL_HIGH | RFlags::IOPL_LOW | RFlags::INTERRUPT_FLAG).bits();
        let selector = get_selector();
        self.stack_frame.code_segment = selector.code_selector.0 as u64;
        self.stack_frame.stack_segment = selector.data_selector.0 as u64;
        trace!("Init stack frame: {:#?}", &self.stack_frame);
    }

    pub fn alloc_init_stack(&mut self) -> VirtAddr {
        // stack top set by pid
        let offset = self.pid().0 as u64 * STACK_MAX_SIZE;
        let stack_top = STACK_INIT_TOP - offset;
        let stack_bottom = STACT_INIT_BOT - offset;

        let stack_top_addr = VirtAddr::new(stack_top);
        let page_table = self.page_table.as_mut().unwrap();
        let alloc = &mut *get_frame_alloc_for_sure();

        elf::map_range(stack_bottom, STACK_DEF_PAGE, page_table, alloc, true).unwrap();

        self.proc_data.set_stack(stack_bottom, STACK_DEF_PAGE);
        stack_top_addr
    }

    fn clone_page_table(
        page_table_source: PhysFrame,
        frame_alloc: &mut BootInfoFrameAllocator,
    ) -> (OffsetPageTable<'static>, PhysFrame) {
        let page_table_addr = frame_alloc
            .allocate_frame()
            .expect("Cannot alloc page table for new process.");

        // 2. copy current page table to new page table
        unsafe {
            copy_nonoverlapping::<PageTable>(
                physical_to_virtual(page_table_source.start_address().as_u64()) as *mut PageTable,
                physical_to_virtual(page_table_addr.start_address().as_u64()) as *mut PageTable,
                1,
            );
        }

        // 3. create page table object
        let page_table = Self::page_table_from_phyframe(page_table_addr);

        (page_table, page_table_addr)
    }

    pub fn new(
        frame_alloc: &mut BootInfoFrameAllocator,
        name: String,
        parent: ProcessId,
        page_table_source: PhysFrame,
        proc_data: Option<ProcessData>,
    ) -> Self {
        let name = name.to_ascii_lowercase();

        // 1. create page table
        let (page_table, page_table_addr) = Self::clone_page_table(page_table_source, frame_alloc);

        trace!("Alloc page table for {}: {:?}", name, page_table_addr);

        // 2. create context
        let status = ProgramStatus::Created;
        let stack_frame = InterruptStackFrameValue {
            instruction_pointer: VirtAddr::new_truncate(0),
            code_segment: 8,
            cpu_flags: 0,
            stack_pointer: VirtAddr::new_truncate(0),
            stack_segment: 0,
        };
        let regs = RegistersValue::default();
        let ticks_passed = 0;
        let pid = ProcessId::new();

        trace!("New process {}#{} created.", name, pid);

        // 3. create process object
        Self {
            pid,
            name,
            parent,
            status,
            ticks_passed,
            stack_frame,
            regs,
            page_table_addr: (page_table_addr, Cr3::read().1),
            page_table: Some(page_table),
            children: Vec::new(),
            proc_data: proc_data.unwrap_or_default(),
        }
    }

    pub fn remove_child(&mut self, child: ProcessId) {
        self.children.retain(|c| *c != child);
    }

    pub fn parent(&self) -> ProcessId {
        self.parent
    }

    pub fn is_on_stack(&self, addr: VirtAddr) -> bool {
        if let Some(stack_range) = self.proc_data.stack_segment {
            let addr = addr.as_u64();
            let cur_stack_bot = stack_range.start.start_address().as_u64();
            trace!("Current stack bot: {:#x}", cur_stack_bot);
            trace!("Address to access: {:#x}", addr);
            addr & STACK_START_MASK == cur_stack_bot & STACK_START_MASK
        } else {
            false
        }
    }

    pub fn try_alloc_new_stack_page(&mut self, addr: VirtAddr) -> Result<(), MapToError<Size4KiB>> {
        let alloc = &mut *get_frame_alloc_for_sure();
        let start_page = Page::<Size4KiB>::containing_address(addr);
        let pages = self.proc_data.stack_segment.unwrap().start - start_page;
        let page_table = self.page_table.as_mut().unwrap();
        trace!(
            "Fill missing pages...[{:#x} -> {:#x}) ({} pages)",
            start_page.start_address().as_u64(),
            self.proc_data
                .stack_segment
                .unwrap()
                .start
                .start_address()
                .as_u64(),
            pages
        );

        elf::map_range(addr.as_u64(), pages, page_table, alloc, true)?;

        let end_page = self.proc_data.stack_segment.unwrap().end;
        let new_stack = PageRange {
            start: start_page,
            end: end_page,
        };

        self.proc_data.stack_memory_usage = new_stack.count();
        self.proc_data.stack_segment = Some(new_stack);

        Ok(())
    }

    fn clone_range(&self, cur_addr: u64, dest_addr: u64, size: usize) {
        trace!("Clone range: {:#x} -> {:#x}", cur_addr, dest_addr);
        unsafe {
            copy_nonoverlapping::<u8>(
                cur_addr as *mut u8,
                dest_addr as *mut u8,
                size * Size4KiB::SIZE as usize,
            );
        }
    }

    fn page_table_from_phyframe(frame: PhysFrame) -> OffsetPageTable<'static> {
        unsafe {
            OffsetPageTable::new(
                (physical_to_virtual(frame.start_address().as_u64()) as *mut PageTable)
                    .as_mut()
                    .unwrap(),
                VirtAddr::new_truncate(*PHYSICAL_OFFSET.get().unwrap()),
            )
        }
    }

    pub fn set_parent(&mut self, pid: ProcessId) {
        self.parent = pid;
    }

    pub fn children(&self) -> Vec<ProcessId> {
        self.children.clone()
    }

    pub fn memory_usage(&self) -> usize {
        self.proc_data.stack_memory_usage
    }
}

impl Drop for Process {
    /// Drop the process, free the stack and page table
    ///
    /// this will be called when the process is removed from the process list
    fn drop(&mut self) {}
}

impl core::fmt::Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let mut f = f.debug_struct("Process");
        f.field("pid", &self.pid);
        f.field("name", &self.name);
        f.field("parent", &self.parent);
        f.field("status", &self.status);
        f.field("ticks_passed", &self.ticks_passed);
        f.field("children", &self.children);
        f.field("page_table_addr", &self.page_table_addr);
        f.field("status", &self.status);
        f.field("stack_top", &self.stack_frame.stack_pointer);
        f.field("cpu_flags", &self.stack_frame.cpu_flags);
        f.field("instruction_pointer", &self.stack_frame.instruction_pointer);
        f.field("stack", &self.proc_data.stack_segment);
        f.field("regs", &self.regs);
        f.finish()
    }
}

impl core::fmt::Display for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let (size, unit) = memory::humanized_size(self.memory_usage() as u64 * 4096);
        write!(
            f,
            " #{:-3} | #{:-3} | {:12} | {:7} | {:>5.1} {} | {:?}",
            u16::from(self.pid),
            u16::from(self.parent),
            self.name,
            self.ticks_passed,
            size,
            unit,
            self.status
        )?;
        Ok(())
    }
}
