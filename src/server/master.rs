use std::{thread, time::Duration};

use libc::*;

pub fn start_master_process() {
    println!("마스터 프로세스 모니터링 시작");

    loop {
        let mut status: c_int = 0;
        let pid = unsafe { libc::waitpid(-1, &mut status, WNOHANG) };

        if pid > 0 {
            if unsafe { WIFEXITED(status) } {
                let exit_code = unsafe { WEXITSTATUS(status) };
                eprintln!("⚠️ Worker {} exited with status {}", pid, exit_code);
            } else if unsafe { WIFSIGNALED(status) } {
                let signal = unsafe { WTERMSIG(status) };
                eprintln!("⚠️ Worker {} killed by signal {}", pid, signal);
            }

            //워커 재시작
        }

        //CPU 과사용 방지
        thread::sleep(Duration::from_secs(1));
    }
}
