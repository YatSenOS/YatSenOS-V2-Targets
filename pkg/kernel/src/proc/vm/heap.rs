use core::sync::atomic::{AtomicU64, Ordering};

use alloc::sync::Arc;
use x86_64::{
    structures::paging::{mapper::UnmapError, FrameDeallocator, Mapper, Page},
    VirtAddr,
};

use super::{FrameAllocatorRef, MapperRef};

// user process runtime heap
// 0x100000000 bytes -> 4GiB
// from 0x0000_2000_0000_0000 to 0x0000_2000_ffff_fff8
pub const HEAP_START: u64 = 0x2000_0000_0000;
pub const HEAP_PAGES: u64 = 0x100000;
pub const HEAP_SIZE: u64 = HEAP_PAGES * crate::memory::PAGE_SIZE;
pub const HEAP_END: u64 = HEAP_START + HEAP_SIZE - 8;

pub struct Heap {
    base: VirtAddr,
    end: Arc<AtomicU64>,
}

impl Heap {
    pub fn empty() -> Self {
        Self {
            base: VirtAddr::new(HEAP_START),
            end: Arc::new(AtomicU64::new(HEAP_START)),
        }
    }

    pub fn fork(&self) -> Self {
        Self {
            base: self.base,
            end: self.end.clone(),
        }
    }

    // pub fn brk(&mut self, new_end: Option<VirtAddr>, _mapper: MapperRef) -> Option<VirtAddr> {
    //     // TODO: Implement this function
    //     None
    // }

    #[inline]
    pub fn memory_usage(&self) -> u64 {
        self.end.load(Ordering::Relaxed) - self.base.as_u64()
    }

    pub(super) fn clean_up(
        &self,
        mapper: MapperRef,
        dealloc: FrameAllocatorRef,
    ) -> Result<(), UnmapError> {
        if self.memory_usage() == 0 {
            return Ok(());
        }

        // load the current end address and reset it to base
        let end_addr = self.end.swap(self.base.as_u64(), Ordering::Relaxed);

        let start_page = Page::containing_address(self.base);
        let end_page = Page::containing_address(VirtAddr::new(end_addr));

        for page in Page::range_inclusive(start_page, end_page) {
            let (frame, flush) = mapper.unmap(page)?;
            trace!("Unmap heap page: {:#x}", page.start_address());

            unsafe {
                dealloc.deallocate_frame(frame);
            }
            flush.flush();
        }

        Ok(())
    }
}

impl core::fmt::Debug for Heap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Heap")
            .field("base", &self.base)
            .field("end", &self.end.load(Ordering::Relaxed))
            .finish()
    }
}
