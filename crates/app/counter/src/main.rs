#![no_std]
#![no_main]

use lib::*;

extern crate lib;

const THREAD_COUNT: usize = 8;
static mut COUNTER: isize = 0;

static MUTEX: Semaphore = Semaphore::new(0xdeadbeef);

fn main() -> isize {
    MUTEX.init(1);
    let mut pids = [0u16; THREAD_COUNT];

    for item in pids.iter_mut() {
        let pid = sys_fork();
        if pid == 0 {
            do_counter_inc();
            sys_exit(0);
        } else {
            *item = pid; // only parent knows child's pid
        }
    }

    let cpid = sys_get_pid();
    println!("process #{} holds threads: {:?}", cpid, &pids);
    sys_stat();

    for pid in pids {
        println!("#{} Waiting for #{}...", cpid, pid);
        sys_wait_pid(pid);
    }

    println!("COUNTER result: {}", unsafe { COUNTER });

    0
}

fn do_counter_inc() {
    for _ in 0..100 {
        // FIXME: protect the critical section
        MUTEX.wait();
        inc_counter();
        MUTEX.signal();
    }
}

/// Increment the counter
///
/// this function simulate a critical section by delay
/// DO NOT MODIFY THIS FUNCTION
fn inc_counter() {
    unsafe {
        delay();
        let mut val = COUNTER;
        delay();
        val += 1;
        delay();
        COUNTER = val;
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
fn delay() {
    for _ in 0..0x100 {
        core::hint::spin_loop();
    }
}

entry!(main);
