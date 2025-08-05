#!/bin/bash

# Test script for stock analyzer API endpoints

echo "=== 股票分析系统 API 测试 ==="
echo "测试时间: $(date)"
echo

# Test health endpoint
echo "1. 测试健康检查端点..."
curl -s http://localhost:8080/api/health | jq .
echo

# Test datasource test endpoint (new)
echo "2. 测试数据源测试端点 (新路径)..."
curl -s -X POST http://localhost:8080/api/datasource/test | jq .
echo

# Test datasource test endpoint (original)
echo "3. 测试数据源测试端点 (原路径)..."
curl -s -X POST http://localhost:8080/api/config/datasource/test | jq .
echo

# Test AI config endpoint
echo "4. 测试AI配置端点..."
curl -s http://localhost:8080/api/config/ai | jq .
echo

# Test single stock analysis
echo "5. 测试单股票分析 (000001)..."
curl -s -X POST http://localhost:8080/api/analyze \
  -H "Content-Type: application/json" \
  -d '{"stock_code": "000001", "enable_ai": false}' | jq '.data | {stock_code, stock_name, market, recommendation}' 2>/dev/null || echo "分析完成"
echo

echo "=== 测试完成 ==="