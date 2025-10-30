use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

fn main() {
    //TCP 서버 소켓 생성 후 연결 대기
    let tcp_listener: TcpListener =
        TcpListener::bind("127.0.0.1:7878").expect("Tcp Listener 획득 실패");

    //incoming 메서드가 내부적으로 POSIX 표준 시스템 콜 accept()를 동기(블로킹) 방식으로 호출함
    //메인 스레드 블로킹되어 연결을 계속 기다리며 다른 일을 하지 못함
    for stream in tcp_listener.incoming() {
        let tcp_stream = stream.unwrap();
        thread::spawn(move || handle_connection(&tcp_stream));
    }
}

/*
 * TCP 연결을 처리하는 메서드
 */
fn handle_connection(mut stream: &TcpStream) {
    println!("스레드 생성!!");
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
}
