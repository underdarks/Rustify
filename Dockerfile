# 2024 에디션을 지원하는 최신 안정 Rust 이미지 사용.
# 특정 버전이 필요하면 `latest` 대신 해당 태그로 교체하세요.
#Base OS: Debian (리눅스 배포판)
#포함 사항: Rust 툴체인 + Cargo + 기본 리눅스 도구
#아키텍처: 보통 linux/amd64 (x86-64 리눅스)
FROM rust:latest 

WORKDIR /usr/src/rustify

# 빌드 종속성을 이미지 빌드 시점에 한 번만 설치하여 캐시됨.
RUN apt-get update && \
    apt-get install -y build-essential pkg-config libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# 소스를 복사하고 릴리스 바이너리를 미리 빌드하여 컨테이너 시작 속도 향상.
COPY . .
RUN cargo build --release

#빌드된 바이너리 실행
CMD ["/usr/src/rustify/target/release/Rustify"]
