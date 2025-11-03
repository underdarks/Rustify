use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};

/**
 ThreadPool 상세 분석
1. mpsc::channel: Multiple Producer Single Consumer
   - 여러 곳에서 보낼 수 있지만(Multiple Producer)
   - 받는 곳은 하나(Single Consumer)

2. Arc (Atomic Reference Counting)
   - 여러 스레드가 receiver를 공유해야 함
   - `Arc::clone()`으로 참조 카운트만 증가 (실제 복사 X)

3. Mutex (Mutual Exclusion)
   - 한 번에 한 워커만 `recv()`할 수 있게 보장
   - 여러 워커가 동시에 메시지를 빼가지 못하도록 잠금

**시각화:**
```
ThreadPool
├─ sender (송신자)
└─ receiver (Arc<Mutex<수신자>>)
   ├─ Worker 0 (Arc 복사본)
   ├─ Worker 1 (Arc 복사본)
   ├─ Worker 2 (Arc 복사본)
   └─ Worker 3 (Arc 복사본)
 */

/*
 - FnOnce(): 한 번만 호출되는 클로저, 매개변수 없음
 - Send: 다른 스레드로 안전하게 이동 가능
 - 'static: 프로그램 전체 생명주기 동안 유효 (스레드가 언제 실행될지 모르니까)
*/
type Task = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,                              //스레드 고유 ID
    thread: Option<thread::JoinHandle<()>>, //스레드 핸들
}

pub struct ThreadPool {
    workers: Vec<Worker>,               //실제 작업 스레드들
    sender: Option<mpsc::Sender<Task>>, //작업을 워커에케 전달하는 채널의 송신자
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Task>>>) -> Worker {
        let thread_id: thread::JoinHandle<()> = thread::spawn(move || {
            loop {
                /*
                 뮤텍스 획득
                 - Mutex<T>는 한 번에 하나의 Worker 스레드만 작업을 요청하도록 하는 것을 보장합니다.(다른 워커 대기)
                 - recv(): 메시지 올 떄 까지 블로킹, 메시지 받으면 자동으로 lock 해제(스코프 벗어나므로, drop 호출)후 다른 워커가 락 획득
                */
                let msg = receiver.lock().unwrap().recv();

                match msg {
                    Ok(task) => {
                        println!("Worker {id} 테스크 실행중...");
                        task();
                    }
                    Err(_) => {
                        println!("Worker {id} disconnected; shutting down.");
                        break;
                    }
                }
            }
        });
        Worker {
            id,
            thread: Some(thread_id),
        }
    }
}

/// 1.ThreadPool은 채널을 생성하고 송신자를 대기시킵니다.
/// 2.각 Worker는 수신자를 보관합니다.
/// 3.채널을 통해 보내려는 클로저를 가진 새로운 구조체 Job을 만듭니다.
/// 4.execute 메서드는 송신자를 통하여 실행하려는 작업을 보냅니다.
/// 5.Worker는 자신의 스레드에서 수신자에 대한 반복을 수행하고 자신이 받은 작업의 클로저를 실행합니다.

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn build(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut threads: Vec<Worker> = Vec::with_capacity(size);

        for id in 0..size {
            threads.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers: threads,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static, //클로저(FnOnce, 한 번만 호출되는 클로저, 매개변수 없음)를 매개변수로 받는다.
    {
        //클로저를 힙에 할당 (Box) → 크기를 컴파일 타임에 몰라도 됨
        let job: Box<F> = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap(); //채널을 통해 스레드 풀에 있는 스레드(워커)에게 작업(Task)을 넘긴다.
    }
}

/*
          Drop 트레이트 매커니즘

1. `sender.take()`를 먼저 drop
   - `Option<Sender>`에서 `Sender`를 꺼내서 drop
   - 채널이 닫힘 → 워커들이 `recv()`에서 `Err` 받음
   - 워커들이 loop를 빠져나옴

2. `worker.thread.take()`
   - `Option<JoinHandle>`에서 `JoinHandle`을 꺼냄
   - 소유권을 가져와서 `join()` 호출 가능
   - `take()` 없이 `join()`하면 소유권 이동 에러 발생!

3. `thread.join()`
   - 각 워커 스레드가 종료될 때까지 대기
   - 모든 작업이 완료되도록 보장

## 실행 흐름 예시
1. 클라이언트 요청 → TcpStream
2. pool.execute(클로저) → Box<클로저>를 채널로 send
3. Worker 0이 lock 획득 → recv() → 작업 받음
4. Worker 0이 handle_connection 실행
5. Worker 1은 대기 중 (lock 획득 못함)
6. Worker 0 작업 완료 → lock 자동 해제
7. Worker 1이 lock 획득 → 다음 작업 받음
 */

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            drop(self.sender.take()); //sender를 해제하므로, 더 이상 아무 메시지도 보내지 않게끔한다.
            println!("Shut down worker {}", worker.id);

            //cannot move out of `worker.thread` which is behind a mutable reference 오류 발생
            //move occurs because `worker.thread` has type `JoinHandle<()>`, which does not implement the `Copy` trait
            //각 worker의 가변 대여만 있고 join이 인수의 소유권을 가져가기 때문에 join을 호출할 수 없음을 알려줍니다
            //take():Option<T>에서 T를 꺼내고 Option을 None으로 바꾼다. 소유권을 가져올 수 있게 해주는 트릭!
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
