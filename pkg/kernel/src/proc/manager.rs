use super::*;
use crate::{
    humanized_size,
    memory::{
        PAGE_SIZE,
        allocator::{ALLOCATOR, HEAP_SIZE},
        get_frame_alloc_for_sure,
    },
};
use alloc::collections::BTreeMap;
use alloc::{collections::VecDeque, format, sync::Arc};
use spin::{Mutex, RwLock};
use x86_64::VirtAddr;

pub static PROCESS_MANAGER: spin::Once<ProcessManager> = spin::Once::new();

pub fn init(init: Arc<Process>) {
    init.write().resume();
    processor::set_pid(init.pid());
    PROCESS_MANAGER.call_once(|| ProcessManager::new(init));
}

pub fn get_process_manager() -> &'static ProcessManager {
    PROCESS_MANAGER
        .get()
        .expect("Process Manager has not been initialized")
}

pub struct ProcessManager {
    processes: RwLock<BTreeMap<ProcessId, Arc<Process>>>,
    ready_queue: Mutex<VecDeque<ProcessId>>,
}

impl ProcessManager {
    pub fn new(init: Arc<Process>) -> Self {
        let mut processes = BTreeMap::new();
        let pid = init.pid();

        trace!("Init {:#?}", init);

        processes.insert(pid, init);
        Self {
            processes: RwLock::new(processes),
            ready_queue: Mutex::new(VecDeque::new()),
        }
    }

    #[inline]
    pub fn push_ready(&self, pid: ProcessId) {
        self.ready_queue.lock().push_back(pid);
    }

    #[inline]
    fn add_proc(&self, pid: ProcessId, proc: Arc<Process>) {
        self.processes.write().insert(pid, proc);
    }

    #[inline]
    fn get_proc(&self, pid: &ProcessId) -> Option<Arc<Process>> {
        self.processes.read().get(pid).cloned()
    }

    pub fn current(&self) -> Arc<Process> {
        self.get_proc(&processor::current_pid())
            .expect("No current process")
    }

    pub(super) fn get_exit_code(&self, pid: ProcessId) -> Option<isize> {
        self.get_proc(&pid).and_then(|p| p.read().exit_code())
    }

    pub fn save_current(&self, context: &ProcessContext) {
        let current = self.current();
        let pid = current.pid();

        let mut inner = current.write();
        inner.tick();
        inner.save(context);
        let status = inner.status();
        drop(inner);

        // debug!("Save process {} #{}", current.name(), pid);

        if status != ProgramStatus::Dead {
            self.push_ready(pid);
        } else {
            debug!("Process {:#?} #{} is dead", current, pid);
        }
    }

    pub fn switch_next(&self, context: &mut ProcessContext) -> ProcessId {
        let mut pid = processor::current_pid();

        while let Some(next) = self.ready_queue.lock().pop_front() {
            let map = self.processes.read();
            let proc = map.get(&next).expect("Process not found");

            if !proc.read().is_ready() {
                debug!("Process #{} is {:?}", next, proc.read().status());
                continue;
            }

            // debug!("Switch process {} #{}", proc.read().name(), next);

            if pid != next {
                proc.write().restore(context);
                processor::set_pid(next);
                pid = next;
            }

            break;
        }

        pid
    }

    pub fn spawn_kernel_thread(
        &self,
        entry: VirtAddr,
        name: String,
        proc_data: Option<ProcessData>,
    ) -> ProcessId {
        let kproc = self.get_proc(&KERNEL_PID).unwrap();
        let page_table = kproc.read().clone_page_table();
        let proc_vm = Some(ProcessVm::new(page_table));
        let proc = Process::new(name, Some(Arc::downgrade(&kproc)), proc_vm, proc_data);

        let stack_top = proc.alloc_init_stack();
        let mut inner = proc.write();
        inner.pause();
        inner.init_stack_frame(entry, stack_top);

        let pid = proc.pid();
        info!("Spawn process: {}#{}", inner.name(), pid);
        drop(inner);

        self.add_proc(pid, proc);
        self.push_ready(pid);

        pid
    }

    pub fn kill_current(&self, ret: isize) {
        self.kill(processor::current_pid(), ret);
    }

    pub fn handle_page_fault(&self, addr: VirtAddr, err_code: PageFaultErrorCode) -> bool {
        if !err_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
            let cur_proc = self.current();
            trace!(
                "Page Fault! Checking if {:#x} is on current process's stack",
                addr
            );

            if cur_proc.pid() == KERNEL_PID {
                info!("Page Fault on Kernel at {:#x}", addr);
            }

            let mut inner = cur_proc.write();
            inner.handle_page_fault(addr)
        } else {
            false
        }
    }

    pub fn kill(&self, pid: ProcessId, ret: isize) {
        let proc = self.get_proc(&pid);

        if proc.is_none() {
            warn!("Process #{} not found.", pid);
            return;
        }

        let proc = proc.unwrap();

        if proc.read().status() == ProgramStatus::Dead {
            warn!("Process #{} is already dead.", pid);
            return;
        }

        trace!("Kill {:#?}", &proc);

        proc.kill(ret);
    }

    pub fn print_process_list(&self) {
        let mut output = String::from("  PID | PPID | Process Name |  Ticks  | Status\n");

        self.processes
            .read()
            .values()
            .filter(|p| p.read().status() != ProgramStatus::Dead)
            .for_each(|p| output += format!("{}\n", p).as_str());

        let heap_used = ALLOCATOR.lock().used();
        let heap_size = HEAP_SIZE;

        output += &format_usage("Kernel", heap_used, heap_size);

        let alloc = get_frame_alloc_for_sure();
        let frames_used = alloc.frames_used();
        let frames_total = alloc.frames_total();

        output += &format_usage(
            "Kernel",
            frames_used * PAGE_SIZE as usize,
            frames_total * PAGE_SIZE as usize,
        );

        output += format!("Queue  : {:?}\n", self.ready_queue.lock()).as_str();

        output += &processor::print_processors();

        print!("{}", output);
    }
}

fn format_usage(name: &str, used: usize, total: usize) -> String {
    let (used_float, used_unit) = humanized_size(used as u64);
    let (total_float, total_unit) = humanized_size(total as u64);

    format!(
        "{:<6} : {:>6.*} {:>3} / {:>6.*} {:>3} ({:>5.2}%)\n",
        name,
        2,
        used_float,
        used_unit,
        2,
        total_float,
        total_unit,
        used as f32 / total as f32 * 100.0
    )
}
