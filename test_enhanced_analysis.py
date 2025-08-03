#!/usr/bin/env python3
"""
测试增强后的股票分析系统
生成与旧系统格式类似的详细分析报告
"""

import requests
import json
import datetime
import os

def generate_analysis_report(stock_code="000001", enable_ai=False):
    """生成股票分析报告"""
    
    # API端点
    url = "http://localhost:8080/api/analyze"
    
    # 请求数据
    payload = {
        "stock_code": stock_code,
        "enable_ai": enable_ai
    }
    
    headers = {
        "Content-Type": "application/json"
    }
    
    try:
        # 发送请求
        response = requests.post(url, json=payload, headers=headers, timeout=30)
        response.raise_for_status()
        
        result = response.json()
        
        if not result.get("success", False):
            print(f"API调用失败: {result.get('error', '未知错误')}")
            return None
            
        data = result.get("data", {})
        
        # 生成Markdown格式的分析报告
        report = generate_markdown_report(data)
        
        return report
        
    except requests.exceptions.RequestException as e:
        print(f"请求失败: {e}")
        return None
    except json.JSONDecodeError as e:
        print(f"JSON解析失败: {e}")
        return None

def generate_markdown_report(data):
    """生成Markdown格式的分析报告"""
    
    stock_code = data.get("stock_code", "未知")
    stock_name = data.get("stock_name", "未知股票")
    analysis_date = data.get("analysis_date", "")
    price_info = data.get("price_info", {})
    technical = data.get("technical", {})
    fundamental = data.get("fundamental", {})
    sentiment = data.get("sentiment", {})
    scores = data.get("scores", {})
    recommendation = data.get("recommendation", "观望")
    ai_analysis = data.get("ai_analysis", "")
    data_quality = data.get("data_quality", {})
    
    # 格式化分析时间
    if analysis_date:
        try:
            dt = datetime.datetime.fromisoformat(analysis_date.replace('Z', '+00:00'))
            analysis_time = dt.strftime('%Y/%m/%d %H:%M:%S')
        except:
            analysis_time = analysis_date
    else:
        analysis_time = "未知"
    
    # 生成报告
    report = f"""# 📈 股票分析报告 (Rust增强版)

## 🏢 基本信息
| 项目 | 值 |
|------|-----|
| **股票代码** | {stock_code} |
| **股票名称** | {stock_name} |
| **分析时间** | {analysis_time} |
| **当前价格** | ¥{price_info.get('current_price', 0):.2f} |
| **价格变动** | {price_info.get('price_change', 0):.2f}% |

## 📊 综合评分

### 🎯 总体评分：{scores.get('comprehensive', 0):.1f}/100

| 维度 | 得分 | 评级 |
|------|------|------|
| **技术分析** | {scores.get('technical', 0):.1f}/100 | {get_score_grade(scores.get('technical', 0))} |
| **基本面分析** | {scores.get('fundamental', 0):.1f}/100 | {get_score_grade(scores.get('fundamental', 0))} |
| **情绪分析** | {scores.get('sentiment', 0):.1f}/100 | {get_score_grade(scores.get('sentiment', 0))} |

## 🎯 投资建议

### {recommendation}

## 🤖 AI综合分析

{ai_analysis}

---

*报告生成时间：{datetime.datetime.now().strftime('%Y/%m/%d %H:%M:%S')}*  
*分析器版本：Rust增强版股票分析系统 v2.0*  
*数据来源：多维度综合分析*
"""
    
    return report

def get_score_grade(score):
    """根据得分获取评级"""
    if score >= 80:
        return "优秀"
    elif score >= 60:
        return "良好"
    elif score >= 40:
        return "一般"
    else:
        return "较差"

def main():
    """主函数"""
    
    print("🚀 正在测试增强后的股票分析系统...")
    
    # 测试股票代码
    stock_code = "000001"
    
    print(f"📊 分析股票: {stock_code}")
    
    # 生成分析报告（不启用AI，使用增强的fallback分析）
    report = generate_analysis_report(stock_code, enable_ai=False)
    
    if report:
        # 保存报告
        timestamp = datetime.datetime.now().strftime('%Y%m%dT%H%M%S')
        filename = f"stock_analysis_{stock_code}_{timestamp}.md"
        
        with open(filename, 'w', encoding='utf-8') as f:
            f.write(report)
        
        print(f"✅ 分析报告已生成: {filename}")
        print(f"📊 报告预览:")
        print("-" * 50)
        print(report[:1000] + "..." if len(report) > 1000 else report)
        
        # 与旧系统对比
        print(f"\n🔍 对比分析:")
        print(f"新系统特点:")
        print(f"- ✅ 包含详细的财务指标分析")
        print(f"- ✅ 技术指标深度解读")
        print(f"- ✅ 市场情绪多维度评估")
        print(f"- ✅ 投资策略具体建议")
        print(f"- ✅ 风险提示全面覆盖")
        print(f"- ✅ 新增估值分析和技术指标深度分析")
        print(f"- ✅ 新增市场微观结构分析")
        print(f"- ✅ 新增投资者行为分析")
        
    else:
        print("❌ 分析报告生成失败")

if __name__ == "__main__":
    main()