# ====================
# 阶段 1: 构建阶段 (使用 rust:alpine 实现静态编译)
# ====================
FROM rust:1.80-alpine3.20 AS builder

# 安装 musl 工具链和编译依赖
RUN apk add --no-cache \
    musl-dev \
    perl \
    make \
    gcc \
    g++ \
    pkgconfig \
    openssl-dev \
    && rm -rf /var/cache/apk/*

# 设置工作目录
WORKDIR /app

# 创建空的目标目录用于缓存
RUN mkdir -p /app/target

# 预先复制 Cargo.toml 和 Cargo.lock 以利用 Docker 缓存
COPY Cargo.toml Cargo.lock ./

# 创建一个虚拟的 main.rs 来预编译依赖
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# 复制完整的源代码和配置
COPY src ./src
COPY config ./config

# 构建静态链接的 release 版本
ENV RUSTFLAGS="-C target-feature=+crt-static -C target-cpu=generic"
RUN cargo build --release \
    && cp /app/target/release/sinan-backend /app/sinan-backend

# 验证二进制文件是静态链接
RUN ldd /app/sinan-backend 2>&1 || true

# ====================
# 阶段 2: 运行阶段 (最小化镜像)
# ====================
FROM alpine:3.20

# 安装必要的运行时依赖
RUN apk add --no-cache \
    ca-certificates \
    tzdata \
    && rm -rf /var/cache/apk/*

# 设置时区
ENV TZ=Asia/Shanghai
RUN ln -sf /usr/share/zoneinfo/${TZ} /etc/localtime \
    && echo ${TZ} > /etc/timezone

# 创建非 root 用户
RUN addgroup -S sinan \
    && adduser -S sinan -G sinan

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件和配置
COPY --from=builder /app/sinan-backend ./sinan-backend
COPY --from=builder /app/config ./config

# 设置执行权限
RUN chmod +x /app/sinan-backend \
    && chown -R sinan:sinan /app

# 切换到非 root 用户
USER sinan

# 暴露端口
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:8080/health || exit 1

# 启动命令
CMD ["./sinan-backend"]
