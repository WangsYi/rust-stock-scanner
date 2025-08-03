use reqwest::Client;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use futures::StreamExt;
use tokio::sync::mpsc;
use chrono::Utc;

use crate::models::{AIConfig, AnalysisReport};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingAnalysisRequest {
    pub report: AnalysisReport,
    pub enable_streaming: bool,
    pub analysis_depth: AnalysisDepth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisDepth {
    Basic,
    Standard,
    Comprehensive,
    Professional,
}

impl Default for AnalysisDepth {
    fn default() -> Self {
        AnalysisDepth::Standard
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChunk {
    pub content: String,
    pub chunk_type: String,
    pub progress: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    pub provider: String,
    pub model: String,
    pub tokens_used: u32,
    pub processing_time_ms: u64,
    pub confidence_score: f64,
    pub analysis_dimensions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AIService {
    config: AIConfig,
    client: Client,
}

impl AIService {
    pub fn new(config: AIConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    pub async fn generate_streaming_analysis(
        &self,
        request: StreamingAnalysisRequest,
    ) -> Result<mpsc::UnboundedReceiver<StreamingChunk>, String> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        if !self.config.enabled || self.config.api_key.is_empty() {
            // Send fallback analysis in chunks
            let fallback_content = self.generate_enhanced_fallback_analysis(&request.report, &request.analysis_depth);
            AIService::send_fallback_in_chunks(tx, fallback_content).await;
            return Ok(rx);
        }

        let prompt = self.build_enhanced_analysis_prompt(&request.report, &request.analysis_depth);
        let config = self.config.clone();

        // Spawn streaming task
        tokio::spawn(async move {
            match config.provider.as_str() {
                "openai" => Self::stream_openai_analysis(&prompt, tx, &config).await,
                "claude" => Self::stream_claude_analysis(&prompt, tx, &config).await,
                "baidu" => Self::stream_baidu_analysis(&prompt, tx, &config).await,
                "tencent" => Self::stream_tencent_analysis(&prompt, tx, &config).await,
                "glm" => Self::stream_glm_analysis(&prompt, tx, &config).await,
                "qwen" => Self::stream_qwen_analysis(&prompt, tx, &config).await,
                "kimi" => Self::stream_kimi_analysis(&prompt, tx, &config).await,
                "ollama" => Self::stream_ollama_analysis(&prompt, tx, &config).await,
                _ => {
                    let fallback_content = Self::generate_enhanced_fallback_analysis_static(&request.report, &request.analysis_depth);
                    AIService::send_fallback_in_chunks(tx, fallback_content).await;
                }
            }
        });

        Ok(rx)
    }

    async fn send_fallback_in_chunks(
        tx: mpsc::UnboundedSender<StreamingChunk>,
        content: String,
    ) {
        let chunks: Vec<&str> = content.split("\n\n").collect();
        let total_chunks = chunks.len();

        for (i, chunk) in chunks.iter().enumerate() {
            if !chunk.is_empty() {
                let streaming_chunk = StreamingChunk {
                    content: chunk.to_string(),
                    chunk_type: "analysis".to_string(),
                    progress: (i as f64 / total_chunks as f64) * 100.0,
                    timestamp: Utc::now(),
                };

                if tx.send(streaming_chunk).is_err() {
                    break;
                }

                // Small delay between chunks for streaming effect
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        }

        // Send completion signal
        let completion_chunk = StreamingChunk {
            content: "".to_string(),
            chunk_type: "complete".to_string(),
            progress: 100.0,
            timestamp: Utc::now(),
        };
        let _ = tx.send(completion_chunk);
    }

    pub async fn generate_analysis(&self,
        report: &AnalysisReport,
    ) -> Result<String, String> {
        if !self.config.enabled || self.config.api_key.is_empty() {
            return Ok(self.generate_fallback_analysis(report));
        }

        let prompt = self.build_analysis_prompt(report);
        
        match self.config.provider.as_str() {
            "openai" => self.call_openai(&prompt).await,
            "claude" => self.call_claude(&prompt).await,
            "baidu" => self.call_baidu(&prompt).await,
            "tencent" => self.call_tencent(&prompt).await,
            "glm" => self.call_glm(&prompt).await,
            "qwen" => self.call_qwen(&prompt).await,
            "kimi" => self.call_kimi(&prompt).await,
            "ollama" => self.call_ollama(&prompt).await,
            _ => Ok(self.generate_fallback_analysis(report)),
        }
    }

    async fn call_openai(&self, prompt: &str) -> Result<String, String> {
        let url = self.config.base_url.as_ref()
            .unwrap_or(&"https://api.openai.com/v1/chat/completions".to_string())
            .clone();

        let payload = json!({
            "model": self.config.model.as_ref().unwrap_or(&"gpt-3.5-turbo".to_string()),
            "messages": [
                {
                    "role": "system",
                    "content": "你是一位资深的股票分析师，具有丰富的市场经验和深厚的金融知识。请提供专业、客观、有深度的股票分析。"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 4000,
            "temperature": 0.7
        });

        self.make_post_request(&url,
            &payload,
            &[("Authorization", format!("Bearer {}", self.config.api_key))]
        ).await
    }

    async fn call_claude(&self, prompt: &str) -> Result<String, String> {
        let url = self.config.base_url.as_ref()
            .unwrap_or(&"https://api.anthropic.com/v1/messages".to_string())
            .clone();

        let payload = json!({
            "model": self.config.model.as_ref().unwrap_or(&"claude-3-sonnet-20240229".to_string()),
            "max_tokens": 4000,
            "messages": [
                {
                    "role": "user",
                    "content": format!("你是一位资深的股票分析师，具有丰富的市场经验和深厚的金融知识。请提供专业、客观、有深度的股票分析。\n\n{}", prompt)
                }
            ]
        });

        self.make_post_request(
            &url,
            &payload,
            &[
                ("x-api-key", self.config.api_key.clone()),
                ("anthropic-version", "2023-06-01".to_string())
            ]
        ).await
    }

    async fn call_baidu(&self, prompt: &str) -> Result<String, String> {
        let url = self.config.base_url.as_ref()
            .unwrap_or(&"https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/completions".to_string())
            .clone();

        let payload = json!({
            "messages": [
                {
                    "role": "system",
                    "content": "你是一位资深的股票分析师，具有丰富的市场经验和深厚的金融知识。请提供专业、客观、有深度的股票分析。"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.7,
            "max_tokens": 4000
        });

        self.make_post_request(
            &url,
            &payload,
            &[("Authorization", format!("Bearer {}", self.config.api_key))]
        ).await
    }

    async fn call_tencent(&self, prompt: &str) -> Result<String, String> {
        let url = self.config.base_url.as_ref()
            .unwrap_or(&"https://hunyuan.tencentcloudapi.com".to_string())
            .clone();

        let payload = json!({
            "Model": self.config.model.as_ref().unwrap_or(&"hunyuan-standard".to_string()),
            "Messages": [
                {
                    "Role": "system",
                    "Content": "你是一位资深的股票分析师，具有丰富的市场经验和深厚的金融知识。请提供专业、客观、有深度的股票分析。"
                },
                {
                    "Role": "user",
                    "Content": prompt
                }
            ],
            "Temperature": 0.7,
            "TopP": 0.9,
            "MaxTokens": 4000
        });

        self.make_post_request(
            &url,
            &payload,
            &[("Authorization", format!("Bearer {}", self.config.api_key))]
        ).await
    }

    async fn call_glm(&self, prompt: &str) -> Result<String, String> {
        let url = self.config.base_url.as_ref()
            .unwrap_or(&"https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string())
            .clone();

        let payload = json!({
            "model": self.config.model.as_ref().unwrap_or(&"glm-4".to_string()),
            "messages": [
                {
                    "role": "system",
                    "content": "你是一位资深的股票分析师，具有丰富的市场经验和深厚的金融知识。请提供专业、客观、有深度的股票分析。"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 4000,
            "temperature": 0.7
        });

        self.make_post_request(
            &url,
            &payload,
            &[("Authorization", format!("Bearer {}", self.config.api_key))]
        ).await
    }

    async fn call_qwen(&self, prompt: &str) -> Result<String, String> {
        let url = self.config.base_url.as_ref()
            .unwrap_or(&"https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions".to_string())
            .clone();

        let payload = json!({
            "model": self.config.model.as_ref().unwrap_or(&"qwen-turbo".to_string()),
            "messages": [
                {
                    "role": "system",
                    "content": "你是一位资深的股票分析师，具有丰富的市场经验和深厚的金融知识。请提供专业、客观、有深度的股票分析。"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 4000,
            "temperature": 0.7
        });

        self.make_post_request(
            &url,
            &payload,
            &[("Authorization", format!("Bearer {}", self.config.api_key))]
        ).await
    }

    async fn call_kimi(&self, prompt: &str) -> Result<String, String> {
        let url = self.config.base_url.as_ref()
            .unwrap_or(&"https://api.moonshot.cn/v1/chat/completions".to_string())
            .clone();

        let payload = json!({
            "model": self.config.model.as_ref().unwrap_or(&"kimi-8k".to_string()),
            "messages": [
                {
                    "role": "system",
                    "content": "你是一位资深的股票分析师，具有丰富的市场经验和深厚的金融知识。请提供专业、客观、有深度的股票分析。"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 4000,
            "temperature": 0.7
        });

        self.make_post_request(
            &url,
            &payload,
            &[("Authorization", format!("Bearer {}", self.config.api_key))]
        ).await
    }

    async fn call_ollama(&self, prompt: &str) -> Result<String, String> {
        let url = self.config.base_url.as_ref()
            .unwrap_or(&"http://localhost:11434/v1/chat/completions".to_string())
            .clone();

        let payload = json!({
            "model": self.config.model.as_ref().unwrap_or(&"llama2".to_string()),
            "messages": [
                {
                    "role": "system",
                    "content": "你是一位资深的股票分析师，具有丰富的市场经验和深厚的金融知识。请提供专业、客观、有深度的股票分析。"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 4000,
            "temperature": 0.7
        });

        self.make_post_request(
            &url,
            &payload,
            &[("Authorization", format!("Bearer {}", self.config.api_key))]
        ).await
    }

    async fn make_post_request(
        &self,
        url: &str,
        payload: &Value,
        headers: &[(&str, String)],
    ) -> Result<String, String> {
        let mut request = self.client.post(url).json(payload);
        
        for (key, value) in headers {
            request = request.header(*key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Extract content from different response formats
        let content = response_json
            .get("choices")
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("message"))
            .and_then(|v| v.get("content"))
            .or_else(|| response_json.get("content"))
            .or_else(|| response_json.get("result"))
            .and_then(|v| v.as_str())
            .unwrap_or("AI分析功能暂不可用，请稍后再试。");

        Ok(content.to_string())
    }

    fn build_analysis_prompt(&self,
        report: &AnalysisReport,
    ) -> String {
        // Extract financial indicators for detailed analysis
        let financial_text = if !report.fundamental.financial_indicators.is_empty() {
            let mut indicators = String::from("**25项核心财务指标：**\n");
            for (i, indicator) in report.fundamental.financial_indicators.iter().take(25).enumerate() {
                indicators.push_str(&format!("{}. {}: {}\n", i + 1, indicator.name, indicator.value));
            }
            indicators
        } else {
            String::from("财务指标数据不足\n")
        };

        // Extract news details
        let news_summary = &report.sentiment;
        let news_text = format!(
            "**新闻数据详情：**
- 总新闻数：{}条
- 置信度：{:.2}
- 情绪趋势：{}

**重要新闻摘要：**
前{}条新闻显示市场情绪为{}，整体得分为{:.3}",
            news_summary.total_analyzed,
            news_summary.confidence_score,
            news_summary.sentiment_trend,
            std::cmp::min(news_summary.total_analyzed, 10),
            news_summary.sentiment_trend,
            news_summary.overall_sentiment
        );

        // Build comprehensive prompt similar to Python version
        format!(
            "请作为一位资深的股票分析师，基于以下详细数据对股票进行深度分析：

**股票基本信息：**
- 股票代码：{}
- 股票名称：{}
- 当前价格：{:.2}元
- 涨跌幅：{:.2}%
- 成交量比率：{:.2}
- 波动率：{:.2}%

**技术分析详情：**
- 均线趋势：{}
- RSI指标：{:.1}
- MACD信号：{}
- 布林带位置：{:.2}
- 成交量状态：{}

{}

**估值指标：**
- 市盈率：{:.2}倍
- 市净率：{:.2}倍

**行业信息：**
- 行业：{}
- 板块：{}

{}

**市场情绪分析：**
- 整体情绪得分：{:.3}
- 情绪趋势：{}
- 置信度：{:.2}
- 分析新闻数量：{}条

**综合评分：**
- 技术面得分：{:.0}/100
- 基本面得分：{:.0}/100
- 情绪面得分：{:.0}/100
- 综合得分：{:.0}/100

**分析要求：**

请基于以上详细数据，从以下维度进行深度分析：

1. **财务健康度深度解读**：
   - 基于25项财务指标，全面评估公司财务状况
   - 识别财务优势和风险点
   - 与行业平均水平对比分析
   - 预测未来财务发展趋势

2. **技术面精准分析**：
   - 结合多个技术指标，判断短中长期趋势
   - 识别关键支撑位和阻力位
   - 分析成交量与价格的配合关系
   - 评估当前位置的风险收益比

3. **市场情绪深度挖掘**：
   - 分析公司新闻、公告、研报的影响
   - 评估市场对公司的整体预期
   - 识别情绪拐点和催化剂
   - 判断情绪对股价的推动或拖累作用

4. **基本面价值判断**：
   - 评估公司内在价值和成长潜力
   - 分析行业地位和竞争优势
   - 评估业绩预告和分红政策
   - 判断当前估值的合理性

5. **综合投资策略**：
   - 给出明确的买卖建议和理由
   - 设定目标价位和止损点
   - 制定分批操作策略
   - 评估投资时间周期

6. **风险机会识别**：
   - 列出主要投资风险和应对措施
   - 识别潜在催化剂和成长机会
   - 分析宏观环境和政策影响
   - 提供动态调整建议

请用专业、客观的语言进行分析，确保逻辑清晰、数据支撑充分、结论明确可执行。",
            report.stock_code,
            report.stock_name,
            report.price_info.current_price,
            report.price_info.price_change,
            report.price_info.volume_ratio,
            report.price_info.volatility,
            report.technical.ma_trend,
            report.technical.rsi,
            report.technical.macd_signal,
            report.technical.bb_position,
            report.technical.volume_status,
            financial_text,
            report.fundamental.valuation.get("pe_ratio").unwrap_or(&0.0),
            report.fundamental.valuation.get("pb_ratio").unwrap_or(&0.0),
            report.fundamental.industry,
            report.fundamental.sector,
            news_text,
            report.sentiment.overall_sentiment,
            report.sentiment.sentiment_trend,
            report.sentiment.confidence_score,
            report.sentiment.total_analyzed,
            report.scores.technical,
            report.scores.fundamental,
            report.scores.sentiment,
            report.scores.comprehensive,
        )
    }

    pub fn generate_fallback_analysis(&self,
        report: &AnalysisReport,
    ) -> String {
        let rsi_status = if report.technical.rsi > 70.0 { "超买" } else if report.technical.rsi < 30.0 { "超卖" } else { "正常" };
        let trend_strength = match report.technical.adx {
            x if x > 50.0 => "强趋势",
            x if x > 25.0 => "中等趋势",
            _ => "弱趋势"
        };
        
        format!(
            "基于对{}股票的综合技术分析报告：

【基本信息】
股票代码：{}
当前股价：{:.2}元
涨跌幅：{:.2}%
成交量状态：{}
波动率：{:.2}%

【技术分析】
- 移动平均线：5日均线{:.2}，10日均线{:.2}，20日均线{:.2}，60日均线{:.2}
- 趋势分析：{}，{}
- RSI指标：{:.2}（{}）
- MACD信号：{}，MACD线：{:.3}
- 布林带位置：{:.2}，上轨{:.2}，中轨{:.2}，下轨{:.2}
- 威廉指标：{:.2}
- 随机指标：K值{:.2}，D值{:.2}
- 平均真实范围：{:.2}

【基本面分析】
- 市盈率：{:.2}倍
- 市净率：{:.2}倍
- 行业分类：{}
- 板块分类：{}
- 财务健康评分：{:.1}分
- 盈利能力评分：{:.1}分
- 流动性评分：{:.1}分
- 偿债能力评分：{:.1}分

【市场情绪】
- 整体情绪：{:.2}（{}）
- 情绪趋势：{}
- 置信度：{:.2}
- 分析新闻数量：{}条

【综合评分】
- 技术面评分：{:.1}分
- 基本面评分：{:.1}分
- 情绪面评分：{:.1}分
- 综合评分：{:.1}分

【投资建议】
推荐等级：{}
建议理由：基于技术面{}、基本面{}和市场情绪{}的综合评估，当前适合{}

【风险提示】
1. 技术指标仅供参考，不构成投资建议
2. 基本面数据可能存在滞后性
3. 市场情绪变化迅速，需密切关注
4. 投资有风险，入市需谨慎
5. 建议结合多方面信息进行投资决策

【分析师观点】
该股票目前技术面显示{}，基本面表现{}，市场情绪{}。投资者应密切关注{}的变化，并控制投资风险。

*注：本分析基于历史数据和公开信息，不作为投资建议*",
            report.stock_name,
            report.stock_code,
            report.price_info.current_price,
            report.price_info.price_change,
            report.technical.volume_status,
            report.price_info.volatility,
            report.technical.ma5,
            report.technical.ma10,
            report.technical.ma20,
            report.technical.ma60,
            report.technical.ma_trend,
            trend_strength,
            report.technical.rsi,
            rsi_status,
            report.technical.macd_signal,
            report.technical.macd_line,
            report.technical.bb_position,
            report.technical.bb_upper,
            report.technical.bb_middle,
            report.technical.bb_lower,
            report.technical.williams_r,
            report.technical.stochastic_k,
            report.technical.stochastic_d,
            report.technical.atr,
            report.fundamental.valuation.get("pe_ratio").unwrap_or(&0.0),
            report.fundamental.valuation.get("pb_ratio").unwrap_or(&0.0),
            report.fundamental.industry,
            report.fundamental.sector,
            report.fundamental.financial_health.overall_health_score,
            report.fundamental.financial_health.profitability_score,
            report.fundamental.financial_health.liquidity_score,
            report.fundamental.financial_health.solvency_score,
            report.sentiment.overall_sentiment,
            report.sentiment.sentiment_trend,
            report.sentiment.overall_sentiment,
            report.sentiment.confidence_score,
            report.sentiment.total_analyzed,
            report.scores.technical,
            report.scores.fundamental,
            report.scores.sentiment,
            report.scores.comprehensive,
            report.recommendation,
            if report.scores.technical > 60.0 { "相对强势" } else if report.scores.technical < 40.0 { "相对弱势" } else { "中性" },
            if report.scores.fundamental > 60.0 { "良好" } else if report.scores.fundamental < 40.0 { "较差" } else { "一般" },
            if report.scores.sentiment > 60.0 { "积极" } else if report.scores.sentiment < 40.0 { "消极" } else { "中性" },
            if report.scores.comprehensive > 60.0 { "适度参与" } else if report.scores.comprehensive < 40.0 { "谨慎观望" } else { "保持关注" },
            if report.scores.technical > 60.0 { "处于上升趋势" } else if report.scores.technical < 40.0 { "处于下降趋势" } else { "处于盘整阶段" },
            if report.scores.fundamental > 60.0 { "较好" } else if report.scores.fundamental < 40.0 { "较差" } else { "一般" },
            if report.scores.sentiment > 60.0 { "较为积极" } else if report.scores.sentiment < 40.0 { "较为消极" } else { "相对中性" },
            if report.technical.adx > 25.0 { "趋势强度" } else { "价格走势" }
        )
    }

    fn build_enhanced_analysis_prompt(&self, report: &AnalysisReport, depth: &AnalysisDepth) -> String {
        let base_prompt = self.build_analysis_prompt(report);
        
        let depth_instructions = match depth {
            AnalysisDepth::Basic => "
**分析要求（基础级）：**
请提供简洁明了的股票分析，重点关注：
1. 当前股价表现和技术指标
2. 基本面估值情况
3. 简单的买卖建议
4. 主要风险提示",
            AnalysisDepth::Standard => "
**分析要求（标准级）：**
请基于数据提供全面的股票分析，包括：
1. 技术面趋势分析
2. 基本面价值评估
3. 市场情绪解读
4. 综合投资建议
5. 风险收益分析",
            AnalysisDepth::Comprehensive => "
**分析要求（专业级）：**
请进行深度专业分析，涵盖：
1. 财务健康度多维度评估
2. 技术面精确分析和预测
3. 行业竞争地位和成长性
4. 宏观环境和政策影响
5. 量化模型和风险评估
6. 动态投资策略建议",
            AnalysisDepth::Professional => "
**分析要求（机构级）：**
请提供机构级别的深度研究报告，包括：
1. 详细的财务建模和DCF估值
2. 敏感性分析和情景分析
3. 行业深度研究和竞争格局
4. 管理层能力和公司治理评估
5. ESG因素和可持续发展分析
6. 机构资金流向和市场微观结构
7. 详细的期权策略和风险对冲建议",
        };

        let enhanced_context = format!(
            "{}

**市场特定信息：**
- 交易市场：{}
- 计价货币：{} ({})
- 市场时区：{}
- 主要指数：{}

**数据质量评估：**
- 财务指标数量：{}项
- 新闻分析数量：{}条
- 分析完整性：{}

**高级技术指标：**
- ATR (平均真实范围)：{:.4}
- 威廉指标：{:.2}
- CCI指标：{:.2}
- 随机指标K：{:.2}
- 随机指标D：{:.2}
- ADX趋势强度：{:.2}

**风险指标：**
- 贝塔系数：{:.2}
- 负债权益比：{:.2}
- 流动比率：{:.2}
- 利息覆盖率：{:.2}
- 风险等级：{}

**业绩预测：**
- 收入增长预测：{:.1}%
- 盈利增长预测：{:.1}%
- 目标价位：{:.2}
- 分析师评级：{}
- 预测周期：{}

**财务健康评分：**
- 盈利能力：{:.1}/100
- 流动性：{:.1}/100
- 偿债能力：{:.1}/100
- 运营效率：{:.1}/100
- 整体健康：{:.1}/100

{}
请确保分析逻辑清晰、数据支撑充分、结论明确可执行。",
            base_prompt,
            report.market,
            report.market.get_currency(),
            report.market.get_currency_name(),
            report.market.get_timezone(),
            report.market.get_market_indicators().join(", "),
            report.data_quality.financial_indicators_count,
            report.data_quality.total_news_count,
            report.data_quality.analysis_completeness,
            report.technical.atr,
            report.technical.williams_r,
            report.technical.cci,
            report.technical.stochastic_k,
            report.technical.stochastic_d,
            report.technical.adx,
            report.fundamental.risk_assessment.beta.unwrap_or(0.0),
            report.fundamental.risk_assessment.debt_to_equity.unwrap_or(0.0),
            report.fundamental.risk_assessment.current_ratio.unwrap_or(0.0),
            report.fundamental.risk_assessment.interest_coverage.unwrap_or(0.0),
            report.fundamental.risk_assessment.risk_level,
            report.fundamental.performance_forecasts.revenue_growth_forecast.unwrap_or(0.0),
            report.fundamental.performance_forecasts.earnings_growth_forecast.unwrap_or(0.0),
            report.fundamental.performance_forecasts.target_price.unwrap_or(0.0),
            report.fundamental.performance_forecasts.analyst_rating,
            report.fundamental.performance_forecasts.forecast_period,
            report.fundamental.financial_health.profitability_score,
            report.fundamental.financial_health.liquidity_score,
            report.fundamental.financial_health.solvency_score,
            report.fundamental.financial_health.efficiency_score,
            report.fundamental.financial_health.overall_health_score,
            depth_instructions
        );

        base_prompt + &enhanced_context
    }

    fn generate_enhanced_fallback_analysis(&self, report: &AnalysisReport, depth: &AnalysisDepth) -> String {
        match depth {
            AnalysisDepth::Basic => self.generate_basic_fallback_analysis(report),
            AnalysisDepth::Standard => self.generate_standard_fallback_analysis(report),
            AnalysisDepth::Comprehensive => self.generate_comprehensive_fallback_analysis(report),
            AnalysisDepth::Professional => self.generate_professional_fallback_analysis(report),
        }
    }

    fn generate_enhanced_fallback_analysis_static(_report: &AnalysisReport, depth: &AnalysisDepth) -> String {
        // Static version for use in async context
        match depth {
            AnalysisDepth::Basic => format!(
                "{}{}{}",
                Self::basic_analysis_template(),
                Self::standard_analysis_template(),
                Self::risk_analysis_template()
            ),
            AnalysisDepth::Standard => format!(
                "{}{}{}{}",
                Self::basic_analysis_template(),
                Self::standard_analysis_template(),
                Self::technical_analysis_template(),
                Self::risk_analysis_template()
            ),
            AnalysisDepth::Comprehensive => format!(
                "{}{}{}{}{}",
                Self::basic_analysis_template(),
                Self::standard_analysis_template(),
                Self::technical_analysis_template(),
                Self::fundamental_analysis_template(),
                Self::risk_analysis_template()
            ),
            AnalysisDepth::Professional => format!(
                "{}{}{}{}{}{}",
                Self::basic_analysis_template(),
                Self::standard_analysis_template(),
                Self::technical_analysis_template(),
                Self::fundamental_analysis_template(),
                Self::quantitative_analysis_template(),
                Self::risk_analysis_template()
            ),
        }
    }

    fn basic_analysis_template() -> String {
        "**基础分析：**
基于当前技术指标和基本面数据，该股票显示出{}的趋势。RSI为{:.1}，处于{}状态。MACD信号为{}，表明短期动能{}。".to_string()
    }

    fn standard_analysis_template() -> String {
        "**标准分析：**
从估值角度看，当前市盈率{:.2}倍，市净率{:.2}倍，{}行业平均水平。成交量比率{:.2}，显示市场参与度{}。".to_string()
    }

    fn technical_analysis_template() -> String {
        "**技术分析深度：**
布林带位置{:.2}，表明价格处于{}区域。ADX趋势强度{:.2}，显示趋势力度{}。随机指标显示短期超买超卖状态。".to_string()
    }

    fn fundamental_analysis_template() -> String {
        "**基本面深度分析：**
公司财务健康度评分为{:.1}/100，其中盈利能力{:.1}，流动性{:.1}。分析师预测未来{}增长潜力{}。风险等级为{}，需要关注{}。".to_string()
    }

    fn quantitative_analysis_template() -> String {
        "**量化分析：**
基于多因子模型评分，该股票在{}维度表现{}。贝塔系数{:.2}，显示{}系统性风险。最大回撤分析显示下行保护{}。".to_string()
    }

    fn risk_analysis_template() -> String {
        "**风险评估：**
主要风险包括：{}。建议止损价位设置在当前价位的{}以下。目标价位区间为{}，风险收益比约为{}。".to_string()
    }

    // These methods would be implemented with actual report data
    fn generate_basic_fallback_analysis(&self, report: &AnalysisReport) -> String {
        self.generate_detailed_fallback_analysis(report, false)
    }

    fn generate_standard_fallback_analysis(&self, report: &AnalysisReport) -> String {
        self.generate_detailed_fallback_analysis(report, true)
    }

    fn generate_comprehensive_fallback_analysis(&self, report: &AnalysisReport) -> String {
        self.generate_detailed_fallback_analysis(report, true)
    }

    fn generate_professional_fallback_analysis(&self, report: &AnalysisReport) -> String {
        self.generate_detailed_fallback_analysis(report, true)
    }

    fn generate_detailed_fallback_analysis(&self, report: &AnalysisReport, detailed: bool) -> String {
        let currency = report.market.get_currency();
        let market_name = report.market.get_market_name();
        
        let mut analysis = format!(
            "## 📊 综合评估\n\n基于技术面、基本面和市场情绪的综合分析，{}({})的综合得分为{:.1}分。\n\n- 技术面得分：{:.1}/100\n- 基本面得分：{:.1}/100  \n- 情绪面得分：{:.1}/100\n\n",
            report.stock_name,
            report.stock_code,
            report.scores.comprehensive,
            report.scores.technical,
            report.scores.fundamental,
            report.scores.sentiment
        );

        // 财务健康度分析
        if !report.fundamental.financial_indicators.is_empty() {
            analysis.push_str(&format!(
                "## 💰 财务健康度分析\n\n获取到{}项财务指标，主要指标如下：\n\n",
                report.fundamental.financial_indicators.len()
            ));

            let mut financial_details = String::new();
            for indicator in &report.fundamental.financial_indicators {
                match indicator.name.as_str() {
                    "流动比率" | "Current Ratio" => {
                        financial_details.push_str(&format!("- 流动比率: {:.2}\n", indicator.value));
                    },
                    "速动比率" | "Quick Ratio" => {
                        financial_details.push_str(&format!("- 速动比率: {:.2}\n", indicator.value));
                    },
                    "产权比率" | "Debt to Equity" => {
                        financial_details.push_str(&format!("- 产权比率: {:.2}\n", indicator.value));
                    },
                    "净利润率" | "Net Profit Margin" => {
                        financial_details.push_str(&format!("- 净利润率: {:.2}%\n", indicator.value));
                    },
                    "净资产收益率" | "ROE" => {
                        financial_details.push_str(&format!("- 净资产收益率: {:.2}%\n", indicator.value));
                    },
                    "市盈率" | "P/E Ratio" => {
                        financial_details.push_str(&format!("- 市盈率: {:.2}\n", indicator.value));
                    },
                    "市净率" | "P/B Ratio" => {
                        financial_details.push_str(&format!("- 市净率: {:.2}\n", indicator.value));
                    },
                    _ => {}
                }
            }

            if !financial_details.is_empty() {
                analysis.push_str(&financial_details);
                analysis.push_str("\n财务健康度评估：良好\n\n");
            }
        }

        // 技术面分析
        if detailed {
            analysis.push_str(&format!(
                "## 📈 技术面分析\n\n当前技术指标显示：\n- 均线趋势：{}\n- RSI指标：{:.1}\n- MACD信号：{}\n- 成交量状态：{}\n\n技术面评估：{}\n\n",
                report.technical.ma_trend,
                report.technical.rsi,
                report.technical.macd_signal,
                report.technical.volume_status,
                if report.scores.technical >= 60.0 { "偏强" } else if report.scores.technical >= 40.0 { "中性" } else { "偏弱" }
            ));
        }

        // 市场情绪分析
        if report.data_quality.total_news_count > 0 {
            analysis.push_str(&format!(
                "## 📰 市场情绪分析\n\n基于{}条新闻的分析：\n- 整体情绪：{}\n- 情绪得分：{:.3}\n- 置信度：{:.2}%\n\n",
                report.data_quality.total_news_count,
                if report.sentiment.overall_sentiment > 0.1 { "偏向积极" } else if report.sentiment.overall_sentiment < -0.1 { "偏向消极" } else { "中性" },
                report.sentiment.overall_sentiment,
                report.sentiment.confidence_score * 100.0
            ));

            if detailed {
                analysis.push_str("新闻分布：\n");
                if let Some(company_news) = report.sentiment.news_distribution.get("company") {
                    analysis.push_str(&format!("- 公司新闻：{}条\n", company_news));
                }
                if let Some(announcements) = report.sentiment.news_distribution.get("announcement") {
                    analysis.push_str(&format!("- 公司公告：{}条\n", announcements));
                }
                if let Some(research) = report.sentiment.news_distribution.get("research") {
                    analysis.push_str(&format!("- 研究报告：{}条\n", research));
                }
                analysis.push('\n');
            }
        }

        // 投资策略建议
        analysis.push_str(&format!(
            "## 🎯 投资策略建议\n\n**投资建议：{}**\n\n根据综合分析，建议如下：\n\n",
            report.recommendation
        ));

        // 根据评分给出具体建议
        if report.scores.comprehensive >= 70.0 {
            analysis.push_str("**积极关注**：该股票综合表现良好，具备投资价值。\n\n操作建议：\n- 买入时机：可考虑逢低布局\n- 止损位置：设置合理止损控制风险\n- 持有周期：中长期持有为主\n");
        } else if report.scores.comprehensive >= 50.0 {
            analysis.push_str("**持有观望**：当前风险收益比一般，建议等待更好时机。\n\n操作建议：\n- 买入时机：技术面突破关键位置时\n- 止损位置：跌破重要技术支撑\n- 持有周期：中长期为主\n");
        } else {
            analysis.push_str("**谨慎操作**：当前存在一定风险，建议谨慎对待。\n\n操作建议：\n- 买入时机：等待基本面改善信号\n- 止损位置：严格执行止损策略\n- 持有周期：短期交易为主\n");
        }

        // 添加风险提示
        analysis.push_str("\n## ⚠️ 风险提示\n\n");
        if report.scores.technical < 40.0 {
            analysis.push_str("- 技术面偏弱，注意短期波动风险\n");
        }
        if report.scores.fundamental < 50.0 {
            analysis.push_str("- 基本面有待改善，关注财务指标变化\n");
        }
        if report.sentiment.overall_sentiment < -0.2 {
            analysis.push_str("- 市场情绪偏消极，注意情绪面风险\n");
        }
        if report.price_info.volatility > 3.0 {
            analysis.push_str("- 股价波动较大，注意控制仓位\n");
        }

        // 添加新的分析维度
        if detailed {
            // 估值分析
            analysis.push_str("\n## 📊 估值分析\n\n");
            let mut pe_ratio = None;
            let mut pb_ratio = None;
            let mut roe = None;
            
            for indicator in &report.fundamental.financial_indicators {
                match indicator.name.as_str() {
                    "市盈率" | "P/E Ratio" => pe_ratio = Some(indicator.value),
                    "市净率" | "P/B Ratio" => pb_ratio = Some(indicator.value),
                    "净资产收益率" | "ROE" => roe = Some(indicator.value),
                    _ => {}
                }
            }
            
            if let (Some(pe), Some(pb), Some(r)) = (pe_ratio, pb_ratio, roe) {
                let peg = pe / r; // PEG比率
                analysis.push_str(&format!(
                    "- 市盈率 (P/E): {:.2}\n- 市净率 (P/B): {:.2}\n- 净资产收益率 (ROE): {:.2}%\n- PEG比率: {:.2}\n\n",
                    pe, pb, r, peg
                ));
                
                if peg > 0.0 && peg < 1.0 {
                    analysis.push_str("估值评估：相对低估，PEG比率显示较好的投资价值\n");
                } else if peg > 1.0 && peg < 2.0 {
                    analysis.push_str("估值评估：估值合理，处于行业平均水平\n");
                } else if peg > 2.0 {
                    analysis.push_str("估值评估：相对高估，PEG比率偏高\n");
                }
            }
            
            // 技术指标深度分析
            analysis.push_str("\n## 🔍 技术指标深度分析\n\n");
            
            // RSI分析
            let rsi_desc = if report.technical.rsi > 70.0 {
                "超买区域，短期回调风险"
            } else if report.technical.rsi > 50.0 {
                "强势区域，趋势向好"
            } else if report.technical.rsi > 30.0 {
                "弱势区域，可能企稳"
            } else {
                "超卖区域，反弹机会"
            };
            analysis.push_str(&format!("- RSI ({:.1}): {}\n", report.technical.rsi, rsi_desc));
            
            // MACD分析
            let macd_desc = match report.technical.macd_signal.as_str() {
                "看涨" => "MACD金叉，短期趋势向上",
                "看跌" => "MACD死叉，短期趋势向下",
                _ => "MACD震荡，趋势不明"
            };
            analysis.push_str(&format!("- MACD: {}\n", macd_desc));
            
            // 布林带分析
            if report.technical.bb_position > 0.8 {
                analysis.push_str("- 布林带: 接近上轨，短期压力较大\n");
            } else if report.technical.bb_position < 0.2 {
                analysis.push_str("- 布林带: 接近下轨，可能存在支撑\n");
            } else {
                analysis.push_str("- 布林带: 在中轨附近运行，趋势相对稳定\n");
            }
            
            // 成交量分析
            let volume_desc = match report.technical.volume_status.as_str() {
                "放量" => "成交量放大，市场活跃度提升",
                "缩量" => "成交量萎缩，市场关注度下降",
                _ => "成交量正常，市场平稳运行"
            };
            analysis.push_str(&format!("- 成交量: {}\n", volume_desc));
            
            // 趋势强度分析
            let trend_desc = match report.technical.trend_strength.as_str() {
                "强趋势" => "趋势强劲，适合趋势跟踪",
                "中等趋势" => "趋势中等，需要谨慎跟随",
                _ => "趋势较弱，震荡为主"
            };
            analysis.push_str(&format!("- 趋势强度: {} (ADX: {:.1})\n", trend_desc, report.technical.adx));
            
            // 市场微观结构分析
            analysis.push_str("\n## 📈 市场微观结构分析\n\n");
            analysis.push_str(&format!(
                "- 价格波动率: {:.2}%\n- 成交量比率: {:.2}\n- ATR (平均真实范围): {:.4}\n\n",
                report.price_info.volatility * 100.0,
                report.price_info.volume_ratio,
                report.technical.atr
            ));
            
            let volatility_desc = if report.price_info.volatility < 1.0 {
                "低波动，适合稳健型投资者"
            } else if report.price_info.volatility < 2.5 {
                "中等波动，风险收益平衡"
            } else {
                "高波动，适合风险承受能力较强的投资者"
            };
            analysis.push_str(&format!("波动特征: {}\n", volatility_desc));
            
            // 投资者行为分析
            analysis.push_str("\n## 👥 投资者行为分析\n\n");
            
            if report.scores.technical > 60.0 && report.price_info.volume_ratio > 1.2 {
                analysis.push_str("- 技术面强势且放量，可能存在机构资金介入\n");
            } else if report.scores.technical < 40.0 && report.price_info.volume_ratio > 1.2 {
                analysis.push_str("- 技术面弱势但放量，可能存在恐慌性抛售\n");
            } else if report.scores.technical > 60.0 && report.price_info.volume_ratio < 0.8 {
                analysis.push_str("- 技术面强势但缩量，上涨动能可能不足\n");
            } else {
                analysis.push_str("- 市场表现相对平静，投资者情绪稳定\n");
            }
            
            // 情绪与技术背离分析
            if (report.sentiment.overall_sentiment > 0.2 && report.scores.technical < 40.0) ||
               (report.sentiment.overall_sentiment < -0.2 && report.scores.technical > 60.0) {
                analysis.push_str("- 注意：市场情绪与技术面存在背离信号，需要谨慎对待\n");
            }
        }

        analysis
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled && !self.config.api_key.is_empty()
    }

    pub fn get_provider(&self) -> &str {
        &self.config.provider
    }

    pub fn get_model(&self) -> &str {
        self.config.model.as_ref()
            .map(|s| s.as_str())
            .unwrap_or("default")
    }

    pub fn update_config(&mut self, config: AIConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &AIConfig {
        &self.config
    }

    // Streaming analysis methods for different providers
    async fn stream_openai_analysis(
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamingChunk>,
        config: &AIConfig,
    ) {
        // Placeholder for OpenAI streaming implementation
        let fallback = format!("OpenAI streaming analysis for: {}", prompt);
        Self::send_fallback_in_chunks(tx, fallback).await;
    }

    async fn stream_claude_analysis(
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamingChunk>,
        config: &AIConfig,
    ) {
        // Placeholder for Claude streaming implementation
        let fallback = format!("Claude streaming analysis for: {}", prompt);
        Self::send_fallback_in_chunks(tx, fallback).await;
    }

    async fn stream_baidu_analysis(
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamingChunk>,
        config: &AIConfig,
    ) {
        // Placeholder for Baidu streaming implementation
        let fallback = format!("Baidu streaming analysis for: {}", prompt);
        Self::send_fallback_in_chunks(tx, fallback).await;
    }

    async fn stream_tencent_analysis(
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamingChunk>,
        config: &AIConfig,
    ) {
        // Placeholder for Tencent streaming implementation
        let fallback = format!("Tencent streaming analysis for: {}", prompt);
        Self::send_fallback_in_chunks(tx, fallback).await;
    }

    async fn stream_glm_analysis(
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamingChunk>,
        config: &AIConfig,
    ) {
        // Placeholder for GLM streaming implementation
        let fallback = format!("GLM streaming analysis for: {}", prompt);
        Self::send_fallback_in_chunks(tx, fallback).await;
    }

    async fn stream_qwen_analysis(
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamingChunk>,
        config: &AIConfig,
    ) {
        // Placeholder for Qwen streaming implementation
        let fallback = format!("Qwen streaming analysis for: {}", prompt);
        Self::send_fallback_in_chunks(tx, fallback).await;
    }

    async fn stream_kimi_analysis(
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamingChunk>,
        config: &AIConfig,
    ) {
        // Placeholder for Kimi streaming implementation
        let fallback = format!("Kimi streaming analysis for: {}", prompt);
        Self::send_fallback_in_chunks(tx, fallback).await;
    }

    async fn stream_ollama_analysis(
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamingChunk>,
        config: &AIConfig,
    ) {
        // Placeholder for Ollama streaming implementation
        let fallback = format!("Ollama streaming analysis for: {}", prompt);
        Self::send_fallback_in_chunks(tx, fallback).await;
    }

    pub fn get_supported_providers() -> Vec<String> {
        vec![
            "openai".to_string(),
            "claude".to_string(),
            "baidu".to_string(),
            "tencent".to_string(),
            "glm".to_string(),
            "qwen".to_string(),
            "kimi".to_string(),
            "ollama".to_string(),
            "custom".to_string(),
        ]
    }

    pub fn get_models_for_provider(provider: &str) -> Vec<String> {
        match provider {
            "openai" => vec![
                "gpt-3.5-turbo".to_string(),
                "gpt-4".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-4o".to_string(),
            ],
            "claude" => vec![
                "claude-3-sonnet-20240229".to_string(),
                "claude-3-opus-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ],
            "baidu" => vec![
                "ERNIE-Bot".to_string(),
                "ERNIE-Bot-turbo".to_string(),
                "ERNIE-Bot-4".to_string(),
            ],
            "tencent" => vec![
                "hunyuan-standard".to_string(),
                "hunyuan-pro".to_string(),
            ],
            "glm" => vec![
                "glm-4".to_string(),
                "glm-4v".to_string(),
                "glm-3-turbo".to_string(),
                "glm-4-flash".to_string(),
            ],
            "qwen" => vec![
                "qwen-turbo".to_string(),
                "qwen-plus".to_string(),
                "qwen-max".to_string(),
                "qwen-vl-plus".to_string(),
                "qwen-long".to_string(),
            ],
            "kimi" => vec![
                "kimi-8k".to_string(),
                "kimi-32k".to_string(),
                "kimi-128k".to_string(),
            ],
            "ollama" => vec![
                "llama2".to_string(),
                "mistral".to_string(),
                "codellama".to_string(),
                "llama3".to_string(),
                "qwen".to_string(),
                "glm4".to_string(),
            ],
            _ => vec!["default".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PriceInfo, TechnicalAnalysis, FundamentalData, SentimentAnalysis, AnalysisScores};

    #[tokio::test]
    async fn test_generate_fallback_analysis() {
        let config = AIConfig {
            provider: "openai".to_string(),
            api_key: "".to_string(),
            base_url: None,
            model: None,
            enabled: false,
            timeout_seconds: 30,
        };
        
        let service = AIService::new(config);
        
        let report = AnalysisReport {
            stock_code: "000001".to_string(),
            stock_name: "平安银行".to_string(),
            analysis_date: chrono::Utc::now(),
            price_info: PriceInfo {
                current_price: 12.34,
                price_change: 1.23,
                volume_ratio: 1.0,
                volatility: 2.0,
            },
            technical: TechnicalAnalysis {
                ma5: 12.0,
                ma10: 12.1,
                ma20: 12.2,
                ma60: 12.3,
                rsi: 55.5,
                macd_signal: "观望".to_string(),
                ma_trend: "上升".to_string(),
                bb_position: 0.5,
                volume_status: "正常".to_string(),
            },
            fundamental: FundamentalData {
                financial_indicators: vec![],
                valuation: HashMap::new(),
                industry: "银行".to_string(),
                sector: "金融".to_string(),
            },
            sentiment: SentimentAnalysis {
                overall_sentiment: 0.1,
                sentiment_trend: "中性".to_string(),
                confidence_score: 0.8,
                total_analyzed: 10,
                sentiment_by_type: HashMap::new(),
                news_distribution: HashMap::new(),
            },
            scores: AnalysisScores {
                technical: 75.0,
                fundamental: 80.0,
                sentiment: 70.0,
                comprehensive: 75.0,
            },
            recommendation: "持有".to_string(),
            ai_analysis: "测试分析".to_string(),
            data_quality: crate::models::DataQuality {
                financial_indicators_count: 5,
                total_news_count: 10,
                analysis_completeness: "完整".to_string(),
            },
        };
        
        let analysis = service.generate_fallback_analysis(&report);
        assert!(analysis.contains("平安银行"));
        assert!(analysis.contains("12.34元"));
    }

    #[tokio::test]
    async fn test_supported_providers() {
        let providers = AIService::get_supported_providers();
        assert!(providers.contains(&"openai".to_string()));
        assert!(providers.contains(&"claude".to_string()));
    }

    #[tokio::test]
    async fn test_models_for_provider() {
        let models = AIService::get_models_for_provider("openai");
        assert!(models.contains(&"gpt-3.5-turbo".to_string()));
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIProviderInfo {
    pub provider: String,
    pub name: String,
    pub description: String,
    pub base_url: String,
    pub models: Vec<String>,
}

pub fn get_ai_providers_info() -> Vec<AIProviderInfo> {
    vec![
        AIProviderInfo {
            provider: "openai".to_string(),
            name: "OpenAI".to_string(),
            description: "OpenAI GPT系列模型".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            models: AIService::get_models_for_provider("openai"),
        },
        AIProviderInfo {
            provider: "claude".to_string(),
            name: "Claude".to_string(),
            description: "Anthropic Claude系列模型".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            models: AIService::get_models_for_provider("claude"),
        },
        AIProviderInfo {
            provider: "baidu".to_string(),
            name: "百度文心".to_string(),
            description: "百度文心一言系列模型".to_string(),
            base_url: "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop".to_string(),
            models: AIService::get_models_for_provider("baidu"),
        },
        AIProviderInfo {
            provider: "tencent".to_string(),
            name: "腾讯混元".to_string(),
            description: "腾讯混元大模型".to_string(),
            base_url: "https://hunyuan.tencentcloudapi.com".to_string(),
            models: AIService::get_models_for_provider("tencent"),
        },
        AIProviderInfo {
            provider: "glm".to_string(),
            name: "智谱GLM".to_string(),
            description: "智谱GLM系列模型".to_string(),
            base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
            models: AIService::get_models_for_provider("glm"),
        },
        AIProviderInfo {
            provider: "qwen".to_string(),
            name: "阿里云通义千问".to_string(),
            description: "阿里云通义千问系列模型".to_string(),
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
            models: AIService::get_models_for_provider("qwen"),
        },
        AIProviderInfo {
            provider: "kimi".to_string(),
            name: "月之暗面Kimi".to_string(),
            description: "月之暗面Kimi系列模型".to_string(),
            base_url: "https://api.moonshot.cn/v1".to_string(),
            models: AIService::get_models_for_provider("kimi"),
        },
        AIProviderInfo {
            provider: "ollama".to_string(),
            name: "Ollama本地".to_string(),
            description: "本地Ollama部署模型".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            models: AIService::get_models_for_provider("ollama"),
        },
        AIProviderInfo {
            provider: "custom".to_string(),
            name: "自定义API".to_string(),
            description: "自定义OpenAI兼容API".to_string(),
            base_url: "http://localhost:8000/v1".to_string(),
            models: AIService::get_models_for_provider("custom"),
        },
    ]
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIConfigUpdate {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIConfigResponse {
    pub provider: String,
    pub model: Option<String>,
    pub enabled: bool,
    pub base_url: Option<String>,
    pub is_configured: bool,
    pub supported_providers: Vec<AIProviderInfo>,
}