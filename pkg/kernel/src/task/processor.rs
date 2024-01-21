use crate::task::ProcessId;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
use x86::cpuid::CpuId;

lazy_static! {
    static ref PROCESSORS: Vec<Mutex<Processor>> = {
        let mut processors = Vec::new();
        for _ in 0..cpu_count() {
            processors.push(Mutex::new(Processor::default()));
        }
        processors
    };
}

#[inline(always)]
fn cpu_count() -> usize {
    // TODO: support multi-core

    1
}

pub fn current<'a>() -> MutexGuard<'a, Processor> {
    let cpuid = CpuId::new()
        .get_feature_info()
        .unwrap()
        .initial_local_apic_id() as usize;

    PROCESSORS[cpuid].lock()
}

#[derive(Default)]
pub struct Processor {
    pub pid: Option<ProcessId>,
}

pub fn current_pid() -> ProcessId {
    current().get_pid().expect("No current process")
}

impl Processor {
    pub fn put_pid(&mut self, pid: ProcessId) {
        self.pid = Some(pid);
    }

    pub fn take_pid(&mut self) -> Option<ProcessId> {
        self.pid.take()
    }

    pub fn get_pid(&self) -> Option<ProcessId> {
        self.pid
    }
}
