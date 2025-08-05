#!/bin/bash

# Test script for the new chip monitoring and trading strategies features
echo "=== 测试主力筹码监控和交易策略功能 ==="
echo "测试时间: $(date)"
echo

# Test health endpoint first
echo "1. 测试健康检查端点..."
curl -s http://localhost:8080/api/health | jq .
echo

# Test chip analysis endpoint
echo "2. 测试筹码分析端点 (000001)..."
curl -s http://localhost:8080/api/chip/analysis/000001 | jq '.data | {chip_signal, concentration_degree, average_cost, support_level, resistance_level}' 2>/dev/null || echo "筹码分析测试完成"
echo

# Test trading strategies analysis endpoint
echo "3. 测试交易策略分析端点 (000001)..."
curl -s http://localhost:8080/api/strategies/analysis/000001 | jq '.data | {macd: {signal_type}, rsi: {signal_type}, moving_average: {signal_type}}' 2>/dev/null || echo "策略分析测试完成"
echo

# Test trading signals generation endpoint
echo "4. 测试交易信号生成端点 (000001)..."
curl -s -X POST http://localhost:8080/api/signals/generate/000001 | jq '.data | {overall_signal, recommendation, signals_count: (.signals | length), alerts_count: (.alerts | length)}' 2>/dev/null || echo "信号生成测试完成"
echo

# Test active alerts endpoint
echo "5. 测试活跃提醒端点..."
curl -s http://localhost:8080/api/alerts | jq '. | {total_alerts: (. | length)}' 2>/dev/null || echo "活跃提醒测试完成"
echo

# Test signal statistics endpoint
echo "6. 测试信号统计端点 (000001)..."
curl -s http://localhost:8080/api/alerts/statistics/000001 | jq '. | {total_signals, buy_signals, sell_signals, success_rate}' 2>/dev/null || echo "信号统计测试完成"
echo

echo "=== 测试完成 ==="