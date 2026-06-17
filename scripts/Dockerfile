FROM python:3.12-slim

# 设置工作目录
WORKDIR /app

# 安装必要的系统包
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        tzdata \
        curl \
    && rm -rf /var/lib/apt/lists/*

# 设置时区
ENV TZ=Asia/Shanghai
RUN ln -sf /usr/share/zoneinfo/${TZ} /etc/localtime \
    && echo ${TZ} > /etc/timezone

# 复制依赖文件
COPY requirements.txt .

# 安装 Python 依赖
RUN pip install --no-cache-dir --upgrade pip \
    && pip install --no-cache-dir -r requirements.txt

# 复制模拟器代码
COPY sensor_simulator_enhanced.py .

# 设置执行权限
RUN chmod +x sensor_simulator_enhanced.py

# 创建非 root 用户
RUN groupadd -r sinan \
    && useradd -r -g sinan sinan \
    && chown -R sinan:sinan /app

USER sinan

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD python -c "import sys; sys.exit(0)" || exit 1

# 默认命令（可通过环境变量覆盖）
CMD ["python", "sensor_simulator_enhanced.py", \
     "--device-id", "sim-sinan-001", \
     "--material", "magnetite_natural", \
     "--period", "han_dynasty", \
     "--interval", "1000"]
