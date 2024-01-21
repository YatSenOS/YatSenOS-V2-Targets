use super::*;
use crate::memory::{
    self,
    allocator::{ALLOCATOR, HEAP_SIZE},
    get_frame_alloc_for_sure,
    user::{USER_ALLOCATOR, USER_HEAP_SIZE},
    PAGE_SIZE,
};
use alloc::{collections::BTreeMap, sync::Weak};
use alloc::{collections::VecDeque, format, sync::Arc};
use spin::{Mutex, RwLock};
use x86_64::VirtAddr;
use xmas_elf::ElfFile;

pub static PROCESS_MANAGER: spin::Once<ProcessManager> = spin::Once::new();

pub fn init(init: Arc<Process>) {
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
        let ready_queue = VecDeque::new();
        processor::current().put_pid(init.pid());
        processes.insert(init.pid(), init);
        Self {
            processes: RwLock::new(processes),
            ready_queue: Mutex::new(ready_queue),
        }
    }

    fn get_proc(&self, pid: ProcessId) -> Option<Arc<Process>> {
        self.processes.read().get(&pid).cloned()
    }

    pub fn current(&self) -> Arc<Process> {
        self.get_proc(current_pid()).expect("No current process")
    }

    pub fn wait_pid(&self, pid: ProcessId) -> isize {
        let proc_map = self.processes.read();
        proc_map
            .get(&pid)
            .and_then(|p| p.read().exit_code())
            .unwrap_or(-1)
    }

    pub fn still_alive(&self, pid: ProcessId) -> bool {
        self.processes
            .read()
            .get(&pid)
            .map(|p| p.read().status() != ProgramStatus::Dead)
            .unwrap_or(false)
    }

    pub fn save_current(&self, context: &mut ProcessContext) -> ProcessId {
        let current = self.current();
        let pid = current.pid();

        let mut current = current.write();
        current.tick();
        current.save(context);
        current.pause();

        pid
    }

    pub fn push_ready(&self, pid: ProcessId) {
        self.ready_queue.lock().push_back(pid);
    }

    pub fn switch_next(&self, context: &mut ProcessContext) -> ProcessId {
        let next = self.ready_queue.lock().pop_front();

        // This should never happen
        if next.is_none() {
            error!("No process in ready queue");
            return current_pid();
        }

        let pid = next.unwrap();
        let map = self.processes.read();
        let proc = map.get(&pid).expect("Process not found");

        // trace!("Next process {} #{}", proc.read().name(), pid);
        // switch to next process
        let mut proc = proc.write();
        proc.resume();
        proc.restore(context);

        processor::current().put_pid(pid);

        pid
    }

    pub fn open(&self, path: &str, mode: u8) -> Option<u8> {
        let res = match path {
            "/dev/random" => Resource::Random(fs::Random::new(
                crate::utils::clock::now().timestamp() as u64,
            )),
            path => {
                let file = crate::filesystem::try_get_file(path, fs::Mode::try_from(mode).unwrap());

                if file.is_err() {
                    return None;
                }

                Resource::File(file.unwrap())
            }
        };

        trace!("Opening {}...\n{:#?}", path, &res);

        let fd = self.current().write().open(res);

        Some(fd)
    }

    pub fn close(&self, fd: u8) -> bool {
        if fd < 3 {
            false // stdin, stdout, stderr are reserved
        } else {
            self.current().write().close(fd)
        }
    }

    pub fn spawn(
        &self,
        elf: &ElfFile,
        name: String,
        parent: Option<Weak<Process>>,
        proc_data: Option<ProcessData>,
    ) -> ProcessId {
        let kproc = self.get_proc(KERNEL_PID).unwrap();
        let page_table = kproc.read().clont_page_table();
        let p = Process::new(name, parent, page_table, proc_data);

        let mut inner = p.write();
        inner.pause();
        inner.load_elf(elf);
        inner.init_stack_frame(
            VirtAddr::new_truncate(elf.header.pt2.entry_point()),
            VirtAddr::new_truncate(STACK_INIT_TOP),
        );
        drop(inner);

        trace!("New {:#?}", &p);
        let pid = p.pid();

        // add to process map
        self.processes.write().insert(pid, p);
        // add to ready queue
        self.push_ready(pid);

        self.print_process_list();

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

    pub fn print_process_list(&self) {
        let mut output =
            String::from("  PID | PPID | Process Name |  Ticks  |   Memory  | Status\n");
        for (_, p) in self.processes.read().iter() {
            if p.read().status() != ProgramStatus::Dead {
                output += format!("{}\n", p).as_str();
            }
        }

        let heap_used = ALLOCATOR.lock().used();
        let heap_size = HEAP_SIZE;

        let user_heap_used = USER_ALLOCATOR.lock().used();
        let user_heap_size = USER_HEAP_SIZE;

        let alloc = get_frame_alloc_for_sure();
        let frames_used = alloc.frames_used();
        let frames_recycled = alloc.recycled_count();
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

        let (user_used, user_used_unit) = memory::humanized_size(user_heap_used as u64);
        let (user_size, user_size_unit) = memory::humanized_size(user_heap_size as u64);

        output += format!(
            "User   : {:>6.*} {} / {:>6.*} {} ({:>5.2}%)\n",
            2,
            user_used,
            user_used_unit,
            2,
            user_size,
            user_size_unit,
            user_heap_used as f64 / user_heap_size as f64 * 100.0
        )
        .as_str();

        // put used/total frames in MiB
        let (used_size, used_unit) =
            memory::humanized_size((frames_used - frames_recycled) as u64 * PAGE_SIZE);
        let (tot_size, tot_unit) = memory::humanized_size(frames_total as u64 * PAGE_SIZE);

        output += format!(
            "Memory : {:>6.*} {} / {:>6.*} {} ({:>5.2}%) [{} recycled]\n",
            2,
            used_size,
            used_unit,
            2,
            tot_size,
            tot_unit,
            (frames_used - frames_recycled) as f64 / frames_total as f64 * 100.0,
            frames_recycled
        )
        .as_str();

        print!("{}", output);
    }

    pub fn fork(&self) {
        let p = self.current().fork();
        let pid = p.pid();
        self.processes.write().insert(pid, p);
        self.push_ready(pid);
    }

    pub fn kill_self(&self, ret: isize) {
        self.kill(current_pid(), ret);
    }

    pub fn unblock(&self, pid: ProcessId) {
        if let Some(proc) = self.get_proc(pid) {
            proc.write().resume();
            self.push_ready(pid);
        }
    }

    pub fn block(&self, pid: ProcessId) {
        if let Some(proc) = self.get_proc(pid) {
            proc.write().pause();
        }
    }

    pub fn handle_page_fault(
        &self,
        addr: VirtAddr,
        err_code: PageFaultErrorCode,
    ) -> Result<(), ()> {
        if !err_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
            let cur_proc = self.current();
            trace!(
                "Page Fault! Checking if {:#x} is on current process's stack",
                addr
            );
            if cur_proc.read().is_on_stack(addr) {
                cur_proc.write().try_alloc_new_stack_page(addr).unwrap();
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    pub fn kill(&self, pid: ProcessId, ret: isize) {
        let proc = self.get_proc(pid);

        if proc.is_none() {
            warn!("Process #{} not found.", pid);
            return;
        }

        let proc = proc.unwrap();

        trace!("Kill {:#?}", &proc);

        proc.kill(ret);
    }
}
