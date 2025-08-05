#!/bin/bash

# 设置智谱GLM API密钥
# 请将你的API密钥替换下面的 "your-api-key-here"

export GLM_API_KEY="your-api-key-here"

# 或者直接修改config.json文件
# cat > config.json << 'EOF'
# {
#   "server": {
#     "host": "0.0.0.0",
#     "port": 8080,
#     "workers": 4
#   },
#   "analysis": {
#     "max_workers": 10,
#     "timeout_seconds": 30,
#     "weights": {
#       "technical": 0.5,
#       "fundamental": 0.3,
#       "sentiment": 0.2
#     },
#     "parameters": {
#       "technical_period_days": 60,
#       "sentiment_period_days": 30
#     }
#   },
#   "akshare": {
#     "proxy_url": "http://localhost:5000",
#     "timeout_seconds": 30
#   },
#   "ai": {
#     "provider": "glm",
#     "api_key": "your-api-key-here",
#     "model": "glm-4",
#     "enabled": true,
#     "timeout_seconds": 120
#   },
#   "database": {
#     "url": "stock_analyzer.db",
#     "max_connections": 5,
#     "enable_migrations": true
#   },
#   "auth": {
#     "enabled": false,
#     "secret_key": "your-secret-key-change-this",
#     "session_timeout": 86400,
#     "bcrypt_cost": 12
#   },
#   "cache": {
#     "enabled": true,
#     "price_data_ttl": 300,
#     "fundamental_data_ttl": 3600,
#     "news_data_ttl": 1800,
#     "stock_name_ttl": 86400,
#     "max_entries": 1000,
#     "cleanup_interval": 60,
#     "enable_stats": true
#   }
# }
# EOF

echo "请将上面的 'your-api-key-here' 替换为你的实际API密钥"
echo "然后重新启动应用"