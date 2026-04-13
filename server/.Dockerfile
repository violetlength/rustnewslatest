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
# 1. 明确指定使用 1.92 版本，并基于 bookworm (Debian 12)
# 如果 1.92 默认是 trixie，也可以写成 rust:1.92-triage
FROM rust:1.92-bookworm as builder

WORKDIR /app

# 2. 安装构建依赖
# 注意：Bookworm 默认源可能需要更新，libssl-dev 用于编译 openssl-sys
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# 3. 优化依赖缓存 (利用 Docker Layer Cache)
# 先复制 Cargo 配置文件
COPY Cargo.toml ./
# 创建虚拟 main.rs 骗过 cargo，使其只下载和编译依赖
RUN mkdir src && echo "fn main() {}" > src/main.rs
# 这一步会下载依赖并编译，如果依赖没变，下次构建会直接命中缓存
RUN cargo build --release
# 删除虚拟源码
RUN rm -rf src

# 4. 复制真实源码并编译
COPY src ./src
# 编译真实项目
RUN cargo build --release

# --- 运行阶段 ---
# 5. 关键点：运行环境必须与构建环境的 GLIBC 版本兼容！
# 既然必须用 Rust 1.92 (基于 Bookworm)，这里也必须用 Bookworm
# 不要使用 bullseye 或 alpine，否则会继续报 GLIBC 错误
FROM debian:bookworm-slim

WORKDIR /app

# 6. 安装运行时必要的库
# ca-certificates: 用于 HTTPS 请求
# libssl3: 运行时需要的 SSL 库 (Bookworm 中是 libssl3)
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# 7. 创建非 root 用户 (安全最佳实践)
# 虽然 slim 镜像可能没有 useradd，但通常都有。如果没有，需先 apt install adduser
RUN useradd -r -u 1000 -g root appuser

# 8. 复制编译产物
COPY --from=builder /app/target/release/newslatest-server /usr/local/bin/
COPY config.toml ./config.toml
COPY icon.ico ./icon.ico

# 9. 授权并切换用户
RUN chown -R appuser:root /app
USER appuser

# EXPOSE 8080

# CMD ["/usr/local/bin/newslatest-server"]

# 在运行阶段添加
ENV PORT=8080  # 默认值，会被 Railway 覆盖

# 修改 CMD，通过环境变量传递
CMD ["sh", "-c", "/usr/local/bin/newslatest-server --port $PORT"]