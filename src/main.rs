use std::{
    fs,
    io::{BufRead, BufReader, ErrorKind, Write},
    net::{TcpListener, TcpStream},
    thread::{self, Thread, sleep},
    time::Duration,
};

use Rustify::ThreadPool;

fn main() {
    let host: String = String::from("127.0.0.1");
    let port: String = String::from("7878");
    let addr: String = format!("{}:{}", host, port);
    let pool: ThreadPool = ThreadPool::build(100);

    //tcp 포트 바인딩
    //많은 운영 체제에는 지원 가능한 동시 연결 개수에 제한이 있습니다; 이 개수를 초과하는 새로운 연결을 시도하면 열려 있는 연결 중 일부가 닫힐 때까지 에러가 발생합니다
    let tcp_listener: TcpListener = TcpListener::bind(addr).expect("tcp Listener 획득 실패!");

    tcp_listener.set_nonblocking(true).unwrap();

    for stream in tcp_listener.incoming() {
        match stream {
            Ok(tcp_stream) => {
                pool.execute(move || {
                    handle_connection(&tcp_stream);
                });
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // 연결 없음 - 정상 상황
                // 짧게 대기 (CPU 낭비 방지)
                // std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                eprintln!("실제 에러 발생: {}", e);
                break;
            }
        }
    }

    // for stream in tcp_listener.incoming() {
    //     let stream: std::net::TcpStream = stream.unwrap();
    //     pool.execute(move || {
    //         handle_connection(&stream);
    //     });

    //     // println!("stream = {:#?}", stream);
    // }

    println!("서버 종료..");
}

fn handle_connection(mut stream: &TcpStream) {
    // println!("커넥션 핸들러 실행!");
    let buf_reader = BufReader::new(stream);
    let req_line = buf_reader.lines().next().unwrap().unwrap();

    let (status, filename) = match &req_line[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let contents = fs::read_to_string(filename).unwrap();
    let len = contents.len();

    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status,
        contents.len(),
        contents
    );

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();

    // println!("Req : {:#?}", http_request);
}
