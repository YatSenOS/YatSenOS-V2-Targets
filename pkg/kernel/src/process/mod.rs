mod manager;

#[allow(clippy::module_inception)]
mod process;

use core::sync::atomic::{AtomicU16, Ordering};

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use manager::*;
use process::*;

pub use process::ProcessData;
use xmas_elf::ElfFile;

use crate::{Registers, Resource};
use alloc::string::String;
use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::{
    registers::control::{Cr2, Cr3},
    structures::idt::InterruptStackFrame,
    VirtAddr,
};

use self::manager::init_PROCESS_MANAGER;

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProgramStatus {
    Created,
    Running,
    Ready,
    Blocked,
    Dead,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessId(pub u16);

impl ProcessId {
    pub fn new() -> Self {
        static NEXT_PID: AtomicU16 = AtomicU16::new(0);
        ProcessId(NEXT_PID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Debug for ProcessId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ProcessId> for u16 {
    fn from(pid: ProcessId) -> Self {
        pid.0
    }
}

/// init process manager
pub fn init(boot_info: &'static boot::BootInfo) {
    let mut alloc = crate::memory::get_frame_alloc_for_sure();
    let kproc_data = ProcessData::new();
    trace!("Init process data: {:#?}", kproc_data);
    // kernel process
    let mut kproc = Process::new(
        &mut alloc,
        String::from("kernel"),
        ProcessId::new(),
        Cr3::read().0,
        Some(kproc_data),
    );
    kproc.resume();
    kproc.set_page_table_with_cr3();

    let app_list = boot_info.loaded_apps.as_ref();

    init_PROCESS_MANAGER(ProcessManager::new(kproc, app_list));
    info!("Process Manager Initialized.");
}

pub fn switch(regs: &mut Registers, sf: &mut InterruptStackFrame) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut manager = get_process_manager_for_sure();
        manager.save_current(regs, sf);
        manager.switch_next(regs, sf);
    });
}

pub fn print_process_list() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager_for_sure().print_process_list();
    })
}

pub fn env(key: &str) -> Option<String> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager_for_sure().current().env(key)
    })
}

pub fn process_exit(ret: isize, regs: &mut Registers, sf: &mut InterruptStackFrame) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut manager = get_process_manager_for_sure();
        manager.kill_self(ret);
        manager.switch_next(regs, sf);
    })
}

pub fn wait_pid(pid: ProcessId) -> isize {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager_for_sure().wait_pid(pid)
    })
}

pub fn handle(fd: u8) -> Option<Resource> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager_for_sure().current().handle(fd)
    })
}

pub fn still_alive(pid: ProcessId) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager_for_sure().still_alive(pid)
    })
}

pub fn current_pid() -> ProcessId {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager_for_sure().current_pid()
    })
}

pub fn kill(pid: ProcessId, regs: &mut Registers, sf: &mut InterruptStackFrame) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut manager = get_process_manager_for_sure();
        if pid == manager.current_pid() {
            manager.kill_self(0xdead);
            manager.switch_next(regs, sf);
        } else {
            manager.kill(pid, 0xdead);
        }
    })
}

pub fn try_resolve_page_fault(
    _err_code: PageFaultErrorCode,
    _sf: &mut InterruptStackFrame,
) -> Result<(), ()> {
    let addr = Cr2::read();
    debug!("Trying to access address: {:?}", addr);

    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager_for_sure();
        debug!("Current process: {:#?}", manager.current());
    });

    Err(())
}

pub fn list_app() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager_for_sure();
        let app_list = manager.app_list();

        if app_list.is_none() {
            println!(">>> No app found in list!");
            return;
        }

        let apps = manager
            .app_list()
            .unwrap()
            .iter()
            .map(|app| app.name.as_str())
            .collect::<Vec<&str>>()
            .join(", ");

        println!(">>> App list: {}", apps);
    });
}

pub fn spawn(name: &str) -> Result<ProcessId, String> {
    let app = x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager_for_sure();
        let app_list = manager.app_list()?;

        app_list.iter().find(|&app| app.name.eq(name))
    });

    if app.is_none() {
        return Err(format!("App not found: {}", name));
    };

    elf_spawn(name.to_string(), &app.unwrap().elf)
}

pub fn elf_spawn(name: String, elf: &ElfFile) -> Result<ProcessId, String> {
    let pid = x86_64::instructions::interrupts::without_interrupts(|| {
        let mut manager = get_process_manager_for_sure();
        let process_name = name.to_lowercase();

        let parent = manager.current().pid();
        let pid = manager.spawn(elf, name, parent, Some(ProcessData::new()));

        debug!("Spawned process: {}#{}", process_name, pid);
        pid
    });

    Ok(pid)
}

pub fn force_show_info() {
    unsafe {
        manager::PROCESS_MANAGER.get().unwrap().force_unlock();
    }

    debug!("{:#?}", get_process_manager_for_sure().current())
}

pub fn handle_page_fault(addr: VirtAddr, err_code: PageFaultErrorCode) -> Result<(), ()> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager_for_sure().handle_page_fault(addr, err_code)
    })
}
