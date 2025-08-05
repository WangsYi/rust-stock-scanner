#!/usr/bin/env python3
import requests
import json
import time

# 测试AI配置
def test_ai_config():
    try:
        response = requests.get("http://localhost:8080/api/config/ai", timeout=5)
        if response.status_code == 200:
            config = response.json()
            print("AI配置加载成功:")
            print(f"Provider: {config['data']['provider']}")
            print(f"Model: {config['data']['model']}")
            print(f"Enabled: {config['data']['enabled']}")
            # 注意：这里需要查看实际返回的超时设置
            return True
        else:
            print(f"获取AI配置失败: {response.status_code}")
            return False
    except Exception as e:
        print(f"请求失败: {e}")
        return False

# 测试股票分析（不使用AI）
def test_analysis_without_ai():
    try:
        payload = {
            "stock_code": "000001",
            "enable_ai": False
        }
        response = requests.post("http://localhost:8080/api/analyze", 
                               json=payload, 
                               timeout=10)
        if response.status_code == 200:
            result = response.json()
            print("分析成功（无AI）:")
            print(f"Stock: {result['data']['stock_code']}")
            print(f"Recommendation: {result['data']['recommendation']}")
            print(f"Fallback used: {result['data']['fallback_used']}")
            return True
        else:
            print(f"分析失败: {response.status_code}")
            return False
    except Exception as e:
        print(f"分析请求失败: {e}")
        return False

if __name__ == "__main__":
    print("开始测试...")
    
    # 等待应用启动
    print("等待应用启动...")
    time.sleep(3)
    
    # 测试AI配置
    if test_ai_config():
        print("✅ AI配置测试通过")
    else:
        print("❌ AI配置测试失败")
    
    # 测试分析功能
    if test_analysis_without_ai():
        print("✅ 分析功能测试通过")
    else:
        print("❌ 分析功能测试失败")
    
    print("测试完成")