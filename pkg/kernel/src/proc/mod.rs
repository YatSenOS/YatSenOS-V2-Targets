mod context;
mod data;
mod manager;
mod paging;
mod pid;
mod process;
mod processor;

use alloc::sync::Arc;
use alloc::vec::Vec;
use manager::*;
use paging::*;
use process::*;

pub use context::ProcessContext;
pub use data::ProcessData;
pub use pid::ProcessId;
use xmas_elf::ElfFile;

use alloc::string::{String, ToString};
use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::VirtAddr;

use crate::Resource;

// 0xffff_ff00_0000_0000 is the kernel's address space
pub const STACK_MAX: u64 = 0x0000_4000_0000_0000;
// stack max addr, every thread has a stack space
// from 0x????_????_0000_0000 to 0x????_????_ffff_ffff
// 0x100000000 bytes -> 4GiB
// allow 0x2000 (4096) threads run as a time
// 0x????_2000_????_???? -> 0x????_3fff_????_????
// init alloc stack has size of 0x2000 (2 frames)
// every time we meet a page fault, we alloc more frames
pub const STACK_MAX_PAGES: u64 = 0x100000;
pub const STACK_MAX_SIZE: u64 = STACK_MAX_PAGES * crate::memory::PAGE_SIZE;
pub const STACK_START_MASK: u64 = !(STACK_MAX_SIZE - 1);
// [bot..0x2000_0000_0000..top..0x3fff_ffff_ffff]
// init stack
pub const STACK_DEF_BOT: u64 = STACK_MAX - STACK_MAX_SIZE;
pub const STACK_DEF_PAGE: u64 = 1;
pub const STACK_DEF_SIZE: u64 = STACK_DEF_PAGE * crate::memory::PAGE_SIZE;
pub const STACT_INIT_BOT: u64 = STACK_MAX - STACK_DEF_SIZE;
pub const STACK_INIT_TOP: u64 = STACK_MAX - 8;
// [bot..0xffffff0100000000..top..0xffffff01ffffffff]
// kernel stack
pub const KSTACK_MAX: u64 = 0xffff_ff02_0000_0000;
pub const KSTACK_DEF_BOT: u64 = KSTACK_MAX - STACK_MAX_SIZE;
pub const KSTACK_DEF_PAGE: u64 = 8;
pub const KSTACK_DEF_SIZE: u64 = KSTACK_DEF_PAGE * crate::memory::PAGE_SIZE;

pub const KSTACK_INIT_BOT: u64 = KSTACK_MAX - KSTACK_DEF_SIZE;
pub const KSTACK_INIT_TOP: u64 = KSTACK_MAX - 8;

pub const KERNEL_PID: ProcessId = ProcessId(1);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProgramStatus {
    Running,
    Ready,
    Blocked,
    Dead,
}

/// init process manager
pub fn init(boot_info: &'static boot::BootInfo) {
    let kproc_data = ProcessData::new().set_stack(KSTACK_INIT_BOT, KSTACK_DEF_PAGE);

    trace!("Init process data: {:#?}", kproc_data);

    // kernel process
    let kproc = Process::new(
        String::from("kernel"),
        None,
        PageTableContext::new(),
        Some(kproc_data),
    );

    kproc.write().resume();
    let app_list = boot_info.loaded_apps.as_ref();
    manager::init(kproc, app_list);

    info!("Process Manager Initialized.");
}

pub fn switch(context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        let pid = manager.save_current(context);
        manager.push_ready(pid);
        manager.switch_next(context);
    });
}

pub fn print_process_list() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().print_process_list();
    })
}

pub fn env(key: &str) -> Option<String> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().current().read().env(key)
    })
}

pub fn process_exit(ret: isize, context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        manager.kill_self(ret);
        manager.switch_next(context);
    })
}

pub fn wait_pid(pid: ProcessId) -> isize {
    x86_64::instructions::interrupts::without_interrupts(|| get_process_manager().wait_pid(pid))
}

pub fn handle(fd: u8) -> Option<Resource> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().current().read().handle(fd)
    })
}

pub fn close(fd: u8) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| get_process_manager().close(fd))
}

pub fn still_alive(pid: ProcessId) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| get_process_manager().wait_pid(pid) < 0)
}

pub fn current_pid() -> ProcessId {
    x86_64::instructions::interrupts::without_interrupts(processor::current_pid)
}

pub fn kill(pid: ProcessId, context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        if pid == processor::current_pid() {
            manager.kill_self(0xdead);
            manager.switch_next(context);
        } else {
            manager.kill(pid, 0xdead);
        }
    })
}

pub fn spawn(name: &str) -> Result<ProcessId, String> {
    let app = x86_64::instructions::interrupts::without_interrupts(|| {
        let app_list = get_process_manager().app_list()?;

        app_list.iter().find(|&app| app.name.eq(name))
    });

    if app.is_none() {
        return Err(format!("App not found: {}", name));
    };

    elf_spawn(name.to_string(), &app.unwrap().elf)
}

pub fn elf_spawn(name: String, elf: &ElfFile) -> Result<ProcessId, String> {
    let pid = x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        let process_name = name.to_lowercase();

        let parent = Arc::downgrade(&manager.current());
        let proc_data = ProcessData::new();

        let pid = manager.spawn(elf, name, Some(parent), Some(proc_data));

        debug!("Spawned process: {}#{}", process_name, pid);
        pid
    });

    Ok(pid)
}

pub fn current_proc_info() {
    debug!("{:#?}", get_process_manager().current())
}

pub fn handle_page_fault(addr: VirtAddr, err_code: PageFaultErrorCode) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().handle_page_fault(addr, err_code)
    })
}

pub fn list_app() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let app_list = get_process_manager().app_list();
        if app_list.is_none() {
            println!(">>> No app found in list!");
            return;
        }

        let apps = app_list
            .unwrap()
            .iter()
            .map(|app| app.name.as_str())
            .collect::<Vec<&str>>()
            .join(", ");

        println!(">>> App list: {}", apps);
    });
}
