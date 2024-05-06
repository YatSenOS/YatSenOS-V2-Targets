#![no_std]
#![no_main]

use lib::*;

extern crate lib;

const QUEUE_COUNT: usize = 16;
static mut COUNT: usize = 0;

static IS_NOT_FULL: Semaphore = Semaphore::new(0x1000);
static IS_NOT_EMPTY: Semaphore = Semaphore::new(0x2000);
static MUTEX: Semaphore = Semaphore::new(0x6666);

fn main() -> usize {
    IS_NOT_EMPTY.init(0);
    IS_NOT_FULL.init(QUEUE_COUNT);
    MUTEX.init(1);

    let mut pids = [0u16; QUEUE_COUNT];

    for i in 0..QUEUE_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            if i % 2 == 0 {
                producer(i);
            } else {
                consumer(i);
            }
        } else {
            pids[i] = pid;
        }
    }

    let cpid = sys_get_pid();

    println!("#{} holds threads: {:?}", cpid, &pids);

    sys_stat();

    for i in 0..QUEUE_COUNT {
        println!("#{} Waiting for #{}...", cpid, pids[i]);
        sys_wait_pid(pids[i]);
    }

    MUTEX.free();
    IS_NOT_EMPTY.free();
    IS_NOT_FULL.free();

    0
}

fn producer(id: usize) -> ! {
    let pid = sys_get_pid();
    println!("New producer #{}({})", id, pid);
    for _ in 0..10 {
        IS_NOT_FULL.wait();
        MUTEX.wait();
        unsafe {
            COUNT += 1;
        }
        println!("Produced by #{:<3}({:<3}) count={}", id, pid, unsafe {
            COUNT
        });
        MUTEX.signal();
        IS_NOT_EMPTY.signal();
    }
    sys_exit(0);
}

fn consumer(id: usize) -> ! {
    let pid = sys_get_pid();
    println!("New consumer #{}({})", id, pid);
    for _ in 0..10 {
        IS_NOT_EMPTY.wait();
        MUTEX.wait();
        unsafe {
            COUNT -= 1;
        }
        println!("Consumed by #{:<3}({:<3}) count={}", id, pid, unsafe {
            COUNT
        });
        MUTEX.signal();
        IS_NOT_FULL.signal();
    }
    sys_exit(0);
}

entry!(main);
