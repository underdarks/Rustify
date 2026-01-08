# Rustify

Nginx를 모방하여 경량, 고성능 Rust 기반 웹 서버/리버스 프록시 프로젝트입니다. </br>
멀티프로세스 아키텍처를 활용하여 안정성과 확장성을 제공합니다.</br></br>

## 📋 프로젝트 개요

Rustify는 Nginx에서 영감을 받은 경량 고성능 웹 서버로서, Rust의 안전성과 성능을 활용합니다. </br>

Master-Worker 프로세스 모델을 사용하여 효율적인 요청 처리와 자동 재시작 기능을 제공합니다.</br></br>



### 주요 특징
- **Master-Worker 프로세스 모델**: Nginx와 유사한 아키텍처로 안정적인 요청 처리
- **멀티코어 지원**: CPU 코어 수에 따라 자동으로 워커 프로세스 생성
- **플랫폼별 I/O 멀티플렉싱**:
  - macOS: kqueue (BSD 기반 이벤트 알림)
  - Linux: epoll (Linux 고성능 이벤트 시스템)
- **리버스 프록시**: reqwest를 활용한 HTTP 요청 포워딩
- **스레드 풀**: 동적 작업 분배를 위한 ThreadPool 구현
- **Docker 지원**: 간편한 컨테이너화 및 배포

</br></br>


## 🏗️ 프로젝트 구조

```
Rustify/
├── src/                          # Rust 소스 코드
│   ├── main.rs                  # 진입점 (마스터 프로세스 시작)
│   ├── lib.rs                   # ThreadPool 구현
│   ├── config/                  # 설정 모듈
│   │   ├── mod.rs
│   │   ├── host.rs             # 호스트 주소/포트 설정
│   │   └── thread_pool.rs       # ThreadPool 설정
│   ├── server/                  # 서버 로직
│   │   ├── mod.rs
│   │   ├── master.rs            # 마스터 프로세스 (워커 모니터링/재시작)
│   │   ├── worker.rs            # 워커 프로세스 (요청 처리)
│   │   └── reverse_proxy.rs     # 리버스 프록시 구현
│   └── utils/                   # 유틸리티
│       ├── mod.rs
│       └── system.rs            # 시스템 정보 조회 (CPU 코어)
├── Cargo.toml                   # 프로젝트 의존성 정의
├── Dockerfile                   # Docker 이미지 빌드 설정
├── docker-compose.yml           # Docker Compose 오케스트레이션
├── hello.html                   # 테스트 HTML 페이지
├── 404.html                     # 404 에러 페이지
├── script.js                    # JavaScript 파일
└── target/                      # 컴파일된 바이너리 및 캐시
```

</br></br>

## 🔧 핵심 모듈 설명

### 1. **Main Process** (`src/main.rs`)

- 서버 시작 지점
- CPU 코어 개수만큼 워커 프로세스 생성 (fork 사용)
- 마스터 프로세스 실행

### 2. **Master Process** (`src/server/master.rs`)

- 워커 프로세스 모니터링
- 종료되거나 신호를 받은 워커 재시작
- 1초 주기로 상태 확인 (CPU 과사용 방지)

### 3. **Worker Process** (`src/server/worker.rs`)

- 실제 HTTP 요청 처리
- `SO_REUSEPORT` 소켓 옵션으로 여러 프로세스가 동일 포트 사용 가능
- 플랫폼별 I/O 멀티플렉싱 활용:
  - **macOS**: Kqueue 이벤트 루프
  - **Linux**: Epoll 이벤트 루프

### 4. **ThreadPool** (`src/lib.rs`)

- Arc + Mutex + MPSC 채널 기반 스레드 풀
- 여러 워커 스레드가 작업 큐에서 태스크를 가져와 처리
- `FnOnce() + Send + 'static` 클로저 지원

### 5. **Reverse Proxy** (`src/server/reverse_proxy.rs`)

- reqwest HTTP 클라이언트 기반
- 30초 타임아웃 설정
- 연결 풀 지원 (호스트당 최대 100개)

### 6. **Configuration** (`src/config/`)

- **host.rs**: 기본 수신 주소 (127.0.0.1:7879)
- **thread_pool.rs**: ThreadPool 파라미터

</br></br>
## 📦 의존성

```toml
[dependencies]
libc = "0.2"              # POSIX 시스템 호출 인터페이스
reqwest = "0.12.24"       # HTTP 클라이언트
tokio = "1"               # 비동기 런타임
```

- **libc**: fork, socket, epoll/kqueue 등 저수준 시스템 호출
- **reqwest**: HTTP 요청 포워딩 (리버스 프록시)
- **tokio**: 비동기 작업 처리

</br></br>
## 🚀 빌드 및 실행

</br></br>
### 로컬 빌드 및 실행

```bash
# Rust 설치 필수 (https://rustup.rs/)
cargo build --release
cargo run
```

</br></br>
### Docker를 이용한 실행

```bash
# Docker & Docker Compose 설치 필수
docker-compose up --build

# 백그라운드에서 실행
docker-compose up -d --build

# 컨테이너 종료
docker-compose down
```

</br></br>
## 📝 설정

**서버 주소 및 포트 변경**: [src/config/host.rs](src/config/host.rs)

```rust
pub const HOST_ADDR: &str = "127.0.0.1:7879";  // 변경 가능
```

</br></br>
## 🔍 동작 원리

### Master-Worker 구조

```
┌─────────────────────────────────────────┐
│      Master Process (PID: 1)             │
│  ├─ 자식 프로세스 모니터링               │
│  └─ 종료된 워커 자동 재시작              │
└─────────────────────────────────────────┘
         │
    fork(worker_count)
         │
┌────────┴─────────────────────────────┐
│                                      │
v                                      v
Worker 0 (SO_REUSEPORT)         Worker N (SO_REUSEPORT)
├─ Kqueue/Epoll 이벤트 루프      ├─ Kqueue/Epoll 이벤트 루프
├─ ThreadPool 작업 분배           ├─ ThreadPool 작업 분배
└─ HTTP 요청 처리                 └─ HTTP 요청 처리
```

### 요청 처리 흐름

1. 클라이언트가 `127.0.0.1:7879`로 요청 전송
2. 여러 워커 프로세스가 동일 포트에서 `accept()` 대기
3. 커널이 로드 밸런싱으로 워커 중 하나 선택
4. 선택된 워커의 Kqueue/Epoll 이벤트 루프에서 처리
5. ThreadPool을 통해 작업 분배
6. 응답 반환

## 📊 시스템 요구사항

- **OS**: macOS, Linux (Kqueue/Epoll 지원)
- **Rust**: 1.70+ (2024 에디션)
- **Docker**: (선택사항)

</br></br>
## 🛠️ 개발 팁

</br></br>
### 디버그 정보 활성화

```bash
# Rust 백트레이스 활성화
RUST_BACKTRACE=1 cargo run
```

</br></br>
### Docker 개발 환경

- `docker-compose.yml`에서 `src/` 마운트로 hot-reload 불가능하지만 빠른 재빌드 가능
- `rust_target` 볼륨으로 빌드 캐시 보존

</br></br>
## 📄 라이선스

이 프로젝트는 개발 중입니다.

</br></br>
## 🤝 기여

이 프로젝트는 학습 및 연구 목적으로 진행 중입니다.

---

**상태**: 활발한 개발 중 (develop 브랜치)
