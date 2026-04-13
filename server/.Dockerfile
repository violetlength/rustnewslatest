# 编译阶段
FROM rust:1.92 as builder

WORKDIR /app

# 安装构建依赖
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# 先复制依赖文件，利用 Docker 缓存
COPY Cargo.toml  ./
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# 复制实际源代码并重新编译
COPY src ./src
# 修改时间戳触发重新链接
RUN touch src/main.rs && \
    cargo build --release

# 运行阶段 - 使用 bookworm-slim 自带 libssl3
FROM debian:bookworm-slim

# 安装运行时依赖（ca-certificates 用于 HTTPS 请求）
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# 复制编译好的二进制文件
COPY --from=builder /app/target/release/newslatest-server /usr/local/bin/

# ✅ 复制配置文件到工作目录
WORKDIR /app
COPY config.toml ./config.toml

COPY icon.ico ./icon.ico

# 暴露端口
EXPOSE 8080

# 明确指定二进制路径，配置文件在当前目录
CMD ["/usr/local/bin/newslatest-server"]

