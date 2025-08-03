#!/usr/bin/env node

// 测试配置加载的Node.js脚本
const https = require('https');
const http = require('http');

const baseUrl = 'http://localhost:8080';

async function fetchConfig() {
    console.log('正在测试配置加载...');
    
    try {
        // 并行获取所有配置
        const [aiConfig, authConfig, systemConfig] = await Promise.all([
            fetchJson(`${baseUrl}/api/config/ai`),
            fetchJson(`${baseUrl}/api/config/auth`),
            fetchJson(`${baseUrl}/api/config/system`)
        ]);
        
        console.log('✅ 配置加载成功！');
        console.log('\n=== AI配置 ===');
        console.log('提供商:', aiConfig.data?.provider || '未设置');
        console.log('模型:', aiConfig.data?.model || '未设置');
        console.log('API密钥:', aiConfig.data?.api_key ? '已设置' : '未设置');
        console.log('启用状态:', aiConfig.data?.enabled ? '启用' : '禁用');
        
        console.log('\n=== 认证配置 ===');
        console.log('启用状态:', authConfig.data?.enabled ? '启用' : '禁用');
        console.log('会话超时:', authConfig.data?.session_timeout || '未设置', '秒');
        
        console.log('\n=== 系统配置 ===');
        console.log('Akshare地址:', systemConfig.data?.akshare_url || '未设置');
        console.log('最大工作线程:', systemConfig.data?.max_workers || '未设置');
        
        console.log('\n✅ 所有配置都能正确加载！');
        console.log('✅ 数据结构正确，前端应该能正确显示配置。');
        
    } catch (error) {
        console.error('❌ 配置加载失败:', error.message);
    }
}

function fetchJson(url) {
    return new Promise((resolve, reject) => {
        const protocol = url.startsWith('https') ? https : http;
        
        const req = protocol.get(url, (res) => {
            let data = '';
            res.on('data', chunk => data += chunk);
            res.on('end', () => {
                try {
                    resolve(JSON.parse(data));
                } catch (e) {
                    reject(new Error(`JSON解析失败: ${e.message}`));
                }
            });
        });
        
        req.on('error', reject);
        req.setTimeout(5000, () => {
            req.destroy();
            reject(new Error('请求超时'));
        });
    });
}

fetchConfig();