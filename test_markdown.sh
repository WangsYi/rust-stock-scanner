#!/bin/bash

echo "🧪 测试Markdown解析功能..."

# 测试API响应
echo "📡 测试API接口..."
curl -s -X POST "http://localhost:8080/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{"stock_code": "000001", "enable_ai": false}' | jq -r '.data.ai_analysis' > /tmp/ai_analysis.md

echo "📝 AI分析内容已保存到 /tmp/ai_analysis.md"
echo "📋 前5行内容:"
head -5 /tmp/ai_analysis.md

echo ""
echo "🔍 检查是否包含markdown格式..."
if grep -q "^#" /tmp/ai_analysis.md; then
    echo "✅ 发现标题格式 (#)"
fi
if grep -q "\*\*" /tmp/ai_analysis.md; then
    echo "✅ 发现加粗格式 (**)"
fi
if grep -q "^-" /tmp/ai_analysis.md; then
    echo "✅ 发现列表格式 (-)"
fi
if grep -q "|.*|" /tmp/ai_analysis.md; then
    echo "✅ 发现表格格式 (|)"
fi

echo ""
echo "🌐 请访问以下地址测试前端渲染:"
echo "   http://localhost:8080/templates/index.html"
echo "   http://localhost:8081/test_markdown_comprehensive.html"