# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Commands

### Building and Running
```bash
# Build the project (optimized release build)
cargo build --release

# Run in development mode
cargo run

# Run with specific configuration
cargo run --release -- --config config.json

# Use the build script (recommended)
./build.sh

# Run on custom port
PORT=8081 cargo run
```

### Development and Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Check code formatting
cargo fmt --check

# Format code
cargo fmt

# Lint code
cargo clippy

# Generate documentation
cargo doc --no-deps
```

### Development Server
```bash
# Start development server with debug logs
RUST_LOG=debug cargo run

# Start with custom host/port
HOST=0.0.0.0 PORT=8081 cargo run
```

## Architecture Overview

This is a high-performance Rust-based stock analysis system with the following key architectural components:

### Core Modules
- **`main.rs`**: Application entry point and HTTP server setup using Actix Web
- **`models.rs`**: Complete data model definitions for stock analysis, configuration, and API responses
- **`handlers.rs`**: HTTP request handlers, API endpoints, and WebSocket management
- **`analyzer.rs`**: Core analysis engine that orchestrates technical, fundamental, and sentiment analysis
- **`data_fetcher.rs`**: Data abstraction layer supporting both real (akshare) and mock data sources
- **`ai_service.rs`**: Multi-provider AI integration with support for OpenAI, Claude, Baidu, Tencent, GLM, Qwen, Kimi, and Ollama
- **`auth.rs`**: JWT-based authentication and user management
- **`database.rs`**: PostgreSQL database integration for persistent storage of analysis results and configurations

### Key Design Patterns
- **Async/Await**: Full async processing using Tokio for high concurrency
- **Dependency Injection**: AppState pattern for shared services across handlers
- **Trait Abstraction**: DataFetcher trait for multiple data source implementations
- **Configuration Management**: Layered configuration (file → environment → defaults)
- **Error Handling**: Unified ApiResponse wrapper for consistent error responses
- **Persistent Storage**: PostgreSQL integration for analysis history and configuration management

### Data Flow
1. **HTTP Request** → **Handler** → **Analyzer** → **DataFetcher** → **AI Service** → **Response**
2. **WebSocket Updates** for real-time progress tracking during batch analysis
3. **Modular Scoring**: Technical (50%), Fundamental (30%), Sentiment (20%) with configurable weights
4. **Persistent Storage**: Analysis results automatically saved to PostgreSQL database with AI provider/model metadata

## Multi-Provider AI Integration

The system supports 9 AI providers with unified interfaces:
- **OpenAI**: GPT-4o, GPT-4o-mini, GPT-3.5-turbo
- **Claude**: Claude 3.5 Sonnet, Claude 3 Haiku
- **Baidu**: ERNIE-Bot-4, ERNIE-Bot-turbo
- **Tencent**: Hunyuan-pro, Hunyuan-standard
- **Zhipu GLM**: GLM-4, GLM-4-air, GLM-3-turbo
- **Alibaba Qwen**: Qwen-turbo, Qwen-plus, Qwen-max
- **Moonshot Kimi**: Moonshot-v1-8k, Moonshot-v1-32k, Moonshot-v1-128k
- **Ollama**: Llama3.1, Qwen2.5, Mistral-nemo
- **Custom**: User-defined models with manual input support

### AI Configuration
AI models are configurable via web interface at `/config` with custom model input support. System prompts are optimized for professional financial analysis with detailed context including 25 financial indicators, news sentiment analysis, and multi-dimensional evaluation.

## Frontend Architecture

### Template Structure
- **`templates/index.html`**: Main single-stock analysis interface with tabbed results/history
- **`templates/batch.html`**: Batch analysis interface with real-time progress tracking
- **`templates/config.html`**: AI provider and system configuration interface

### Frontend Features
- **Modern UI**: Glassmorphism design with gradient backgrounds and smooth animations
- **Tabbed Interface**: Switch between analysis results and history
- **Local Storage**: Analysis history persists across sessions
- **Streaming Support**: Real-time progress updates during analysis
- **Responsive Design**: Mobile-friendly layout with adaptive components

### JavaScript Architecture
- **Modular Functions**: Separated concerns for analysis, history, UI management
- **Local Storage**: Analysis history with preview generation
- **Streaming API**: Server-sent events for real-time updates
- **Error Handling**: Graceful fallbacks for streaming failures

## Persistent Storage

### Database Integration
The system now includes SQLite and PostgreSQL integration for persistent storage of:
- **Analysis Results**: All stock analyses are automatically saved with complete metadata
- **Configuration Management**: AI provider configurations and system settings can be saved and retrieved
- **History Tracking**: Complete analysis history with search and filtering capabilities

### Database Support
- **SQLite (Default)**: Embedded database, no external dependencies required
- **PostgreSQL**: Server-based database for production deployments
- **Automatic Detection**: System automatically detects database type from URL format

### Database Schema
- **saved_analyses**: Stores complete analysis reports with technical, fundamental, and sentiment data
- **saved_configurations**: Stores AI provider configurations and system settings
- **Automatic Indexing**: Optimized queries for stock codes, dates, and configuration types

### Setup and Configuration
```bash
# Set up database (auto-detects type)
./setup_database.sh

# SQLite (default, embedded)
export DATABASE_URL=sqlite:stock_analyzer.db
export DATABASE_MAX_CONNECTIONS=5
export DATABASE_ENABLE_MIGRATIONS=true

# PostgreSQL (server-based)
export DATABASE_URL=postgres://user@localhost:5432/stock_analyzer
export DATABASE_MAX_CONNECTIONS=5
export DATABASE_ENABLE_MIGRATIONS=true
```

## Configuration System

### Configuration Sources (Priority Order)
1. **Environment Variables**: Runtime overrides
2. **`config.json`**: File-based configuration
3. **Default Values**: Built-in fallbacks

### Key Configuration Sections
- **Server**: Host, port, worker threads
- **Analysis**: Worker limits, timeouts, scoring weights, analysis periods
- **Akshare**: Proxy URL and timeout for external data source
- **AI**: Provider selection, API keys, model configuration
- **Auth**: JWT settings, user management (optional)

### Environment Variables
```bash
# Server Configuration
HOST=0.0.0.0
PORT=8080
WORKERS=4

# Data Source
AKSERVICE_URL=http://localhost:5000
AKSERVICE_TIMEOUT=30

# Analysis Parameters
MAX_WORKERS=10
TIMEOUT_SECONDS=30
TECHNICAL_WEIGHT=0.5
FUNDAMENTAL_WEIGHT=0.3
SENTIMENT_WEIGHT=0.2
TECHNICAL_PERIOD=60
SENTIMENT_PERIOD=30

# AI Configuration
AI_PROVIDER=openai
AI_API_KEY=your-api-key
AI_MODEL=gpt-4o
AI_ENABLED=true
AI_TIMEOUT=30

# Authentication (Optional)
AUTH_ENABLED=false
AUTH_SECRET_KEY=your-secret-key
SESSION_TIMEOUT=86400

# Database Configuration
DATABASE_URL=sqlite:stock_analyzer.db
DATABASE_MAX_CONNECTIONS=5
DATABASE_ENABLE_MIGRATIONS=true
```

## API Structure

### Core Analysis Endpoints
- **`POST /api/analyze`**: Single stock analysis with AI integration
- **`POST /api/batch/analyze`**: Batch analysis with progress tracking
- **`GET /api/batch/status/{task_id}`**: Batch analysis status monitoring

### Data Access Endpoints
- **`GET /api/stock/{code}/price`**: Historical price data
- **`GET /api/stock/{code}/fundamental`**: Fundamental analysis data
- **`GET /api/stock/{code}/news`**: News and sentiment data
- **`GET /api/stock/{code}/name`**: Stock name lookup

### Configuration Endpoints
- **`GET /api/config/ai`**: Current AI configuration
- **`POST /api/config/ai`**: Update AI configuration
- **`GET /api/config/ai/providers`**: Available AI providers
- **`POST /api/config/ai/test`**: Test AI connection

### History and Persistent Storage Endpoints
- **`GET /api/history`**: Query analysis history with filtering support
- **`GET /api/history/{id}`**: Get specific analysis by ID
- **`POST /api/configurations`**: Save configuration (AI provider, system settings)
- **`GET /api/configurations`**: List saved configurations
- **`POST /api/configurations/{id}/activate`**: Activate saved configuration
- **`DELETE /api/configurations/{id}`**: Delete saved configuration

### WebSocket Support
- **`/ws`**: Real-time progress updates for batch analysis

## Development Guidelines

### Adding New Analysis Features
1. **Update Models**: Add new data structures in `models.rs`
2. **Extend Analyzer**: Implement analysis logic in `analyzer.rs`
3. **Add API Endpoints**: Create handlers in `handlers.rs`
4. **Update Frontend**: Modify templates and JavaScript as needed
5. **Update Configuration**: Add new configuration parameters

### Adding New AI Providers
1. **Implement Provider**: Add provider logic in `ai_service.rs`
2. **Update Provider List**: Add to `get_ai_providers_info()` function
3. **Update Configuration**: Add provider-specific configuration options
4. **Test Integration**: Verify provider functionality with test cases

### Frontend Development
- **Use Modern JavaScript**: ES6+ features with proper error handling
- **Maintain State**: Use local storage for persistent data
- **Responsive Design**: Ensure mobile compatibility
- **User Experience**: Add loading states, error handling, and user feedback

### Performance Considerations
- **Async Processing**: Use Tokio for concurrent operations
- **Connection Pooling**: Reuse HTTP connections for external APIs
- **Caching**: Implement caching for frequently accessed data
- **Resource Management**: Proper cleanup of resources and connections

### Database Usage Examples
```bash
# Query analysis history for a specific stock
curl "http://localhost:8080/api/history?stock_code=000001&limit=10"

# Get analysis history for a date range
curl "http://localhost:8080/api/history?start_date=2024-01-01T00:00:00Z&end_date=2024-12-31T23:59:59Z"

# Save AI configuration
curl -X POST "http://localhost:8080/api/configurations?type=ai&name=my-gpt4-config" \
  -H "Content-Type: application/json" \
  -d '{"provider": "openai", "model": "gpt-4", "api_key": "sk-..."}'

# List saved configurations
curl "http://localhost:8080/api/configurations?type=ai"

# Activate a saved configuration
curl -X POST "http://localhost:8080/api/configurations/{uuid}/activate"

# Run with SQLite (default)
cargo run

# Run with PostgreSQL
DATABASE_URL=postgres://user@localhost:5432/stock_analyzer cargo run
```

## Data Sources

### Primary Data Source (Akshare)
The system integrates with a Python akshare service running on `localhost:5000` by default. This service provides:
- Real-time stock price data
- Financial indicators and fundamental data
- News and sentiment analysis
- Market metadata

### Mock Data Fallback
When the akshare service is unavailable, the system automatically falls back to mock data:
- Realistic stock price simulations
- Generated financial indicators
- Sample news and sentiment data
- Maintains full functionality for development/testing

## Testing Strategy

### Unit Tests
```bash
# Test individual modules
cargo test analyzer::tests::test_scoring

# Test data fetcher implementations
cargo test data_fetcher::tests::test_mock_data

# Test AI service integration
cargo test ai_service::tests::test_provider_config
```

### Integration Tests
```bash
# Test complete analysis workflow
cargo test integration::tests::test_full_analysis

# Test batch analysis functionality
cargo test integration::tests::test_batch_processing
```

### Performance Testing
```bash
# Run benchmarks
cargo bench

# Test concurrent analysis
cargo test performance::tests::test_concurrent_requests
```

## Deployment

### Production Build
```bash
cargo build --release
```

### Configuration Management
- Use `config.json` for production settings
- Set environment variables for sensitive data (API keys)
- Configure appropriate worker counts for server capacity

### Service Management
- The service can be run as a systemd service
- Log rotation should be configured for production use
- Health checks available at `/api/health`

## Troubleshooting

### Common Issues
- **Port Conflicts**: Check if port 8080 is available, use `PORT=8081` to override
- **Akshare Service**: Ensure Python akshare service is running on `localhost:5000`
- **Database Connection**: Verify database file exists (SQLite) or PostgreSQL is running and `DATABASE_URL` is correctly configured
- **API Keys**: Verify AI provider API keys are correctly configured
- **Memory Usage**: Monitor memory usage during batch analysis operations

### Debug Logging
```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Log specific modules
RUST_LOG=analyzer=debug,ai_service=debug cargo run
```

### Performance Profiling
```bash
# Build with profiling
cargo build --release

# Use performance monitoring tools
perf record --call-graph dwarf ./target/release/rust-stock-analyzer
```