FROM nginx:1.27-alpine

# 安装必要工具
RUN apk add --no-cache tzdata \
    && rm -rf /var/cache/apk/*

# 设置时区
ENV TZ=Asia/Shanghai
RUN ln -sf /usr/share/zoneinfo/${TZ} /etc/localtime \
    && echo ${TZ} > /etc/timezone

# 复制 Nginx 配置
COPY nginx.conf /etc/nginx/nginx.conf
COPY conf.d /etc/nginx/conf.d

# 复制前端静态资源
COPY . /usr/share/nginx/html

# 暴露端口
EXPOSE 80

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -qO- http://localhost/ || exit 1

# 启动 Nginx
CMD ["nginx", "-g", "daemon off;"]
