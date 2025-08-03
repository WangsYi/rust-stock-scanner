# Rust Stock Analyzer

基于 Rust 的高性能股票分析系统，支持单股和批量分析，集成 akshare 数据源。

## 功能特性

### ✅ 已完成功能
- **单股分析** - 完整的股票分析功能
- **批量分析** - 支持同时分析多只股票
- **实时进度** - WebSocket 实时更新分析进度
- **多维度评分** - 技术面、基本面、情绪面综合分析
- **mock数据** - 脱机状态下使用模拟数据
- **akshare集成** - 通过HTTP代理集成真实akshare数据
- **REST API** - 完整的RESTful API接口
- **Web界面** - 现代化的响应式Web界面

### 📊 分析维度
- **技术面**: MA、RSI、MACD、布林带等指标
- **基本面**: 财务指标、估值数据、行业信息
- **情绪面**: 新闻情绪分析、市场关注度
- **AI分析**: 基于数据的智能投资建议

## 快速开始

### 1. 安装依赖

```bash
# 安装Rust (如果尚未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 进入项目目录
cd rust-stock-analyzer

# 构建项目
cargo build --release
```

### 2. 启动akshare服务（可选）

```bash
# 启动Python akshare服务
python3 ../akshare_service.py

# 或使用一键脚本
../setup_akshare_service.sh
```

### 3. 运行应用

```bash
# 直接运行
cargo run --release

# 或使用配置文件
cargo run --release -- --config config.json
```

### 4. 访问应用

- **Web界面**: http://localhost:8080
- **批量分析**: http://localhost:8080/batch
- **API文档**: http://localhost:8080/api/health

## API接口

### 单股分析
```bash
POST /api/analyze
Content-Type: application/json

{
  "stock_code": "000001",
  "enable_ai": true
}
```

### 批量分析
```bash
POST /api/batch/analyze
Content-Type: application/json

{
  "stock_codes": ["000001", "600036", "300019"],
  "enable_ai": true
}
```

### 获取进度
```bash
GET /api/batch/status/{task_id}
```

### 获取股票数据
```bash
GET /api/stock/{code}/price?days=30
GET /api/stock/{code}/fundamental
GET /api/stock/{code}/news?days=15
GET /api/stock/{code}/name
```

## 配置

### 环境变量
```bash
# 服务器配置
HOST=0.0.0.0
PORT=8080
WORKERS=4

# akshare配置
AKSERVICE_URL=http://localhost:5000
AKSERVICE_TIMEOUT=30

# 分析配置
MAX_WORKERS=10
TIMEOUT_SECONDS=30
TECHNICAL_WEIGHT=0.5
FUNDAMENTAL_WEIGHT=0.3
SENTIMENT_WEIGHT=0.2
TECHNICAL_PERIOD=60
SENTIMENT_PERIOD=30
```

### 配置文件
创建 `config.json`:
```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "workers": 4
  },
  "analysis": {
    "max_workers": 10,
    "timeout_seconds": 30,
    "weights": {
      "technical": 0.5,
      "fundamental": 0.3,
      "sentiment": 0.2
    },
    "parameters": {
      "technical_period_days": 60,
      "sentiment_period_days": 30
    }
  },
  "akshare": {
    "proxy_url": "http://localhost:5000",
    "timeout_seconds": 30
  }
}
```

## 使用示例

### 单股分析
1. 访问 http://localhost:8080
2. 输入股票代码，如 `000001`
3. 点击"开始分析"
4. 查看详细的分析报告

### 批量分析
1. 访问 http://localhost:8080/batch
2. 输入多个股票代码，每行一个
3. 点击"开始批量分析"
4. 实时查看分析进度
5. 完成后查看所有结果

### API调用示例

```bash
# 单股分析
curl -X POST http://localhost:8080/api/analyze \
  -H "Content-Type: application/json" \
  -d '{"stock_code": "000001", "enable_ai": true}'

# 批量分析
curl -X POST http://localhost:8080/api/batch/analyze \
  -H "Content-Type: application/json" \
  -d '{"stock_codes": ["000001", "600036", "300019"], "enable_ai": true}'

# 获取进度
curl http://localhost:8080/api/batch/status/{task_id}
```

## 性能特点

- **高性能**: Rust实现，支持并发处理
- **低延迟**: 异步I/O，响应快速
- **可扩展**: 模块化设计，易于扩展
- **容错性**: 优雅降级，支持mock数据
- **实时监控**: WebSocket实时更新

## 项目结构

```
rust-stock-analyzer/
├── src/
│   ├── main.rs          # 主程序入口
│   ├── models.rs        # 数据模型定义
│   ├── data_fetcher.rs  # 数据获取模块
│   ├── analyzer.rs      # 分析引擎
│   └── handlers.rs      # Web处理模块
├── templates/
│   ├── index.html       # 单股分析页面
│   └── batch.html       # 批量分析页面
├── config.json          # 配置文件
├── Cargo.toml          # Rust依赖配置
└── README.md           # 项目说明文档
```

## 开发

### 添加新功能
1. 在 `models.rs` 中添加新数据模型
2. 在 `analyzer.rs` 中实现新分析算法
3. 在 `handlers.rs` 中添加新API端点
4. 更新 `templates/` 中的Web界面

### 运行测试
```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_analyzer

# 性能测试
cargo bench
```

### 调试
```bash
# 调试模式运行
cargo run

# 查看日志
RUST_LOG=debug cargo run

# 性能分析
cargo run --release
```

## 故障排除

### 常见问题

1. **端口占用**
   ```bash
   # 查找占用进程
   lsof -i :8080
   # 或修改端口
   PORT=8081 cargo run
   ```

2. **akshare服务未启动**
   - 使用mock数据运行
   - 或启动Python服务：python3 ../akshare_service.py

3. **依赖问题**
   ```bash
   cargo clean
   cargo build --release
   ```

4. **权限问题**
   ```bash
   chmod +x target/release/rust-stock-analyzer
   ```

## 许可证

MIT License - 详见 LICENSE 文件