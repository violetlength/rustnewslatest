
# 编译阶段
FROM rust:1.92.0 as builder
WORKDIR /app
COPY . .
# 建议先安装构建依赖，防止编译时也缺库
RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo build --release

# 运行阶段
FROM debian:bullseye-slim

# 👇 新增：更新源并安装 libssl3
# 注意：debian bullseye 默认源可能只有 libssl1.1，可能需要添加 bookworm 源或者换用 bookworm 基础镜像
# 更简单的做法是直接换用 debian:bookworm-slim (Debian 12)，它原生支持 libssl.so.3

# --- 推荐修改如下 ---
FROM debian:bookworm-slim 
# ^^^ 将 bullseye 改为 bookworm，因为 bookworm 默认包含 libssl3

RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/newslatest-server /usr/local/bin/newslatest-server

EXPOSE 8080
CMD ["newslatest-server"]