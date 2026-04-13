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

# --- 构建阶段 ---
# 使用一个版本明确且稳定的 Rust 镜像
FROM rust:1.82-bookworm as builder

# 设置工作目录
WORKDIR /app

# 1. 安装构建依赖
# libssl-dev 用于编译依赖 openssl-sys 的 crate
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# 2. 缓存 Rust 依赖
# 复制 Cargo 文件
COPY Cargo.toml ./
# 创建一个虚拟的 main.rs 来触发依赖下载和编译
RUN mkdir src && echo "fn main() {}" > src/main.rs
# 设置 CARGO_HOME 环境变量，确保依赖被缓存在 /app/cargo-cache
ENV CARGO_HOME=/app/cargo-cache
RUN cargo fetch
RUN cargo build --release
# 删除虚拟源码，保留编译好的依赖
RUN rm -rf src

# 3. 复制实际源代码并编译
# 复制整个 src 目录，确保所有文件都被包含
COPY src ./src
# 再次构建，这次会使用真实的源代码
# cargo build 会自动利用上一步缓存的依赖
RUN cargo build --release

# --- 运行阶段 ---
# 关键修改：使用与构建阶段相同的基础镜像版本！
# 这确保了运行时的 GLIBC 版本与编译时一致，彻底避免兼容性问题。
FROM rust:1.82-bookworm as runner

# 1. 安装运行时依赖
RUN apt-get update && \
    apt-get install -y ca-certificates openssl && \
    rm -rf /var/lib/apt/lists/*

# 2. 创建非 root 用户以增强安全性
RUN useradd -r -u 1000 -g root appuser

# 3. 设置工作目录
WORKDIR /app

# 4. 复制编译产物和配置文件
COPY --from=builder /app/target/release/newslatest-server /usr/local/bin/
COPY config.toml ./config.toml
COPY icon.ico ./icon.ico

# 5. 修改文件所有者，让 appuser 可以读取
RUN chown -R appuser:root /app

# 6. 切换到非 root 用户
USER appuser

# 7. 暴露端口
EXPOSE 8080

# 8. 启动应用
CMD ["/usr/local/bin/newslatest-server"]