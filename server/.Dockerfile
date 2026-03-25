
# # 编译阶段
# FROM rust:1.92.0 as builder
# WORKDIR /app
# COPY . .
# # 建议先安装构建依赖，防止编译时也缺库
# RUN apt-get update && apt-get install -y pkg-config libssl-dev
# RUN cargo build --release

# # 运行阶段
# FROM debian:bullseye-slim

# # 👇 新增：更新源并安装 libssl3
# # 注意：debian bullseye 默认源可能只有 libssl1.1，可能需要添加 bookworm 源或者换用 bookworm 基础镜像
# # 更简单的做法是直接换用 debian:bookworm-slim (Debian 12)，它原生支持 libssl.so.3

# # --- 推荐修改如下 ---
# FROM debian:bookworm-slim 
# # ^^^ 将 bullseye 改为 bookworm，因为 bookworm 默认包含 libssl3

# RUN apt-get update && \
#     apt-get install -y ca-certificates libssl3 && \
#     rm -rf /var/lib/apt/lists/*

# COPY --from=builder /app/target/release/newslatest-server /usr/local/bin/newslatest-server

# # ✅ 关键：复制配置文件
# COPY config.toml /app/config.toml

# EXPOSE 8080
# CMD ["newslatest-server"]


# 编译阶段
FROM rust:1.92 as builder

WORKDIR /app

# 安装构建依赖
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# 先复制依赖文件，利用 Docker 缓存
COPY Cargo.toml Cargo.lock ./
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

# 暴露端口
EXPOSE 8080

# 明确指定二进制路径，配置文件在当前目录
CMD ["/usr/local/bin/newslatest-server"]