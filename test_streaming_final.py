#!/usr/bin/env python3
"""
测试流式AI分析功能
"""
import subprocess
import json
import time

print('🚀 开始完整测试流式AI分析功能...')

# 启动流式分析
proc = subprocess.Popen([
    'curl', '-X', 'POST', 
    'http://localhost:8080/api/analyze/stream',
    '-H', 'Content-Type: application/json',
    '-d', json.dumps({'stock_code': '000001', 'enable_ai': True}),
    '-m', '45'
], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

# 读取输出
lines = []
for line in proc.stdout:
    line = line.strip()
    if line.startswith('data: '):
        data_str = line[6:]
        try:
            data = json.loads(data_str)
            lines.append(data)
            
            if data.get('type') == 'streaming_content':
                content = data.get('content', '')
                print(f'📝 流式内容: {len(content)} 字符')
            elif data.get('type') == 'progress':
                progress_data = data.get('data', {})
                percentage = progress_data.get('percentage', 0)
                status = progress_data.get('status', '')
                print(f'📊 进度: {percentage:.1f}% - {status}')
            elif data.get('type') == 'final_result':
                print('🎉 分析完成!')
            elif data.get('type') == 'started':
                print(f'🎯 {data.get("message", "开始分析")}')
                
        except json.JSONDecodeError:
            pass

# 统计结果
streaming_count = len([l for l in lines if l.get('type') == 'streaming_content'])
progress_count = len([l for l in lines if l.get('type') == 'progress'])
final_result = any(l.get('type') == 'final_result' for l in lines)

print(f'\n📊 测试结果统计:')
print(f'   流式内容片段: {streaming_count}')
print(f'   进度更新: {progress_count}')
print(f'   最终结果: {"✅" if final_result else "❌"}')

if streaming_count > 0 and progress_count > 0 and final_result:
    print('🎉 所有功能测试通过!')
else:
    print('⚠️ 部分功能可能存在问题')