# # 编译阶段
# FROM rust:1.92 as builder

# WORKDIR /app

# # 安装构建依赖
# RUN apt-get update && \
#     apt-get install -y pkg-config libssl-dev && \
#     rm -rf /var/lib/apt/lists/*

# # 先复制依赖文件，利用 Docker 缓存
# COPY Cargo.toml  ./
# RUN mkdir src && \
#     echo "fn main() {}" > src/main.rs && \
#     cargo build --release && \
#     rm -rf src

# # 复制实际源代码并重新编译
# COPY src ./src
# # 修改时间戳触发重新链接
# RUN touch src/main.rs && \
#     cargo build --release

# # 运行阶段 - 使用 bookworm-slim 自带 libssl3
# FROM debian:bookworm-slim

# # 安装运行时依赖（ca-certificates 用于 HTTPS 请求）
# RUN apt-get update && \
#     apt-get install -y ca-certificates && \
#     rm -rf /var/lib/apt/lists/*

# # 复制编译好的二进制文件
# COPY --from=builder /app/target/release/newslatest-server /usr/local/bin/

# # ✅ 复制配置文件到工作目录
# WORKDIR /app
# COPY config.toml ./config.toml

# COPY icon.ico ./icon.ico

# # 暴露端口
# EXPOSE 8080

# # 明确指定二进制路径，配置文件在当前目录
# CMD ["/usr/local/bin/newslatest-server"]


# 构建阶段 - 保持现状
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
RUN cargo build --release

# 运行阶段 - 修改这里！
# 不要用 debian:bookworm-slim，改用与构建相同的完整镜像
FROM rust:1.92-bookworm

WORKDIR /app

# 安装运行时依赖
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN useradd -r -u 1000 -g root appuser

# 复制编译产物
COPY --from=builder /app/target/release/newslatest-server /usr/local/bin/
COPY config.toml ./config.toml
COPY icon.ico ./icon.ico

RUN chown -R appuser:root /app
USER appuser

EXPOSE 8080

CMD ["/usr/local/bin/newslatest-server"]