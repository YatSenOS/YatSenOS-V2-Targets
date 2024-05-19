use alloc::collections::BTreeSet;

use super::*;
use crate::{
    memory::{
        allocator::{ALLOCATOR, HEAP_SIZE},
        get_frame_alloc_for_sure,
        user::{USER_ALLOCATOR, USER_HEAP_SIZE},
        PAGE_SIZE,
    },
    utils::humanized_size,
};
use alloc::{collections::BTreeMap, collections::VecDeque, format, sync::Weak};
use spin::{Mutex, RwLock};

pub static PROCESS_MANAGER: spin::Once<ProcessManager> = spin::Once::new();

pub fn init(init: Arc<Process>, app_list: boot::AppListRef) {
    processor::set_pid(init.pid());
    PROCESS_MANAGER.call_once(|| ProcessManager::new(init, app_list));
}

pub fn get_process_manager() -> &'static ProcessManager {
    PROCESS_MANAGER
        .get()
        .expect("Process Manager has not been initialized")
}

pub struct ProcessManager {
    processes: RwLock<BTreeMap<ProcessId, Arc<Process>>>,
    ready_queue: Mutex<VecDeque<ProcessId>>,
    app_list: boot::AppListRef,
    wait_queue: Mutex<BTreeMap<ProcessId, BTreeSet<ProcessId>>>,
}

impl ProcessManager {
    pub fn new(init: Arc<Process>, app_list: boot::AppListRef) -> Self {
        let mut processes = BTreeMap::new();
        let pid = init.pid();
        processes.insert(pid, init);
        Self {
            processes: RwLock::new(processes),
            app_list,
            ready_queue: Mutex::new(VecDeque::new()),
            wait_queue: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn app_list(&self) -> boot::AppListRef {
        self.app_list
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

    pub fn wait_pid(&self, pid: ProcessId) -> Option<isize> {
        if let Some(ret) = self.get_ret(pid) {
            return Some(ret);
        };

        // push the current process to the wait queue
        let mut wait_queue = self.wait_queue.lock();
        let entry = wait_queue.entry(pid).or_default();
        entry.insert(processor::current_pid());

        None
    }

    pub(super) fn get_ret(&self, pid: ProcessId) -> Option<isize> {
        self.get_proc(&pid).and_then(|p| p.read().exit_code())
    }

    pub fn save_current(&self, context: &ProcessContext) -> ProcessId {
        let current = self.current();
        let pid = current.pid();

        let mut current = current.write();
        current.tick();
        current.save(context);

        // debug!("Save process {} #{}", current.name(), pid);

        pid
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

            if pid != next {
                proc.write().restore(context);
                processor::set_pid(next);
                pid = next;
            }

            break;
        }

        pid
    }

    #[inline]
    pub fn read(&self, fd: u8, buf: &mut [u8]) -> isize {
        self.current().read().read(fd, buf)
    }

    #[inline]
    pub fn write(&self, fd: u8, buf: &[u8]) -> isize {
        self.current().read().write(fd, buf)
    }

    pub fn spawn(
        &self,
        elf: &ElfFile,
        name: String,
        parent: Option<Weak<Process>>,
        proc_data: Option<ProcessData>,
    ) -> ProcessId {
        let kproc = self.get_proc(&KERNEL_PID).unwrap();
        let page_table = kproc.read().clone_page_table();
        let proc_vm = Some(ProcessVm::new(page_table));
        let proc = Process::new(name, parent, proc_vm, proc_data);

        let mut inner = proc.write();
        inner.pause();
        inner.load_elf(elf);
        inner.init_stack_frame(
            VirtAddr::new_truncate(elf.header.pt2.entry_point()),
            VirtAddr::new_truncate(super::stack::STACK_INIT_TOP),
        );
        drop(inner);

        trace!("New {:#?}", &proc);

        let pid = proc.pid();
        self.add_proc(pid, proc);
        self.push_ready(pid);

        pid
    }

    // DEPRECATED: do not spawn kernel thread
    // pub fn spawn_kernel_thread(
    //     &self,
    //     entry: VirtAddr,
    //     stack_top: VirtAddr,
    //     name: String,
    //     parent: ProcessId,
    //     proc_data: Option<ProcessData>,
    // ) -> ProcessId {
    //     let kproc = self.get_proc(KERNEL_PID).unwrap();
    //     let page_table = kproc.read().clont_page_table();
    //     let mut p = Process::new(
    //         &mut crate::memory::get_frame_alloc_for_sure(),
    //         name,
    //         parent,
    //         page_table,
    //         proc_data,
    //     );
    //     p.pause();
    //     p.init_stack_frame(entry, stack_top);
    //     info!("Spawn process: {}#{}", p.name(), p.pid());
    //     let pid = p.pid();
    //     self.processes.push(p);
    //     pid
    // }

    pub fn wake_up(&self, pid: ProcessId) {
        if let Some(proc) = self.get_proc(&pid) {
            proc.write().pause();
            self.push_ready(pid);
        }
    }

    pub fn kill_self(&self, ret: isize) {
        self.kill(processor::current_pid(), ret);
    }

    pub fn handle_page_fault(&self, addr: VirtAddr, err_code: PageFaultErrorCode) -> bool {
        if !err_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
            let cur_proc = self.current();
            trace!(
                "Page Fault! Checking if {:#x} is on current process's stack",
                addr
            );

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

        trace!("Kill {:#?}", &proc);

        proc.kill(ret);

        if let Some(pids) = self.wait_queue.lock().remove(&pid) {
            for p in pids {
                self.wake_up(p);
            }
        }
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

        let user_heap_used = USER_ALLOCATOR.lock().used();
        let user_heap_size = USER_HEAP_SIZE;

        output += &format_usage("User", user_heap_used, user_heap_size);

        let alloc = get_frame_alloc_for_sure();
        let frames_used = alloc.frames_used();
        let frames_total = alloc.frames_total();

        let used = frames_used * PAGE_SIZE as usize;
        let total = frames_total * PAGE_SIZE as usize;

        output += &format_usage("Memory", used, total);

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
