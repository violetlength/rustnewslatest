# 编译阶段
FROM rust:1.92.0 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

# 运行阶段
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rustnewslatest-server /usr/local/bin/rustnewslatest-server
EXPOSE 8080
CMD ["rustnewslatest-server"]