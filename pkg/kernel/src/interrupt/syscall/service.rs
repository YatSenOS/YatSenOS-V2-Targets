use core::alloc::Layout;

use crate::process::ProcessId;
use crate::utils::Registers;
use crate::utils::*;
use x86_64::structures::idt::InterruptStackFrame;

use super::SyscallArgs;

pub fn sys_clock() -> i64 {
    clock::now().timestamp_nanos_opt().unwrap_or_default()
}

pub fn sys_allocate(args: &SyscallArgs) -> usize {
    let layout = unsafe { (args.arg0 as *const Layout).as_ref().unwrap() };

    if layout.size() == 0 {
        return 0;
    }

    let ret = crate::memory::user::USER_ALLOCATOR
        .lock()
        .allocate_first_fit(*layout);

    match ret {
        Ok(ptr) => ptr.as_ptr() as usize,
        Err(_) => 0,
    }
}

pub fn sys_deallocate(args: &SyscallArgs) {
    let layout = unsafe { (args.arg1 as *const Layout).as_ref().unwrap() };

    if args.arg0 == 0 || layout.size() == 0 {
        return;
    }

    let ptr = args.arg0 as *mut u8;

    unsafe {
        crate::memory::user::USER_ALLOCATOR
            .lock()
            .deallocate(core::ptr::NonNull::new_unchecked(ptr), *layout);
    }
}

pub fn spawn_process(args: &SyscallArgs) -> usize {
    let name = unsafe {
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
            args.arg0 as *const u8,
            args.arg1,
        ))
    };

    let pid = crate::process::spawn(name);

    if pid.is_err() {
        warn!("spawn_process: failed to spawn process: {}", name);
        return 0;
    }

    u16::from(pid.unwrap()) as usize
}

pub fn sys_read(args: &SyscallArgs) -> usize {
    let fd = get_handle(args.arg0 as u8);
    if let Some(res) = fd {
        let buf = unsafe { core::slice::from_raw_parts_mut(args.arg1 as *mut u8, args.arg2) };
        if let Ok(size) = res.read(buf) {
            size
        } else {
            0
        }
    } else {
        0
    }
}

pub fn sys_write(args: &SyscallArgs) -> usize {
    let fd = get_handle(args.arg0 as u8);
    if let Some(res) = fd {
        let buf = unsafe { core::slice::from_raw_parts_mut(args.arg1 as *mut u8, args.arg2) };
        if let Ok(size) = res.write(buf) {
            size
        } else {
            0
        }
    } else {
        0
    }
}

pub fn sys_get_pid() -> u16 {
    u16::from(crate::process::current_pid())
}

pub fn exit_process(args: &SyscallArgs, regs: &mut Registers, sf: &mut InterruptStackFrame) {
    crate::process::process_exit(args.arg0 as isize, regs, sf);
}

pub fn list_process() {
    crate::process::print_process_list();
}

pub fn get_handle(fd: u8) -> Option<Resource> {
    crate::process::handle(fd)
}

pub fn sys_wait_pid(args: &SyscallArgs) -> usize {
    let pid = ProcessId(args.arg0 as u16);
    let ret = crate::process::wait_pid(pid);
    ret as usize
}

pub fn sys_kill(args: &SyscallArgs, regs: &mut Registers, sf: &mut InterruptStackFrame) {
    let pid = ProcessId(args.arg0 as u16);
    if pid == ProcessId(1) {
        warn!("sys_kill: cannot kill kernel!");
        return;
    }
    crate::process::kill(pid, regs, sf);
}
