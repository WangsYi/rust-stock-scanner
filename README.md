# Rust 股票分析器

基于 Rust 的高性能股票分析系统，集成多种 AI 服务提供商，支持单股和批量分析，提供专业的投资建议。

## 🚀 核心特性

### 🎯 智能分析
- **多维度评分**: 技术面 (50%)、基本面 (30%)、情绪面 (20%)
- **AI 智能分析**: 集成 9 大 AI 服务提供商
- **实时数据处理**: WebSocket 实时更新分析进度
- **专业报告**: 生成详细的股票分析报告

### 🤖 AI 服务集成
- **OpenAI**: GPT-4o、GPT-4o-mini、GPT-3.5-turbo
- **Claude**: Claude 3.5 Sonnet、Claude 3 Haiku
- **百度文心**: ERNIE-Bot-4、ERNIE-Bot-turbo
- **腾讯混元**: Hunyuan-pro、Hunyuan-standard
- **智谱 GLM**: GLM-4、GLM-4-air、GLM-3-turbo
- **阿里通义**: Qwen-turbo、Qwen-plus、Qwen-max
- **月之暗面**: Moonshot-v1-8k、Moonshot-v1-32k、Moonshot-v1-128k
- **Ollama**: Llama3.1、Qwen2.5、Mistral-nemo
- **自定义模型**: 支持用户自定义 AI 模型

### 📊 数据分析维度
- **技术分析**: 移动平均线、RSI、MACD、布林带等 25+ 技术指标
- **基本面分析**: 财务指标、估值分析、行业对比
- **情绪分析**: 新闻情感分析、市场关注度、舆情监控
- **智能建议**: 基于多维度数据的专业投资建议

### 💾 数据持久化
- **SQLite**: 默认嵌入式数据库，无需额外配置
- **PostgreSQL**: 生产级数据库支持
- **自动迁移**: 数据库表结构自动创建和更新
- **历史记录**: 完整的分析历史和配置管理

## 🛠️ 快速开始

### 环境要求
- Rust 1.70+
- PostgreSQL (可选，用于生产环境)
- Python 3.8+ (可选，用于 AKShare 服务)

### 1. 克隆项目
```bash
git clone https://github.com/WangsYi/rust-stock-scanner.git
cd rust-stock-analyzer
```

### 2. 构建项目
```bash
# 开发模式构建
cargo build

# 生产模式构建
cargo build --release
```

### 3. 配置环境
```bash
# 复制环境变量模板
cp .env.example .env

# 编辑配置文件
nano .env
```

### 4. 启动应用
```bash
# 开发模式运行
cargo run

# 生产模式运行
cargo run --release

# 使用自定义配置
cargo run --release -- --config config.json
```

### 5. 访问应用
- **主页**: http://localhost:8080
- **批量分析**: http://localhost:8080/batch
- **配置管理**: http://localhost:8080/config
- **健康检查**: http://localhost:8080/api/health

## 🔧 配置说明

### 环境变量配置
```bash
# 服务器配置
HOST=0.0.0.0
PORT=8080
WORKERS=4
RUST_LOG=info

# 数据库配置
DATABASE_URL=sqlite:stock_analyzer.db
DATABASE_MAX_CONNECTIONS=5
DATABASE_ENABLE_MIGRATIONS=true

# AI 服务配置
AI_PROVIDER=openai
AI_API_KEY=your-api-key-here
AI_MODEL=gpt-4o
AI_ENABLED=true
AI_TIMEOUT=30

# 分析参数配置
MAX_WORKERS=10
TIMEOUT_SECONDS=30
TECHNICAL_WEIGHT=0.5
FUNDAMENTAL_WEIGHT=0.3
SENTIMENT_WEIGHT=0.2
TECHNICAL_PERIOD=60
SENTIMENT_PERIOD=30

# AKShare 服务配置
AKSERVICE_URL=http://localhost:5000
AKSERVICE_TIMEOUT=30

# 认证配置 (可选)
AUTH_ENABLED=false
AUTH_SECRET_KEY=your-secret-key
SESSION_TIMEOUT=86400
```

### 配置文件示例
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
  "ai": {
    "provider": "openai",
    "model": "gpt-4o",
    "enabled": true,
    "timeout": 30
  },
  "database": {
    "url": "sqlite:stock_analyzer.db",
    "max_connections": 5,
    "enable_migrations": true
  }
}
```

## 📡 API 接口

### 分析接口
```bash
# 单股分析
POST /api/analyze
Content-Type: application/json

{
  "stock_code": "000001",
  "enable_ai": true,
  "ai_provider": "openai",
  "ai_model": "gpt-4o"
}

# 批量分析
POST /api/batch/analyze
Content-Type: application/json

{
  "stock_codes": ["000001", "600036", "300019"],
  "enable_ai": true
}

# 获取批量分析进度
GET /api/batch/status/{task_id}
```

### 数据接口
```bash
# 获取股票价格数据
GET /api/stock/{code}/price?days=30

# 获取基本面数据
GET /api/stock/{code}/fundamental

# 获取新闻情绪数据
GET /api/stock/{code}/news?days=15

# 获取股票名称
GET /api/stock/{code}/name
```

### 配置管理接口
```bash
# 获取 AI 配置
GET /api/config/ai

# 更新 AI 配置
POST /api/config/ai

# 获取可用的 AI 提供商
GET /api/config/ai/providers

# 测试 AI 连接
POST /api/config/ai/test
```

### 历史记录接口
```bash
# 获取分析历史
GET /api/history?stock_code=000001&limit=10

# 获取特定分析结果
GET /api/history/{id}

# 保存配置
POST /api/configurations

# 获取保存的配置
GET /api/configurations
```

## 🐳 Docker 部署

### 快速启动
```bash
# 启动所有服务
docker-compose --profile proxy up -d

# 仅启动核心服务
docker-compose up -d

# 开发环境
docker-compose -f docker-compose.dev.yml up -d
```

### 环境配置
```bash
# 复制环境变量
cp .env.example .env

# 编辑配置
nano .env
```

## 🎮 使用指南

### 单股分析
1. 访问 http://localhost:8080
2. 输入股票代码（如：000001）
3. 选择 AI 提供商和模型
4. 点击"开始分析"
5. 查看详细的分析报告

### 批量分析
1. 访问 http://localhost:8080/batch
2. 输入多个股票代码（每行一个）
3. 配置分析参数
4. 点击"开始批量分析"
5. 实时查看分析进度
6. 完成后查看所有结果

### 配置管理
1. 访问 http://localhost:8080/config
2. 配置 AI 服务提供商
3. 设置 API 密钥
4. 选择默认模型
5. 保存配置

## 🏗️ 项目架构

### 核心模块
```
src/
├── main.rs              # 应用入口和 HTTP 服务器
├── models.rs            # 数据模型定义
├── handlers.rs          # HTTP 请求处理
├── analyzer.rs          # 核心分析引擎
├── data_fetcher.rs      # 数据获取抽象层
├── ai_service.rs        # AI 服务集成
├── auth.rs              # 认证和用户管理
└── database.rs          # 数据库集成
```

### 前端界面
```
templates/
├── index.html           # 单股分析界面
├── batch.html           # 批量分析界面
└── config.html          # 配置管理界面
```

### 配置文件
```
├── .env.example         # 环境变量模板
├── config.json          # 应用配置
├── Cargo.toml           # Rust 依赖配置
└── CLAUDE.md            # 开发指南
```

## 🔧 开发指南

### 添加新的 AI 提供商
1. 在 `ai_service.rs` 中实现新的提供者逻辑
2. 更新 `get_ai_providers_info()` 函数
3. 添加相应的配置选项
4. 测试集成功能

### 添加新的分析指标
1. 在 `models.rs` 中定义新的数据结构
2. 在 `analyzer.rs` 中实现分析逻辑
3. 更新评分算法
4. 添加相应的测试

### 数据库操作
```bash
# 运行数据库迁移
./setup_database.sh

# 使用 SQLite（默认）
cargo run

# 使用 PostgreSQL
DATABASE_URL=postgres://user@localhost:5432/stock_analyzer cargo run
```

## 🧪 测试

### 运行测试
```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test analyzer::tests::test_scoring

# 运行集成测试
cargo test integration::tests::test_full_analysis
```

### 性能测试
```bash
# 运行基准测试
cargo bench

# 性能分析
cargo build --release
perf record --call-graph dwarf ./target/release/rust-stock-analyzer
```

## 🔍 故障排除

### 常见问题

1. **端口占用**
   ```bash
   # 查找占用进程
   lsof -i :8080
   # 或修改端口
   PORT=8081 cargo run
   ```

2. **AI 服务连接失败**
   - 检查 API 密钥是否正确
   - 验证网络连接
   - 查看 AI 服务状态

3. **数据库连接失败**
   ```bash
   # SQLite: 检查文件权限
   chmod 644 stock_analyzer.db
   
   # PostgreSQL: 检查服务状态
   systemctl status postgresql
   ```

4. **AKShare 服务未启动**
   - 使用 mock 数据运行
   - 或启动 Python 服务：`python3 akshare_service.py`

### 调试模式
```bash
# 启用调试日志
RUST_LOG=debug cargo run

# 记录特定模块日志
RUST_LOG=analyzer=debug,ai_service=debug cargo run
```

## 📊 性能优化

### 系统优化
- **并发处理**: 使用 Tokio 异步运行时
- **连接池**: 复用 HTTP 连接和数据库连接
- **缓存策略**: 实现智能缓存机制
- **资源管理**: 合理分配系统资源

### 配置优化
```bash
# 生产环境配置
RUST_LOG=warn
MAX_WORKERS=20
DATABASE_MAX_CONNECTIONS=20
CACHE_ENABLED=true
```

## 🚀 部署指南

### 生产环境部署
1. **构建优化版本**
   ```bash
   cargo build --release
   ```

2. **配置生产环境**
   ```bash
   # 设置环境变量
   export RUST_LOG=warn
   export DATABASE_URL=postgres://user@localhost:5432/stock_analyzer
   export MAX_WORKERS=20
   ```

3. **使用 Docker 部署**
   ```bash
   docker-compose --profile proxy up -d
   ```

4. **服务管理**
   ```bash
   # 使用 systemd 管理服务
   sudo systemctl enable rust-stock-analyzer
   sudo systemctl start rust-stock-analyzer
   ```

## 📝 更新日志

### v2.0.0 (当前版本)
- ✅ 集成 9 大 AI 服务提供商
- ✅ 添加数据库持久化支持
- ✅ 实现配置管理系统
- ✅ 优化性能和稳定性
- ✅ 完善 Docker 部署方案

### v1.0.0
- ✅ 基础股票分析功能
- ✅ 批量分析支持
- ✅ WebSocket 实时更新
- ✅ 响应式 Web 界面

## 🤝 贡献指南

1. Fork 本项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [AKShare](https://github.com/akfamily/akshare) - 开源金融数据接口库
- [Actix Web](https://actix.rs/) - 高性能 Rust Web 框架
- [Tokio](https://tokio.rs/) - Rust 异步运行时
- 所有 AI 服务提供商的支持

## 📞 联系方式

- 项目地址: [https://github.com/WangsYi/rust-stock-scanner](https://github.com/WangsYi/rust-stock-scanner)
- 问题反馈: [GitHub Issues](https://github.com/WangsYi/rust-stock-scanner/issues)
- 邮箱: [your-email@example.com](mailto:your-email@example.com)