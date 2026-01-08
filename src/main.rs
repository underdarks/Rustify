use std::{
    io::{Error, Result, Write},
    net::{TcpListener, TcpStream},
};

mod config;
mod server;
mod utils;

use libc::{getpid, getppid};

use crate::{
    config::host::HOST_ADDR,
    server::{
        master::start_master_process,
        worker::{create_reusable_listener, start_worker_process},
    },
    utils::system::get_cpu_count,
};

fn main() -> Result<()> {
    let worker_count: usize = get_cpu_count();

    //서버 소켓 생성
    // let server_tcp_socket: TcpListener = create_reusable_listener(HOST_ADDR).unwrap();

    //워커 프로세스에게 fd, kqueue를 넘겨서 공유해줘야함
    for id in 0..worker_count {
        // println!("📍 Before fork (id={}), current PID: {}", id, unsafe {
        //     libc::getpid()
        // });

        //마스터 프로세스 기반 복제
        match unsafe { { libc::fork() } } {
            //자식 프로세스(Worker)
            0 => {
                // drop(server_tcp_socket); //부모 리스너 닫기

                let my_pid = unsafe { getpid() };
                let parent_pid = unsafe { getppid() };

                //각 worker가 자체 리스너 생성
                start_worker_process(id, parent_pid)?; //워커들 무한루프로 계속 실행(블로킹)
                std::process::exit(0); //종료 시그널이나 에러 발생시를 위한 자식 프로세스 종료
            }
            pid if pid > 0 => {
                //부모 프로세스(Master)
                // println!("✅ Spawned worker {} (PID: {})", id, pid);
            }

            _ => {
                eprintln!("❌ Fork failed");
                return Err(Error::last_os_error());
            }
        }

        //     getpid()
        // });
    }

    start_master_process();

    Ok(())
}
