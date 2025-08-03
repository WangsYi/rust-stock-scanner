# Docker部署指南

这个Docker配置为股票分析系统提供了完整的容器化部署方案，包含所有必要的组件和服务。

## 服务架构

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   stock-analyzer│    │   postgres      │    │  akshare-service│
│   (Rust App)    │    │   (Database)    │    │  (Python API)   │
│   Port: 8080    │    │   Port: 5432    │    │   Port: 5000    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│      nginx      │    │      redis      │    │   volumes       │
│   (Proxy)       │    │   (Cache)       │    │   (Storage)     │
│   Port: 80      │    │   Port: 6379    │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## 快速开始

### 1. 克隆项目
```bash
git clone <repository-url>
cd rust-stock-analyzer
```

### 2. 配置环境变量
```bash
# 复制环境变量模板
cp .env.example .env

# 编辑配置文件
nano .env
```

### 3. 构建和启动所有服务
```bash
# 启动所有服务（包含nginx代理）
docker-compose --profile proxy up -d

# 或者只启动核心服务
docker-compose up -d
```

### 4. 验证服务状态
```bash
# 查看所有容器状态
docker-compose ps

# 查看服务日志
docker-compose logs -f stock-analyzer

# 检查健康状态
curl http://localhost:8080/api/health
```

## 环境变量配置

### 必需配置
```bash
# 数据库配置
DATABASE_URL=postgres://stockuser:stockpass@postgres:5432/stock_analyzer
DATABASE_MAX_CONNECTIONS=10
DATABASE_ENABLE_MIGRATIONS=true

# AI服务配置
AI_PROVIDER=openai
AI_API_KEY=your-openai-api-key
AI_MODEL=gpt-4
AI_ENABLED=true
AI_TIMEOUT=60

# AKShare服务配置
AKSERVICE_URL=http://akshare-service:5000
AKSERVICE_TIMEOUT=30
```

### 可选配置
```bash
# 服务器配置
HOST=0.0.0.0
PORT=8080
RUST_LOG=info
MAX_WORKERS=10

# 分析参数
TECHNICAL_WEIGHT=0.5
FUNDAMENTAL_WEIGHT=0.3
SENTIMENT_WEIGHT=0.2
TECHNICAL_PERIOD=60
SENTIMENT_PERIOD=30

# 缓存配置
CACHE_ENABLED=true
CACHE_PRICE_TTL=300
CACHE_FUNDAMENTAL_TTL=3600
CACHE_NEWS_TTL=1800
```

## 服务说明

### stock-analyzer (主应用)
- **功能**: Rust股票分析引擎
- **端口**: 8080
- **依赖**: PostgreSQL, AKShare服务
- **健康检查**: `/api/health`

### postgres (数据库)
- **功能**: PostgreSQL数据存储
- **端口**: 5432
- **数据持久化**: 是
- **初始化**: 自动创建表和索引

### akshare-service (数据源)
- **功能**: Python AKShare数据服务
- **端口**: 5000
- **缓存**: Redis缓存支持
- **健康检查**: `/health`

### redis (缓存)
- **功能**: 内存缓存服务
- **端口**: 6379
- **持久化**: AOF模式
- **用途**: 缓存股票数据和会话

### nginx (反向代理)
- **功能**: HTTP反向代理和负载均衡
- **端口**: 80, 443
- **特性**: SSL终止、压缩、安全头
- **配置文件**: `nginx/nginx.conf`

## 数据持久化

### 卷挂载
```yaml
volumes:
  postgres_data:      # PostgreSQL数据
  redis_data:         # Redis数据
  akshare_cache:      # AKShare缓存
  stock_analyzer_data: # 应用数据
```

### 数据备份
```bash
# 备份数据库
docker exec stock-postgres pg_dump -U stockuser stock_analyzer > backup.sql

# 恢复数据库
docker exec -i stock-postgres psql -U stockuser stock_analyzer < backup.sql
```

## 监控和日志

### 查看日志
```bash
# 查看所有服务日志
docker-compose logs -f

# 查看特定服务日志
docker-compose logs -f stock-analyzer
docker-compose logs -f postgres

# 查看最近100行日志
docker-compose logs --tail=100 stock-analyzer
```

### 健康检查
```bash
# 检查主应用
curl http://localhost:8080/api/health

# 检查AKShare服务
curl http://localhost:5000/health

# 检查数据库
docker exec stock-postgres pg_isready -U stockuser -d stock_analyzer
```

### 性能监控
```bash
# 查看容器资源使用
docker stats

# 查看容器详细信息
docker inspect stock-analyzer
```

## 扩展和配置

### 水平扩展
```yaml
# 扩展多个实例
docker-compose up -d --scale stock-analyzer=3

# 使用负载均衡
upstream stock_analyzer {
    server stock-analyzer-1:8080;
    server stock-analyzer-2:8080;
    server stock-analyzer-3:8080;
}
```

### SSL配置
```bash
# 创建SSL证书目录
mkdir -p nginx/ssl

# 放置证书文件
nginx/ssl/cert.pem
nginx/ssl/key.pem

# 修改nginx配置启用HTTPS
```

### 生产环境配置
```yaml
# 生产环境优化
environment:
  - RUST_LOG=warn
  - MAX_WORKERS=20
  - DATABASE_MAX_CONNECTIONS=20
  - CACHE_ENABLED=true

# 资源限制
deploy:
  resources:
    limits:
      cpus: '2.0'
      memory: 2G
    reservations:
      cpus: '1.0'
      memory: 1G
```

## 故障排除

### 常见问题

1. **服务启动失败**
   ```bash
   # 检查容器日志
   docker-compose logs stock-analyzer
   
   # 检查端口占用
   netstat -tulpn | grep :8080
   ```

2. **数据库连接失败**
   ```bash
   # 检查数据库状态
   docker-compose logs postgres
   
   # 测试数据库连接
   docker exec stock-postgres psql -U stockuser -d stock_analyzer
   ```

3. **AKShare服务无响应**
   ```bash
   # 检查AKShare服务
   docker-compose logs akshare-service
   
   # 测试AKShare连接
   curl http://localhost:5000/health
   ```

### 重启服务
```bash
# 重启单个服务
docker-compose restart stock-analyzer

# 重启所有服务
docker-compose restart

# 重新构建并启动
docker-compose up -d --build
```

### 清理和重置
```bash
# 停止所有服务
docker-compose down

# 删除所有数据（谨慎使用）
docker-compose down -v

# 清理未使用的镜像
docker image prune
```

## 开发环境

### 本地开发
```bash
# 启动开发环境
docker-compose -f docker-compose.dev.yml up -d

# 使用热重载
docker-compose -f docker-compose.dev.yml up --build
```

### 调试模式
```bash
# 启用调试日志
docker-compose run -e RUST_LOG=debug stock-analyzer

# 进入容器调试
docker exec -it stock-analyzer bash
```

## 安全考虑

### 网络安全
- 使用内部网络隔离容器
- 配置防火墙规则
- 启用HTTPS加密

### 数据安全
- 定期备份数据库
- 使用强密码
- 加密敏感配置

### 访问控制
- 限制容器权限
- 使用非root用户运行
- 配置访问日志

## 更新和维护

### 更新应用
```bash
# 拉取最新代码
git pull origin main

# 重新构建和启动
docker-compose up -d --build
```

### 更新依赖
```bash
# 更新Rust依赖
docker-compose run --rm stock-analyzer cargo update

# 更新Python依赖
docker-compose run --rm akshare-service pip install -r requirements.txt --upgrade
```

### 定期维护
```bash
# 清理日志
docker-compose logs --tail=0 > /dev/null

# 清理缓存
docker exec stock-redis redis-cli FLUSHDB

# 数据库维护
docker exec stock-postgres vacuumdb -U stockuser -d stock_analyzer --analyze
```