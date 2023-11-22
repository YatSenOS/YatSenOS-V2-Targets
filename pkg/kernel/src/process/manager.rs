use super::*;
use crate::memory::{
    self,
    allocator::{ALLOCATOR, HEAP_SIZE},
    get_frame_alloc_for_sure, PAGE_SIZE,
};
use crate::utils::Registers;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::vec::Vec;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::paging::PhysFrame;
use x86_64::VirtAddr;

once_mutex!(pub PROCESS_MANAGER: ProcessManager);
guard_access_fn! {
    pub get_process_manager(PROCESS_MANAGER: ProcessManager)
}

pub struct ProcessManager {
    /// pid of the current running process
    cur_pid: ProcessId,
    processes: Vec<Process>,
    exit_code: BTreeMap<ProcessId, isize>,
}

impl ProcessManager {
    pub fn new(init: Process) -> Self {
        let mut processes = Vec::<Process>::new();
        let exit_code = BTreeMap::new();
        let cur_pid = init.pid();
        processes.push(init);
        Self {
            cur_pid,
            processes,
            exit_code,
        }
    }

    fn current_mut(&mut self) -> &mut Process {
        self.processes
            .iter_mut()
            .find(|x| x.pid() == self.cur_pid)
            .unwrap()
    }

    fn pid_mut(&mut self, pid: ProcessId) -> &mut Process {
        self.processes.iter_mut().find(|x| x.pid() == pid).unwrap()
    }

    pub fn current(&self) -> &Process {
        self.processes
            .iter()
            .find(|x| x.pid() == self.cur_pid)
            .unwrap()
    }

    pub fn current_pid(&self) -> ProcessId {
        self.cur_pid
    }

    pub fn save_current(&mut self, regs: &mut Registers, sf: &mut InterruptStackFrame) {
        let current = self.current_mut();
        if current.is_running() {
            current.tick();
            current.save(regs, sf);
        }
        // trace!("Paused process #{}", self.cur_pid);
    }

    fn get_next_pos(&self) -> usize {
        let cur_pos = self
            .processes
            .iter()
            .position(|x| x.pid() == self.cur_pid)
            .unwrap_or(0);

        let mut next_pos = (cur_pos + 1) % self.processes.len();

        while self.processes[next_pos].status() != ProgramStatus::Ready {
            next_pos = (next_pos + 1) % self.processes.len();
        }

        next_pos
    }

    pub fn wait_pid(&mut self, pid: ProcessId) -> isize {
        if self.exit_code.contains_key(&pid) {
            *self.exit_code.get(&pid).unwrap()
        } else {
            -1
        }
    }

    pub fn still_alive(&self, pid: ProcessId) -> bool {
        self.processes.iter().any(|x| x.pid() == pid)
    }

    pub fn switch_next(&mut self, regs: &mut Registers, sf: &mut InterruptStackFrame) {
        let pos = self.get_next_pos();
        let p = &mut self.processes[pos];

        // trace!("Next process {} #{}", p.name(), p.pid());
        if p.pid() == self.cur_pid {
            // the next process to be resumed is the same as the current one
            p.resume();
        } else {
            // switch to next process
            p.restore(regs, sf);
            self.cur_pid = p.pid();
        }
    }

    fn get_kernel_page_table(&self) -> PhysFrame {
        let proc = self.processes.first().unwrap();
        proc.page_table_addr()
    }

    pub fn spawn_kernel_thread(
        &mut self,
        entry: VirtAddr,
        name: String,
        proc_data: Option<ProcessData>,
    ) -> ProcessId {
        let mut p = Process::new(
            &mut get_frame_alloc_for_sure(),
            name,
            ProcessId(0),
            self.get_kernel_page_table(),
            proc_data,
        );
        let stack_top = p.alloc_init_stack();
        p.pause();
        p.init_stack_frame(entry, stack_top);
        info!("Spawn process: {}#{}", p.name(), p.pid());
        let pid = p.pid();
        self.processes.push(p);
        pid
    }

    pub fn print_process_list(&self) {
        let mut output =
            String::from("  PID | PPID | Process Name |  Ticks  |   Memory  | Status\n");
        for p in self.processes.iter() {
            output += format!("{}\n", p).as_str();
        }

        let heap_used = ALLOCATOR.lock().used();
        let heap_size = HEAP_SIZE;

        let alloc = get_frame_alloc_for_sure();
        let frames_used = alloc.frames_used();
        let frames_total = alloc.frames_total();

        let (sys_used, sys_used_unit) = memory::humanized_size(heap_used as u64);
        let (sys_size, sys_size_unit) = memory::humanized_size(heap_size as u64);

        output += format!(
            "Kernel : {:>6.*} {} / {:>6.*} {} ({:>5.2}%)\n",
            2,
            sys_used,
            sys_used_unit,
            2,
            sys_size,
            sys_size_unit,
            heap_used as f64 / heap_size as f64 * 100.0
        )
        .as_str();

        // put used/total frames in MiB
        let (used_size, used_unit) = memory::humanized_size(frames_used as u64 * PAGE_SIZE);
        let (tot_size, tot_unit) = memory::humanized_size(frames_total as u64 * PAGE_SIZE);

        output += format!(
            "Memory : {:>6.*} {} / {:>6.*} {} ({:>5.2}%)\n",
            2,
            used_size,
            used_unit,
            2,
            tot_size,
            tot_unit,
            frames_used as f64 / frames_total as f64 * 100.0
        )
        .as_str();

        print!("{}", output);
    }

    pub fn kill_self(&mut self, ret: isize) {
        // we dont remove the process from the list
        // as we have no interrupt frame and regs to restore
        // so we only need to ensure the process will not be scheduled again
        let pid = self.cur_pid;
        if self.exit_code.try_insert(pid, ret).is_err() {
            error!("Process #{} already exited", pid);
        }

        trace!("Process #{} exited (blocked) with code {}", pid, ret);

        let p = self.current_mut();
        p.block();
    }

    pub fn handle_page_fault(
        &mut self,
        addr: VirtAddr,
        err_code: PageFaultErrorCode,
    ) -> Result<(), ()> {
        if !err_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
            let cur_proc = self.current_mut();
            trace!(
                "Page Fault! Checking if {:#x} is on current process's stack",
                addr
            );
            if cur_proc.is_on_stack(addr) {
                cur_proc.try_alloc_new_stack_page(addr).unwrap();
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}
