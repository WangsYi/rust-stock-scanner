#!/bin/bash

echo "🎯 Markdown库集成最终验证"
echo "================================"

# 1. 检查服务器状态
echo "1. 检查服务器状态..."
if curl -s http://localhost:8080/api/health > /dev/null; then
    echo "✅ 服务器运行正常"
else
    echo "❌ 服务器未运行"
    exit 1
fi

# 2. 检查主页面库引用
echo "2. 检查主页面库引用..."
if grep -q "marked.min.js" /home/wangs/code/stock-scanner/templates/index.html; then
    echo "✅ 主页面包含marked.js引用"
else
    echo "❌ 主页面缺少marked.js引用"
fi

if grep -q "DOMPurify" /home/wangs/code/stock-scanner/templates/index.html; then
    echo "✅ 主页面包含DOMPurify引用"
else
    echo "❌ 主页面缺少DOMPurify引用"
fi

# 3. 检查JavaScript函数更新
echo "3. 检查JavaScript函数更新..."
if grep -q "marked.parse" /home/wangs/code/stock-scanner/templates/index.html; then
    echo "✅ JavaScript函数已更新为使用marked.js"
else
    echo "❌ JavaScript函数未更新"
fi

# 4. 测试API响应
echo "4. 测试API响应..."
API_RESPONSE=$(curl -s -X POST "http://localhost:8080/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{"stock_code": "000001", "enable_ai": false}')

if echo "$API_RESPONSE" | jq -e '.success' > /dev/null; then
    echo "✅ API调用成功"
    
    # 检查AI分析内容
    AI_ANALYSIS=$(echo "$API_RESPONSE" | jq -r '.data.ai_analysis')
    echo "$AI_ANALYSIS" > /tmp/final_test_ai_analysis.md
    
    echo "📝 AI分析内容示例（前5行）:"
    head -5 /tmp/final_test_ai_analysis.md
    
    # 检查各种markdown格式
    echo "🔍 检查markdown格式支持:"
    if grep -q "^#" /tmp/final_test_ai_analysis.md; then
        echo "  ✅ 标题格式 (#)"
    fi
    if grep -q "\*\*" /tmp/final_test_ai_analysis.md; then
        echo "  ✅ 加粗格式 (**)"
    fi
    if grep -q "^-" /tmp/final_test_ai_analysis.md; then
        echo "  ✅ 列表格式 (-)"
    fi
    if grep -q "|.*|" /tmp/final_test_ai_analysis.md; then
        echo "  ✅ 表格格式 (|)"
    fi
    if grep -q "```" /tmp/final_test_ai_analysis.md; then
        echo "  ✅ 代码块格式 (\`\`\`)"
    fi
    if grep -q ">" /tmp/final_test_ai_analysis.md; then
        echo "  ✅ 引用格式 (>)"
    fi
    
else
    echo "❌ API调用失败"
    echo "$API_RESPONSE" | head -5
fi

echo ""
echo "🌐 可用的测试页面:"
echo "   - 主应用: http://localhost:8080/templates/index.html"
echo "   - 验证页面: http://localhost:8080/test_markdown_verification.html"
echo "   - 库测试: http://localhost:8080/test_markdown_library.html"
echo "   - 综合测试: http://localhost:8080/test_markdown_comprehensive.html"

echo ""
echo "🔧 如果页面仍显示旧格式，请尝试:"
echo "   1. 清除浏览器缓存 (Ctrl+F5 或 Cmd+Shift+R)"
echo "   2. 检查浏览器开发者工具的控制台是否有错误"
echo "   3. 确认网络可以访问CDN (cdn.jsdelivr.net)"
echo "   4. 尝试使用无痕/隐私模式访问页面"

echo ""
echo "✅ Markdown库集成验证完成！"