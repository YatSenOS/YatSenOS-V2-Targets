use super::ProcessId;
use super::*;
use crate::memory::{self, *};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use spin::*;
use x86_64::structures::paging::mapper::{CleanUp, MapToError};
use x86_64::structures::paging::page::PageRange;
use x86_64::structures::paging::*;
use x86_64::VirtAddr;
use xmas_elf::ElfFile;

#[derive(Clone)]
pub struct Process {
    pid: ProcessId,
    inner: Arc<RwLock<ProcessInner>>,
}

pub struct ProcessInner {
    name: String,
    parent: Option<Weak<Process>>,
    children: Vec<Arc<Process>>,
    ticks_passed: usize,
    status: ProgramStatus,
    exit_code: Option<isize>,
    context: ProcessContext,
    page_table: Option<PageTableContext>,
    proc_data: Option<ProcessData>,
}

impl Process {
    #[inline]
    pub fn pid(&self) -> ProcessId {
        self.pid
    }

    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<ProcessInner> {
        self.inner.write()
    }

    #[inline]
    pub fn read(&self) -> RwLockReadGuard<ProcessInner> {
        self.inner.read()
    }

    pub fn new(
        name: String,
        parent: Option<Weak<Process>>,
        page_table: PageTableContext,
        proc_data: Option<ProcessData>,
    ) -> Arc<Self> {
        let name = name.to_ascii_lowercase();

        // create context
        let pid = ProcessId::new();

        let inner = ProcessInner {
            name,
            parent,
            status: ProgramStatus::Ready,
            context: ProcessContext::default(),
            ticks_passed: 0,
            exit_code: None,
            children: Vec::new(),
            page_table: Some(page_table),
            proc_data: Some(proc_data.unwrap_or_default()),
        };

        trace!("New process {}#{} created.", &inner.name, pid);

        // create process struct
        Arc::new(Self {
            pid,
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    pub fn kill(&self, ret: isize) {
        let mut inner = self.inner.write();

        debug!(
            "Killing process {}#{} with ret code: {}",
            inner.name(),
            self.pid,
            ret
        );

        inner.kill(self.pid, ret);
    }
}

impl ProcessInner {
    pub fn name(&self) -> &str {
        &self.name
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

    pub fn is_ready(&self) -> bool {
        self.status == ProgramStatus::Ready
    }

    pub fn exit_code(&self) -> Option<isize> {
        self.exit_code
    }

    pub fn clont_page_table(&self) -> PageTableContext {
        self.page_table.as_ref().unwrap().clone()
    }

    /// Save the process's context
    /// mark the process as ready
    pub(super) fn save(&mut self, context: &ProcessContext) {
        self.context.save(context);
        self.status = ProgramStatus::Ready;
    }

    /// Restore the process's context
    /// mark the process as running
    pub(super) fn restore(&mut self, context: &mut ProcessContext) {
        self.context.restore(context);
        self.page_table.as_ref().unwrap().load();
        self.status = ProgramStatus::Running;
    }

    pub fn init_stack_frame(&mut self, entry: VirtAddr, stack_top: VirtAddr) {
        self.context.init_stack_frame(entry, stack_top)
    }

    pub fn parent(&self) -> Option<Arc<Process>> {
        self.parent.as_ref().and_then(|p| p.upgrade())
    }

    pub fn load_elf(&mut self, elf: &ElfFile) {
        let alloc = &mut *get_frame_alloc_for_sure();

        let page_table = self.page_table.as_ref().unwrap();
        let mut mapper = page_table.mapper();

        let code_segments = elf::load_elf(
            elf,
            *PHYSICAL_OFFSET.get().unwrap(),
            &mut mapper,
            alloc,
            true,
        )
        .unwrap();

        let stack_segment =
            elf::map_range(STACT_INIT_BOT, STACK_DEF_PAGE, &mut mapper, alloc, true).unwrap();

        // record memory usage
        let proc_data = self.proc_data.as_mut().unwrap();
        proc_data.code_memory_usage = code_segments.iter().map(|seg| seg.count()).sum();
        proc_data.stack_memory_usage = stack_segment.count();
        proc_data.code_segments = Some(code_segments);
        proc_data.stack_segment = Some(stack_segment);
    }

    pub fn try_alloc_new_stack_page(&mut self, addr: VirtAddr) -> Result<(), MapToError<Size4KiB>> {
        let alloc = &mut *get_frame_alloc_for_sure();
        let new_start_page = Page::<Size4KiB>::containing_address(addr);
        let old_stack = self.proc_data.as_ref().unwrap().stack_segment.unwrap();

        let pages = old_stack.start - new_start_page;
        let page_table = &mut self.page_table.as_mut().unwrap().mapper();

        trace!(
            "Fill missing pages...[{:#x} -> {:#x}) ({} pages)",
            new_start_page.start_address().as_u64(),
            old_stack.start.start_address().as_u64(),
            pages
        );

        let user_access = processor::current_pid() != KERNEL_PID;
        elf::map_range(addr.as_u64(), pages, page_table, alloc, user_access)?;

        let new_stack = PageRange {
            start: new_start_page,
            end: old_stack.end,
        };

        let proc_data = self.proc_data.as_mut().unwrap();
        proc_data.stack_memory_usage = new_stack.count();
        proc_data.stack_segment = Some(new_stack);

        Ok(())
    }

    pub fn kill(&mut self, pid: ProcessId, ret: isize) {
        self.clean_up_page_table(pid);
        self.proc_data.take();
        self.page_table.take();
        self.exit_code = Some(ret);
        self.status = ProgramStatus::Dead;
    }

    fn clean_up_page_table(&mut self, pid: ProcessId) {
        let page_table = self.page_table.take().unwrap();
        let mut mapper = page_table.mapper();

        let frame_deallocator = &mut *get_frame_alloc_for_sure();

        let proc_data = self.proc_data.as_mut().unwrap();
        let stack = proc_data.stack_segment.unwrap();

        trace!(
            "Free stack for {}#{}: [{:#x} -> {:#x}) ({} frames)",
            self.name,
            pid,
            stack.start.start_address(),
            stack.end.start_address(),
            stack.count()
        );

        elf::unmap_range(
            stack.start.start_address().as_u64(),
            stack.count() as u64,
            &mut mapper,
            frame_deallocator,
            true,
        )
        .unwrap();

        // clean up page table when no other process using it
        if page_table.using_count() == 1 {
            trace!("Clean up page table for {}#{}", self.name, pid);
            unsafe {
                if let Some(ref mut segments) = proc_data.code_segments {
                    for range in segments {
                        for page in range {
                            if let Ok(ret) = mapper.unmap(page) {
                                frame_deallocator.deallocate_frame(ret.0);
                                ret.1.flush();
                            }
                        }
                    }
                }
                // free P1-P3
                mapper.clean_up(frame_deallocator);
                // free P4
                frame_deallocator.deallocate_frame(page_table.reg.addr);
            }
        }

        drop(page_table);
    }
}

impl core::ops::Deref for Process {
    type Target = Arc<RwLock<ProcessInner>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl core::ops::Deref for ProcessInner {
    type Target = ProcessData;

    fn deref(&self) -> &Self::Target {
        self.proc_data
            .as_ref()
            .expect("Process data empty. The process may be killed.")
    }
}

impl core::ops::DerefMut for ProcessInner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.proc_data
            .as_mut()
            .expect("Process data empty. The process may be killed.")
    }
}

impl core::fmt::Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let mut f = f.debug_struct("Process");
        f.field("pid", &self.pid);

        let inner = self.inner.read();
        f.field("name", &inner.name);
        f.field("parent", &inner.parent().map(|p| p.pid));
        f.field("status", &inner.status);
        f.field("ticks_passed", &inner.ticks_passed);
        f.field("children", &inner.children.iter().map(|c| c.pid.0));
        f.field("page_table", &inner.page_table);
        f.field("status", &inner.status);
        f.field("stack", &inner.stack_segment);
        f.field("context", &inner.context);
        f.finish()
    }
}

impl core::fmt::Display for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let inner = self.inner.read();
        let (size, unit) = memory::humanized_size(inner.memory_usage() as u64 * 4096);
        write!(
            f,
            " #{:-3} | #{:-3} | {:12} | {:7} | {:>5.1} {} | {:?}",
            u16::from(self.pid),
            inner.parent().map(|p| u16::from(p.pid)).unwrap_or(0),
            inner.name,
            inner.ticks_passed,
            size,
            unit,
            inner.status
        )?;
        Ok(())
    }
}
