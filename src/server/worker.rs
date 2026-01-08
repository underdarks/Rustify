use std::{
    io::{Error, ErrorKind, Result, Write},
    mem::zeroed,
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    os::fd::{AsRawFd, FromRawFd, RawFd},
    time::Duration,
    u32,
};

#[cfg(target_os = "macos")]
use Rustify::Kqueue;

#[cfg(target_os = "linux")]
use Rustify::Epoll;

use Rustify::ThreadPool;

use libc::{
    AF_INET, SO_REUSEADDR, SO_REUSEPORT, SOL_SOCKET, bind, c_void, close, sa_family_t, sockaddr_in,
    socklen_t,
};

#[cfg(target_os = "macos")]
use libc::kevent;

#[cfg(target_os = "linux")]
use libc::epoll_event;

/*
  SO_REUSEPORT 소켓 생성
*/
pub fn create_reusable_listener(addr: &str) -> Result<TcpListener> {
    let addr: SocketAddr = addr.parse().unwrap();

    //소켓 생성
    let sockfd: i32 = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };

    if sockfd < 0 {
        return Err(Error::last_os_error());
    }

    //SO_REUSEADDR 설정
    let optval: libc::c_int = 1;
    unsafe {
        libc::setsockopt(
            sockfd,
            SOL_SOCKET,
            SO_REUSEADDR,
            &optval as *const _ as *const c_void,
            size_of::<libc::c_int>() as socklen_t,
        );
    }

    //SO_REUSEPORT 설정(여러 프로세스가 같은 포트 사용)
    unsafe {
        libc::setsockopt(
            sockfd,
            SOL_SOCKET,
            SO_REUSEPORT,
            &optval as *const _ as *const c_void,
            size_of::<libc::c_int>() as socklen_t,
        );
    }

    //bind
    let sockaddr: sockaddr_in = {
        let mut sa: sockaddr_in = unsafe { zeroed::<sockaddr_in>() };
        sa.sin_family = AF_INET as sa_family_t;
        sa.sin_port = addr.port().to_be();

        match addr.ip() {
            IpAddr::V4(ip) => {
                sa.sin_addr.s_addr = u32::from_ne_bytes(ip.octets());
            }

            _ => panic!("Ipv4만 지원됩니다!"),
        }

        sa
    };

    let ret = unsafe {
        bind(
            sockfd,
            &sockaddr as *const libc::sockaddr_in as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        )
    };

    if ret < 0 {
        unsafe { close(sockfd) };
        return Err(Error::last_os_error());
    }

    //listen
    let ret = unsafe { libc::listen(sockfd, 15000) };

    if ret < 0 {
        unsafe { close(sockfd) };
        return Err(Error::last_os_error());
    }

    Ok(unsafe { TcpListener::from_raw_fd(sockfd) })
}

pub fn start_worker_process(id: usize, parent_pid: i32) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        start_worker_process_kqueue(id, parent_pid)
    }

    #[cfg(target_os = "linux")]
    {
        start_worker_process_epoll(id, parent_pid)
    }
}

// ============= macOS (kqueue) 워커 구현 =============
#[cfg(target_os = "macos")]
pub fn start_worker_process_kqueue(id: usize, parent_pid: i32) -> Result<()> {
    use crate::config::host::HOST_ADDR;

    let pid: i32 = unsafe { libc::getpid() };
    println!(
        "👷 Worker {} started (PID: {},  Parent PID={})",
        id + 1,
        pid,
        parent_pid
    );

    //각 Worker가 자체 리스너 생성(SO_REUSEPORT 덕분)
    let tcp_listener: TcpListener = create_reusable_listener(HOST_ADDR)?;
    tcp_listener.set_nonblocking(true)?;

    //각 Worker가 자체 kqueue 생성
    let kqueue: Kqueue = Kqueue::new()?;
    let listener_fd: i32 = tcp_listener.as_raw_fd();
    kqueue.add(listener_fd)?; //소켓 fd를 커널에 등록

    //각 worker당 스레드풀 생성
    let pool: ThreadPool = ThreadPool::build(100);

    let mut events: Vec<kevent> = vec![unsafe { std::mem::zeroed::<libc::kevent>() }; 128];
    let mut total = 0;

    loop {
        let event_count: usize = kqueue.wait(&mut events)?;

        for i in 0..event_count {
            let event: &kevent = &events[i];
            let fd = event.udata as RawFd;

            println!("fd = {}, listener_fd = {}", fd, listener_fd);

            if fd == listener_fd {
                let mut batch_count = 0;

                //하나의 워커 프로세스가 현재 이벤트 큐에 있는 연결을 모두 처리
                loop {
                    match tcp_listener.accept() {
                        //해당 소켓의 accept queue에서 가져옴
                        Ok((stream, _addr)) => {
                            batch_count += 1;
                            total += 1;

                            if total % 100 == 0 {
                                // println!(
                                //     "👷 Worker {} processed {} total (batch: {})",
                                //     id, total, batch_count
                                // );
                            }

                            pool.execute(move || {
                                // println!("pid : {} 워커 프로세스에서 http 연결 처리", pid);
                                handle_connection(&stream);
                            });
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => {
                            eprintln!("❌ Worker {} accept error: {}", id, e);
                            break;
                        }
                    }
                }

                // batch 처리가 끝났으면 로그 출력
                if batch_count > 0 {
                    println!("👷 Worker {} completed batch of {}", id + 1, batch_count);
                }
            }
        }
    }
}

// ============= Linux (epoll) 워커 구현 =============
#[cfg(target_os = "linux")]
pub fn start_worker_process_epoll(id: usize, parent_pid: i32) -> Result<()> {
    let pid: i32 = unsafe { libc::getpid() };
    println!(
        "👷 Worker {} started (PID: {},  Parent PID={})",
        id + 1,
        pid,
        parent_pid
    );

    //각 Worker가 자체 리스너 생성(SO_REUSEPORT 덕분)
    let tcp_listener: TcpListener = create_reusable_listener("HOST_ADDR")?;
    tcp_listener.set_nonblocking(true)?;

    //각 Worker가 자체 epoll 생성
    let epoll: Epoll = Epoll::new()?;
    let listener_fd: i32 = tcp_listener.as_raw_fd();
    epoll.add(listener_fd)?; //소켓 fd를 커널에 등록

    //각 worker당 스레드풀 생성
    let pool: ThreadPool = ThreadPool::build(100);

    let mut events: Vec<epoll_event> =
        vec![unsafe { std::mem::zeroed::<libc::epoll_event>() }; 128];
    let mut total = 0;

    loop {
        let event_count: usize = epoll.wait(&mut events)?;

        for i in 0..event_count {
            let event: &epoll_event = &events[i];
            let fd = event.u64 as RawFd;

            println!("fd = {}, listenr_fd = {}", fd, listener_fd);

            if fd == listener_fd {
                let mut batch_count = 0;

                //하나의 워커 프로세스가 현재 이벤트 큐에 있는 연결을 모두 처리
                loop {
                    match tcp_listener.accept() {
                        //해당 소켓의 accept queue에서 가져옴
                        Ok((stream, _addr)) => {
                            batch_count += 1;
                            total += 1;

                            if total % 100 == 0 {
                                // println!(
                                //     "👷 Worker {} processed {} total (batch: {})",
                                //     id, total, batch_count
                                // );
                            }

                            pool.execute(move || {
                                // println!("pid : {} 워커 프로세스에서 http 연결 처리", pid);
                                handle_connection(&stream);
                            });
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => {
                            eprintln!("❌ Worker {} accept error: {}", id, e);
                            break;
                        }
                    }
                }

                // batch 처리가 끝났으면 로그 출력
                if batch_count > 0 {
                    println!("👷 Worker {} completed batch of {}", id + 1, batch_count);
                }
            }
        }
    }
}

fn handle_connection(mut stream: &TcpStream) {
    // println!("커넥션 핸들러 실행!");
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();

    let response = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
