#!/bin/bash

echo "🧪 验证marked.js库集成状态..."

# 检查服务器状态
echo "📡 检查服务器状态..."
if curl -s http://localhost:8080/api/health > /dev/null; then
    echo "✅ 服务器运行正常"
else
    echo "❌ 服务器未运行"
    exit 1
fi

# 测试API响应
echo "📡 测试API响应..."
API_RESPONSE=$(curl -s -X POST "http://localhost:8080/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{"stock_code": "000001", "enable_ai": false}')

if echo "$API_RESPONSE" | jq -e '.success' > /dev/null; then
    echo "✅ API调用成功"
    
    # 提取AI分析内容
    AI_ANALYSIS=$(echo "$API_RESPONSE" | jq -r '.data.ai_analysis')
    echo "$AI_ANALYSIS" > /tmp/ai_analysis_verification.md
    
    echo "📝 AI分析内容已保存到 /tmp/ai_analysis_verification.md"
    
    # 检查markdown格式
    echo "🔍 检查markdown格式..."
    if grep -q "^#" /tmp/ai_analysis_verification.md; then
        echo "✅ 发现标题格式 (#)"
    fi
    if grep -q "\*\*" /tmp/ai_analysis_verification.md; then
        echo "✅ 发现加粗格式 (**)"
    fi
    if grep -q "^-" /tmp/ai_analysis_verification.md; then
        echo "✅ 发现列表格式 (-)"
    fi
    if grep -q "|.*|" /tmp/ai_analysis_verification.md; then
        echo "✅ 发现表格格式 (|)"
    fi
    
else
    echo "❌ API调用失败"
    echo "$API_RESPONSE"
fi

echo ""
echo "🌐 验证页面地址:"
echo "   http://localhost:8080/test_markdown_verification.html"
echo "   http://localhost:8080/templates/index.html"
echo "   http://localhost:8080/test_markdown_library.html"

echo ""
echo "🔧 故障排除提示:"
echo "   1. 如果页面仍显示旧格式，请清除浏览器缓存 (Ctrl+F5)"
echo "   2. 确保marked.js和DOMPurify库正确加载"
echo "   3. 检查浏览器控制台是否有JavaScript错误"
echo "   4. 验证网络连接可以访问CDN"

echo ""
echo "📋 测试步骤:"
echo "   1. 访问 http://localhost:8080/test_markdown_verification.html"
echo "   2. 检查'库加载状态'是否显示成功"
echo "   3. 点击各个测试按钮验证markdown渲染"
echo "   4. 查看原始markdown和渲染后的HTML对比"