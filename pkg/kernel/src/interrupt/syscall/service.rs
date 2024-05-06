use core::alloc::Layout;

use crate::proc::*;
use crate::utils::*;

use super::SyscallArgs;

pub fn sys_clock() -> i64 {
    clock::now()
        .and_utc()
        .timestamp_nanos_opt()
        .unwrap_or_default()
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

    let pid = crate::proc::spawn(name);

    if pid.is_err() {
        warn!("spawn_process: failed to spawn process: {}", name);
        return 0;
    }

    pid.unwrap().0 as usize
}

pub fn sys_read(args: &SyscallArgs) -> usize {
    let buf = unsafe { core::slice::from_raw_parts_mut(args.arg1 as *mut u8, args.arg2) };
    let fd = args.arg0 as u8;
    read(fd, buf) as usize
}

pub fn sys_write(args: &SyscallArgs) -> usize {
    let buf = unsafe { core::slice::from_raw_parts(args.arg1 as *const u8, args.arg2) };
    let fd = args.arg0 as u8;
    write(fd, buf) as usize
}

pub fn sys_get_pid() -> u16 {
    current_pid().0
}

pub fn exit_process(args: &SyscallArgs, context: &mut ProcessContext) {
    process_exit(args.arg0 as isize, context);
}

pub fn list_process() {
    print_process_list();
}

pub fn sys_wait_pid(args: &SyscallArgs, context: &mut ProcessContext) {
    let pid = ProcessId(args.arg0 as u16);
    wait_pid(pid, context);
}

pub fn sys_kill(args: &SyscallArgs, context: &mut ProcessContext) {
    let pid = ProcessId(args.arg0 as u16);
    if pid == ProcessId(1) {
        warn!("sys_kill: cannot kill kernel!");
        return;
    }
    kill(pid, context);
}
