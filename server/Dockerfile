# 构建阶段
FROM rust:1.92-bookworm as builder

WORKDIR /app

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY src ./src
COPY config.toml ./config.toml
COPY icon.ico ./icon.ico
COPY config ./config
RUN cargo build --release

# 运行阶段
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/newslatest-server /usr/local/bin/
COPY --from=builder /app/config.toml ./config.toml
COPY --from=builder /app/icon.ico ./icon.ico
COPY --from=builder /app/config ./config

EXPOSE 8080

ENV RUST_LOG=info
ENV PORT=8080

CMD ["/usr/local/bin/newslatest-server"]