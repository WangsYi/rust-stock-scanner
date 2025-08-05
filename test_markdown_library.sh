#!/bin/bash

echo "🧪 测试marked.js库集成..."

# 测试基本API调用
echo "📡 测试API接口..."
curl -s -X POST "http://localhost:8080/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{"stock_code": "000001", "enable_ai": false}' | jq -r '.data.ai_analysis' > /tmp/ai_analysis_new.md

echo "📝 AI分析内容已保存到 /tmp/ai_analysis_new.md"
echo "📋 前10行内容:"
head -10 /tmp/ai_analysis_new.md

echo ""
echo "🔍 检查markdown格式..."
if grep -q "^#" /tmp/ai_analysis_new.md; then
    echo "✅ 发现标题格式 (#)"
fi
if grep -q "\*\*" /tmp/ai_analysis_new.md; then
    echo "✅ 发现加粗格式 (**)"
fi
if grep -q "^-" /tmp/ai_analysis_new.md; then
    echo "✅ 发现列表格式 (-)"
fi
if grep -q "|.*|" /tmp/ai_analysis_new.md; then
    echo "✅ 发现表格格式 (|)"
fi
if grep -q "```" /tmp/ai_analysis_new.md; then
    echo "✅ 发现代码块格式 (\`\`\`)"
fi

echo ""
echo "🌐 访问以下地址测试新的marked.js渲染:"
echo "   http://localhost:8080/templates/index.html"
echo "   http://localhost:8080/test_markdown_library.html"
echo "   http://localhost:8080/test_markdown_comprehensive.html"
echo "   http://localhost:8080/test_markdown_fixed.html"

echo ""
echo "🎯 marked.js库优势:"
echo "   ✅ 成熟稳定，广泛使用"
echo "   ✅ 支持完整的GitHub Flavored Markdown"
echo "   ✅ 自动处理表格、代码块等复杂格式"
echo "   ✅ 更好的性能和兼容性"
echo "   ✅ 内置安全处理 (配合DOMPurify)"