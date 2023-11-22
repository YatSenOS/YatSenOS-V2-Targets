use lib::*;

pub fn exec(name: &str) {
    let start = sys_time();

    let pid = sys_spawn(name.to_ascii_lowercase().as_str());

    if pid == 0 {
        errln!("failed to spawn process: {}", name);
        return;
    }

    let ret = sys_wait_pid(pid);
    let time = sys_time() - start;

    println!(
        "[+] process exited with code {} @ {}s",
        ret,
        time.num_seconds()
    );
}

pub fn kill(pid: u16) {
    sys_kill(pid);
}
