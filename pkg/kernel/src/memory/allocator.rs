// reference: https://github.com/xfoxfu/rust-xos/blob/main/kernel/src/allocator.rs

use linked_list_allocator::LockedHeap;
use x86_64::VirtAddr;

pub const HEAP_SIZE: usize = 8 * 1024 * 1024; // 8 MiB

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init() {
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

    let heap_start = VirtAddr::from_ptr(unsafe { HEAP.as_ptr() });
    let heap_end = heap_start + HEAP_SIZE as u64;

    unsafe {
        ALLOCATOR.lock().init(HEAP.as_mut_ptr(), HEAP_SIZE);
    }

    debug!(
        "Kernel Heap      : 0x{:016x}-0x{:016x}",
        heap_start.as_u64(),
        heap_end.as_u64()
    );

    let (size, unit) = crate::humanized_size(HEAP_SIZE as u64);
    info!("Kernel Heap Size : {:>7.*} {}", 3, size, unit);

    info!("Kernel Heap Initialized.");
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}
