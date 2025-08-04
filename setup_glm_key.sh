#!/bin/bash

# 设置GLM API密钥脚本
echo "=== GLM API 密钥设置脚本 ==="
echo ""

# 检查是否提供了API密钥参数
if [ -z "$1" ]; then
    echo "使用方法: $0 <your-glm-api-key>"
    echo "例如: $0 your-api-key-here"
    echo ""
    echo "或者你可以设置环境变量:"
    echo "export GLM_API_KEY=\"your-api-key-here\""
    exit 1
fi

API_KEY="$1"

# 方法1: 设置环境变量
export GLM_API_KEY="$API_KEY"
echo "✅ 已设置环境变量 GLM_API_KEY"

# 方法2: 更新config.json文件
if [ -f "config.json" ]; then
    # 创建临时文件
    temp_file=$(mktemp)
    
    # 使用Python来更新JSON文件
    python3 -c "
import json
with open('config.json', 'r') as f:
    config = json.load(f)

config['ai']['api_key'] = '$API_KEY'

with open('$temp_file', 'w') as f:
    json.dump(config, f, indent=2, ensure_ascii=False)
"
    
    # 替换原文件
    mv "$temp_file" config.json
    echo "✅ 已更新 config.json 文件"
else
    echo "❌ config.json 文件不存在"
fi

# 方法3: 创建setup脚本
cat > setup_glm_api.sh << 'EOF'
#!/bin/bash
export GLM_API_KEY="$API_KEY"
echo "GLM API密钥已设置，请运行以下命令来启动应用:"
echo "source setup_glm_api.sh"
echo "cargo run"
EOF

chmod +x setup_glm_api.sh
echo "✅ 已创建 setup_glm_api.sh 脚本"

echo ""
echo "=== 设置完成 ==="
echo "现在你可以:"
echo "1. 重启应用: cargo run"
echo "2. 或者运行: source setup_glm_api.sh && cargo run"
echo "3. 测试AI分析功能"
echo ""
echo "测试命令:"
echo "curl -X POST \"http://localhost:8080/api/analyze\" \\"
echo "  -H \"Content-Type: application/json\" \\"
echo "  -d '{\"stock_code\": \"000001\", \"enable_ai\": true}'"