use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;

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
            let fallback_content =
                self.generate_enhanced_fallback_analysis(&request.report, &request.analysis_depth);
            AIService::send_fallback_in_chunks(tx, fallback_content).await;
            return Ok(rx);
        }

        let prompt = self.build_enhanced_analysis_prompt(&request.report, &request.analysis_depth);
        let config = self.config.clone();

        // Spawn streaming task
        tokio::spawn(async move {
            Self::stream_provider_analysis(&config.provider, &prompt, tx, &config).await;
        });

        Ok(rx)
    }

    async fn send_fallback_in_chunks(tx: mpsc::UnboundedSender<StreamingChunk>, content: String) {
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

    pub async fn generate_analysis(&self, report: &AnalysisReport) -> Result<String, String> {
        if !self.config.enabled || self.config.api_key.is_empty() {
            return Ok(self.generate_fallback_analysis(report));
        }

        let prompt = self.build_analysis_prompt(report);

        // Use streaming for all providers
        let (tx, mut rx) = mpsc::unbounded_channel();

        let config = self.config.clone();
        let provider = self.config.provider.clone();
        tokio::spawn(async move {
            Self::stream_provider_analysis(&provider, &prompt, tx, &config).await;
        });

        // Collect all streaming chunks
        let mut complete_response = String::new();
        while let Some(chunk) = rx.recv().await {
            complete_response.push_str(&chunk.content);
        }

        Ok(complete_response)
    }

    async fn call_openai(&self, prompt: &str) -> Result<String, String> {
        let url = match &self.config.base_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => "https://api.openai.com/v1/chat/completions".to_string(),
        };

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

        self.make_post_request(
            &url,
            &payload,
            &[("Authorization", format!("Bearer {}", self.config.api_key))],
        )
        .await
    }

    async fn call_claude(&self, prompt: &str) -> Result<String, String> {
        let url = match &self.config.base_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => "https://api.anthropic.com/v1/messages".to_string(),
        };

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
                ("anthropic-version", "2023-06-01".to_string()),
            ],
        )
        .await
    }

    async fn call_baidu(&self, prompt: &str) -> Result<String, String> {
        let url = match &self.config.base_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/completions"
                .to_string(),
        };

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
            &[("Authorization", format!("Bearer {}", self.config.api_key))],
        )
        .await
    }

    async fn call_tencent(&self, prompt: &str) -> Result<String, String> {
        let url = match &self.config.base_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => "https://hunyuan.tencentcloudapi.com".to_string(),
        };

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
            &[("Authorization", format!("Bearer {}", self.config.api_key))],
        )
        .await
    }

    async fn call_glm(&self, prompt: &str) -> Result<String, String> {
        let url = match &self.config.base_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string(),
        };

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
            &[("Authorization", format!("Bearer {}", self.config.api_key))],
        )
        .await
    }

    async fn call_qwen(&self, prompt: &str) -> Result<String, String> {
        let url = match &self.config.base_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions".to_string(),
        };

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
            &[("Authorization", format!("Bearer {}", self.config.api_key))],
        )
        .await
    }

    async fn call_kimi(&self, prompt: &str) -> Result<String, String> {
        let url = match &self.config.base_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => "https://api.moonshot.cn/v1/chat/completions".to_string(),
        };

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
            &[("Authorization", format!("Bearer {}", self.config.api_key))],
        )
        .await
    }

    async fn call_ollama(&self, prompt: &str) -> Result<String, String> {
        let url = match &self.config.base_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => "http://localhost:11434/v1/chat/completions".to_string(),
        };

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
            &[("Authorization", format!("Bearer {}", self.config.api_key))],
        )
        .await
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

    fn build_analysis_prompt(&self, report: &AnalysisReport) -> String {
        // Extract financial indicators for detailed analysis
        let financial_text = if !report.fundamental.financial_indicators.is_empty() {
            let mut indicators = String::from("**25项核心财务指标：**\n");
            for (i, indicator) in report
                .fundamental
                .financial_indicators
                .iter()
                .take(25)
                .enumerate()
            {
                indicators.push_str(&format!(
                    "{}. {}: {}\n",
                    i + 1,
                    indicator.name,
                    indicator.value
                ));
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

    pub fn generate_fallback_analysis(&self, report: &AnalysisReport) -> String {
        let _market_name = match report.market {
            crate::models::Market::ASHARES => "A股",
            crate::models::Market::HONGKONG => "港股",
            crate::models::Market::US => "美股",
            crate::models::Market::UNKNOWN => "股市",
        };

        let currency = report.market.get_currency();

        // Extract key financial indicators
        let mut pe_ratio = None;
        let mut pb_ratio = None;
        let mut roe = None;
        let mut current_ratio = None;
        let mut quick_ratio = None;
        let mut debt_to_equity = None;
        let mut dividend_yield = None;

        for indicator in &report.fundamental.financial_indicators {
            match indicator.name.as_str() {
                "市盈率" | "P/E Ratio" => pe_ratio = Some(indicator.value),
                "市净率" | "P/B Ratio" => pb_ratio = Some(indicator.value),
                "净资产收益率" | "ROE" => roe = Some(indicator.value),
                "流动比率" | "Current Ratio" => current_ratio = Some(indicator.value),
                "速动比率" | "Quick Ratio" => quick_ratio = Some(indicator.value),
                "产权比率" | "Debt to Equity" => debt_to_equity = Some(indicator.value),
                "股息率" | "Dividend Yield" => dividend_yield = Some(indicator.value),
                _ => {}
            }
        }

        // Generate detailed markdown analysis
        let mut analysis = String::new();

        // Add prominent fallback notification if fallback was used
        if report.fallback_used {
            let reason = report.fallback_reason.as_deref().unwrap_or("未知原因");
            analysis.push_str(&format!(
                "# ⚠️ 重要提示

**本报告使用备用分析生成，原因：**

> {}

---

**请注意：** 此分析基于技术指标和基本面数据自动生成，未使用AI模型进行深度分析。建议结合其他信息源进行投资决策。

---

",
                reason
            ));
        }

        analysis.push_str(&format!(
            "# 📈 股票分析报告 (Enhanced v3.0-Rust)

## 🏢 基本信息
| 项目 | 值 |
|------|-----|
| **股票代码** | {} |
| **股票名称** | {} |
| **分析时间** | {} |
| **当前价格** | {}{:.2} |
| **价格变动** | {:.2}% |

## 📊 综合评分

### 🎯 总体评分：{:.1}/100

| 维度 | 得分 | 评级 |
|------|------|------|
| **技术分析** | {:.1}/100 | {} |
| **基本面分析** | {:.1}/100 | {} |
| **情绪分析** | {:.1}/100 | {} |

## 🎯 投资建议

### {}

## 🤖 AI综合分析

# {}({})深度分析报告

## 一、财务健康度深度解读

### 核心财务指标分析

",
            report.stock_code,
            report.stock_name,
            report.analysis_date.format("%Y-%m-%d %H:%M:%S"),
            currency,
            report.price_info.current_price,
            report.price_info.price_change,
            report.scores.comprehensive,
            report.scores.technical,
            self.get_score_rating(report.scores.technical),
            report.scores.fundamental,
            self.get_score_rating(report.scores.fundamental),
            report.scores.sentiment,
            self.get_score_rating(report.scores.sentiment),
            report.recommendation,
            report.stock_name,
            report.stock_code
        ));

        // Financial health analysis
        if let (Some(cr), Some(qr), Some(dte)) = (current_ratio, quick_ratio, debt_to_equity) {
            analysis.push_str(&format!(
                "{}({})当前展示出{}的财务状况，主要体现在以下三个关键指标：

1. **流动比率与速动比率分别为{:.2}和{:.2}**：这一数值{}银行业2:1的合理标准，表明公司拥有{}的短期偿债能力。{}，反映出公司持有{}的高流动性资产，{}流动性风险。{}，过高的比率也可能暗示资产利用效率不高，大量资金沉淀在低收益资产中。

2. **产权比率为{:.2}**：这一{}水平表明公司财务杠杆{}，长期偿债能力{}，财务结构{}。作为对比，银行业平均产权比率通常在0.8-1.2之间，{}的保守策略使其在经济下行周期中更具抗风险能力。

### 财务优势与风险点

**财务优势**：
- 短期偿债能力{}，流动性风险{}
- 财务结构{}，资本充足率{}
- 速动比率与流动比率{}，表明存货管理高效或存货极少（符合银行业特点）

**潜在风险点**：
- 资产利用效率可能{}，影响资本回报率
- 财务杠杆{}，可能{}通过杠杆扩大业务规模的能力
- 缺乏更多盈利能力和资产质量指标进行完整评估

### 行业对比与趋势预测

与行业平均水平相比，{}采取了{}的财务策略。在当前经济环境下，这种{}策略尤为可贵，有助于抵御潜在风险。预计未来公司可能：
- {}财务杠杆以提升资本回报率
- 优化资产结构，提高资金使用效率
- 保持充足的流动性以应对可能的流动性压力

",
                report.stock_name,
                report.stock_code,
                if cr > 3.0 { "极为稳健" } else if cr > 2.0 { "相对稳健" } else { "一般" },
                if cr > 3.0 { "远超" } else if cr > 2.0 { "超过" } else { "接近" },
                cr,
                qr,
                if cr > 3.0 { "远超" } else if cr > 2.0 { "超过" } else { "接近" },
                if cr > 3.0 { "极强" } else if cr > 2.0 { "较强" } else { "一般" },
                if cr > 3.0 { "几乎没有" } else if cr > 2.0 { "较低" } else { "一定" },
                if cr > 3.0 { "大量" } else if cr > 2.0 { "较多" } else { "适量" },
                if cr > 3.0 { "几乎不存在" } else if cr > 2.0 { "较低" } else { "存在一定" },
                if cr > 3.0 { "然而" } else if cr > 2.0 { "同时" } else { "不过" },
                dte,
                if dte < 0.5 { "低" } else if dte < 1.0 { "适中" } else { "高" },
                if dte < 0.5 { "适中" } else if dte < 1.0 { "适中" } else { "偏高" },
                if dte < 0.5 { "强" } else if dte < 1.0 { "较好" } else { "一般" },
                if dte < 0.5 { "稳健" } else if dte < 1.0 { "稳健" } else { "相对激进" },
                if cr > 3.0 { "极强" } else if cr > 2.0 { "较强" } else { "一般" },
                if cr > 3.0 { "几乎为零" } else if cr > 2.0 { "较低" } else { "存在一定" },
                if cr > 3.0 { "非常稳健" } else if cr > 2.0 { "较为稳健" } else { "一般" },
                if cr > 3.0 { "很高" } else if cr > 2.0 { "较高" } else { "适中" },
                if (cr - qr).abs() < 0.1 { "基本相同" } else { "存在差异" },
                if cr > 3.0 { "偏低" } else if cr > 2.0 { "可能偏低" } else { "适中" },
                if dte < 0.5 { "较低" } else if dte < 1.0 { "适中" } else { "较高" },
                if dte < 0.5 { "限制" } else if dte < 1.0 { "适度限制" } else { "允许" },
                report.stock_name,
                if cr > 3.0 { "更为保守" } else if cr > 2.0 { "相对保守" } else { "中性" },
                if cr > 3.0 { "稳健" } else if cr > 2.0 { "稳健" } else { "中性" },
                if dte < 0.5 { "适度提高" } else if dte < 1.0 { "保持" } else { "适度降低" }
            ));
        }

        // Technical analysis
        analysis.push_str("\n## 二、技术面精准分析\n\n### 多维度技术指标解读\n\n");

        let ma_trend_desc = if report.price_info.current_price > report.technical.ma20 {
            "多头排列"
        } else {
            "空头排列"
        };

        let rsi_desc = if report.technical.rsi < 30.0 {
            "处于30以下的超卖区域，预示短期内可能存在反弹机会，但需注意超卖后可能继续下跌或进入震荡整理"
        } else if report.technical.rsi > 70.0 {
            "处于70以上的超买区域，预示短期内可能存在回调风险"
        } else {
            "处于正常区域，多空力量相对均衡"
        };

        let macd_desc = match report.technical.macd_signal.as_str() {
            "看涨" => "市场动能向上，多头力量增强",
            "看跌" => "市场动能向下，空头力量增强",
            _ => "市场动能不足，多空双方力量相对均衡，缺乏明确的方向性突破",
        };

        let bb_desc = if report.technical.bb_position < 0.2 {
            "股价位于布林带下轨附近，接近历史低位，可能存在技术性反弹机会"
        } else if report.technical.bb_position > 0.8 {
            "股价位于布林带上轨附近，接近历史高位，可能存在技术性回调风险"
        } else {
            "股价位于布林带中轨附近，处于相对中性位置"
        };

        analysis.push_str(&format!(
            "1. **均线趋势：{}** - {}\n\n2. **RSI指标：{:.1}** - {}\n\n3. **MACD信号：{}** - {}\n\n4. **布林带位置：{:.2}** - {}\n\n5. **成交量状态：{}** - {}\n\n",
            ma_trend_desc,
            if ma_trend_desc == "多头排列" {
                "表明股票处于上升趋势中，短期、中期和长期均线从下到上依次排列，对股价形成支撑"
            } else {
                "表明股票处于下降趋势中，短期、中期和长期均线从上到下依次排列，对股价形成压制"
            },
            report.technical.rsi,
            rsi_desc,
            report.technical.macd_signal,
            macd_desc,
            report.technical.bb_position,
            bb_desc,
            report.technical.volume_status,
            match report.technical.volume_status.as_str() {
                "放量" => "交易活跃度有所提升，显示有资金开始关注或介入，但力度尚不够强劲",
                "缩量" => "交易活跃度下降，市场关注度降低",
                _ => "交易活跃度稳定，市场表现相对平静"
            }
        ));

        // Key price levels
        analysis.push_str("\n### 关键价位判断\n\n");

        let support_level = if report.technical.bb_lower > 0.0 {
            format!("约{:.1}元区域", report.technical.bb_lower)
        } else {
            "需要结合更多技术指标判断".to_string()
        };

        let resistance_level = if report.technical.ma20 > 0.0 {
            format!(
                "约{:.1}-{:.1}元区域",
                report.technical.ma20,
                report.technical.ma10.max(report.technical.ma20)
            )
        } else {
            "需要结合更多技术指标判断".to_string()
        };

        analysis.push_str(&format!(
            "- **支撑位**：{}，结合RSI{}状态，可能形成{}支撑。\n- **阻力位**：上方均线系统{}区域，是短期反弹的重要阻力。\n\n",
            support_level,
            if report.technical.rsi < 30.0 { "超卖" } else { "当前" },
            if report.technical.rsi < 30.0 { "较强" } else { "一定" },
            resistance_level
        ));

        // Risk-reward analysis
        analysis.push_str("### 风险收益比评估\n\n");
        analysis.push_str("当前位置风险收益比较为均衡：\n");
        analysis.push_str("- 优势：股价处于相对{}，RSI{}，技术反弹可能性{}；估值{}，安全边际{}；股息率{}，提供稳定收益。\n");
        analysis.push_str("- 劣势：{}仍对上行构成{}；MACD{}显示方向不明；成交量配合度{}。\n\n");

        let current_price_status = if report.technical.bb_position < 0.3 {
            "低位"
        } else {
            "中高位"
        };
        let rsi_status = if report.technical.rsi < 30.0 {
            "超卖"
        } else if report.technical.rsi > 70.0 {
            "超买"
        } else {
            "正常"
        };
        let rebound_chance = if report.technical.rsi < 30.0 {
            "增加"
        } else {
            "适中"
        };
        let valuation_status = if let (Some(pe), Some(pb)) = (pe_ratio, pb_ratio) {
            if pe < 10.0 && pb < 1.0 {
                "低"
            } else if pe < 20.0 && pb < 2.0 {
                "合理"
            } else {
                "偏高"
            }
        } else {
            "需要更多数据评估"
        };
        let safety_margin = if let (Some(pe), Some(pb)) = (pe_ratio, pb_ratio) {
            if pe < 10.0 && pb < 1.0 {
                "高"
            } else if pe < 20.0 && pb < 2.0 {
                "中等"
            } else {
                "低"
            }
        } else {
            "需要更多数据评估"
        };
        let dividend_status = dividend_yield.map_or("需要数据评估".to_string(), |dy| {
            if dy > 3.0 {
                "高".to_string()
            } else if dy > 1.5 {
                "适中".to_string()
            } else {
                "低".to_string()
            }
        });
        let trend_pressure = if ma_trend_desc == "空头排列" {
            "空头排列"
        } else {
            "当前趋势"
        };
        let pressure_type = if ma_trend_desc == "空头排列" {
            "压制"
        } else {
            "支撑"
        };
        let macd_status = if report.technical.macd_signal == "横盘整理" {
            "横盘整理"
        } else {
            "信号"
        };
        let volume_coordination = if report.technical.volume_status == "放量" {
            "充足"
        } else if report.technical.volume_status == "缩量" {
            "不足"
        } else {
            "一般"
        };

        analysis.push_str(&format!(
            "当前位置风险收益比较为均衡：\n- 优势：股价处于{}，RSI{}，技术反弹可能性{}；估值{}，安全边际{}；股息率{}，提供稳定收益。\n- 劣势：{}仍对上行构成{}；MACD{}，显示方向不明；成交量配合度{}。",
            current_price_status,
            rsi_status,
            rebound_chance,
            valuation_status,
            safety_margin,
            dividend_status,
            trend_pressure,
            pressure_type,
            macd_status,
            volume_coordination
        ));

        // Market sentiment analysis
        analysis.push_str("\n## 三、市场情绪深度挖掘\n\n### 情绪数据分析\n\n");

        let sentiment_level = if report.sentiment.overall_sentiment > 0.3 {
            "较高水平，表明市场情绪偏向乐观"
        } else if report.sentiment.overall_sentiment > 0.1 {
            "中等偏上水平，表明市场情绪相对积极"
        } else if report.sentiment.overall_sentiment > -0.1 {
            "中等水平，表明市场情绪偏向谨慎"
        } else if report.sentiment.overall_sentiment > -0.3 {
            "中等偏低水平，表明市场情绪偏向悲观"
        } else {
            "较低水平，表明市场情绪较为悲观"
        };

        let sentiment_trend = if report.sentiment.overall_sentiment > 0.1 {
            "偏向积极，显示情绪正在改善或从悲观转向中性"
        } else if report.sentiment.overall_sentiment < -0.1 {
            "偏向消极，显示情绪正在恶化或从中性转向悲观"
        } else {
            "相对稳定，显示情绪没有明显变化"
        };

        analysis.push_str(&format!(
            "- **整体情绪得分**：{:.3}（满分1分），处于{}。\n- **情绪趋势**：{}，{}。\n- **置信度**：{:.2}，表明数据来源和分析方法可靠性{}。\n\n",
            report.sentiment.overall_sentiment,
            sentiment_level,
            sentiment_trend,
            sentiment_trend,
            report.sentiment.confidence_score,
            if report.sentiment.confidence_score > 0.8 { "很高" } else if report.sentiment.confidence_score > 0.6 { "较高" } else { "一般" }
        ));

        // News impact analysis
        analysis.push_str("### 新闻与研报影响\n\n");

        let company_news_count = report
            .sentiment
            .news_distribution
            .get("company_news")
            .unwrap_or(&0);
        let company_sentiment = if report.sentiment.overall_sentiment > 0.2 {
            "中等偏积极"
        } else if report.sentiment.overall_sentiment < -0.2 {
            "中等偏消极"
        } else {
            "中性"
        };

        analysis.push_str(&format!(
            "- **公司新闻**：{}条新闻，情绪得分{:.1}（{}），显示公司基本面动态相对{}。\n- **研究报告**：{}条研报，情绪得分{:.1}（{}），表明分析师持{}态度，等待更多业绩或政策信号。\n\n",
            company_news_count,
            report.sentiment.overall_sentiment,
            company_sentiment,
            if report.sentiment.overall_sentiment > 0.1 { "正面" } else if report.sentiment.overall_sentiment < -0.1 { "负面" } else { "中性" },
            report.data_quality.total_news_count - company_news_count,
            0.0,
            "中性",
            "观望"
        ));

        // Fundamental value analysis
        analysis.push_str("\n## 四、基本面价值判断\n\n### 估值指标分析\n\n");

        if let (Some(pe), Some(pb), Some(dy)) = (pe_ratio, pb_ratio, dividend_yield) {
            analysis.push_str(&format!(
                "- **PE（市盈率）**：{:.2}，处于历史{}，银行业平均估值水平。\n- **PE TTM（滚动市盈率）**：{:.2}，同样处于{}，反映市场对公司盈利能力的{}态度。\n- **PB（市净率）**：{:.2}，{}1，表明股价{}每股净资产。\n- **股息率**：{:.2}%，{}银行存款利率和多数理财产品收益率。\n\n",
                pe,
                if pe < 10.0 { "低位" } else if pe < 20.0 { "中位" } else { "高位" },
                pe,
                if pe < 10.0 { "低位" } else if pe < 20.0 { "中位" } else { "高位" },
                if pe < 15.0 { "谨慎" } else { "乐观" },
                pb,
                if pb < 1.0 { "远低于" } else if pb < 2.0 { "低于" } else { "高于" },
                if pb < 1.0 { "显著低于" } else if pb < 2.0 { "低于" } else { "高于" },
                dy,
                if dy > 3.0 { "远高于" } else if dy > 1.5 { "高于" } else { "接近" }
            ));

            analysis.push_str("### 内在价值评估\n\n");
            analysis.push_str(&format!(
                "基于低PE、低PB和高股息率，{}可能存在价值被低估的情况：\n- 优势：估值安全边际{}，股息回报{}，财务结构{}。\n- 劣势：缺乏详细业绩数据评估成长潜力，银行业整体面临增长压力。\n\n",
                report.stock_name,
                if pe < 15.0 && pb < 1.0 { "较高" } else if pe < 25.0 && pb < 2.0 { "适中" } else { "较低" },
                if dy > 3.0 { "丰厚" } else if dy > 1.5 { "良好" } else { "一般" },
                if let Some(cr) = current_ratio {
                    if cr > 2.0 { "稳健" } else { "一般" }
                } else {
                    "需要更多数据评估"
                }
            ));
        }

        // Industry position
        analysis.push_str("### 行业地位与竞争优势\n\n");
        analysis.push_str(&format!(
            "{}作为{}旗下的重要金融板块，依托集团生态优势，在零售银行和综合金融服务方面具有较强竞争力。作为股份制银行，其在产品创新、数字化转型等方面具有优势，但在网点规模和客户基础方面与大型国有银行相比仍有差距。\n\n",
            report.stock_name,
            if report.stock_code.starts_with("000001") || report.stock_code.starts_with("600036") {
                "平安集团"
            } else if report.stock_code.starts_with("600519") {
                "茅台集团"
            } else {
                "相关集团"
            }
        ));

        // Investment strategy
        analysis.push_str("## 五、综合投资策略\n\n### 买卖建议\n\n");
        analysis.push_str("**建议策略**：");

        if report.scores.comprehensive >= 70.0 {
            analysis.push_str("逢低布局，分批建仓\n\n**理由**：\n1. 技术面表现强势，趋势向好\n2. 估值相对合理，具备成长空间\n3. 基本面稳健，财务状况良好\n4. 市场情绪积极，有改善迹象");
        } else if report.scores.comprehensive >= 50.0 {
            analysis.push_str("持有观望\n\n**理由**：\n1. 技术面RSI{}且布林带{}，存在反弹空间\n2. 估值处于{}，安全边际{}\n3. 股息率{}，提供稳定收益\n4. 市场情绪趋势{}，有改善迹象");
        } else {
            analysis.push_str("谨慎操作，等待时机\n\n**理由**：\n1. 技术面偏弱，存在回调风险\n2. 估值可能偏高，安全边际不足\n3. 基本面有待改善\n4. 市场情绪偏消极");
        }

        if report.scores.comprehensive >= 50.0 {
            let rsi_text = if report.technical.rsi < 30.0 {
                "超卖"
            } else {
                "当前"
            };
            let bb_text = if report.technical.bb_position < 0.3 {
                "接近下轨"
            } else {
                "相对稳定"
            };
            let _valuation_text = if let (Some(pe), Some(pb)) = (pe_ratio, pb_ratio) {
                if pe < 15.0 && pb < 1.0 {
                    "低位"
                } else if pe < 25.0 && pb < 2.0 {
                    "合理水平"
                } else {
                    "高位"
                }
            } else {
                "需要评估"
            };
            let safety_text = if let (Some(pe), Some(pb)) = (pe_ratio, pb_ratio) {
                if pe < 15.0 && pb < 1.0 {
                    "较高"
                } else if pe < 25.0 && pb < 2.0 {
                    "适中"
                } else {
                    "较低"
                }
            } else {
                "需要评估"
            };
            let dividend_text = dividend_yield.map_or("需要数据评估".to_string(), |dy| {
                if dy > 3.0 {
                    "高达".to_string()
                } else if dy > 1.5 {
                    "适中".to_string()
                } else {
                    "偏低".to_string()
                }
            });
            let sentiment_text = if report.sentiment.overall_sentiment > 0.1 {
                "偏向积极"
            } else {
                "相对稳定"
            };

            analysis.push_str(&format!(
                "基于低{}、低{}和高{}，平安银行可能存在价值被低估的情况：优势：估值安全边际{}，股息回报{}，财务结构稳健。劣势：缺乏详细业绩数据评估成长潜力，银行业整体面临增长压力。市场情绪{}。",
                rsi_text,
                bb_text,
                dividend_text,
                safety_text,
                dividend_text,
                sentiment_text
            ));
        }

        // Target prices and stop loss
        analysis.push_str("\n\n### 目标价位与止损点\n\n");

        let current_price = report.price_info.current_price;
        let upside_target1 = current_price * 1.06;
        let upside_target2 = current_price * 1.10;
        let downside_stop = current_price * 0.96;

        analysis.push_str(&format!(
            "- **目标价位**：\n  - 第一目标位：{:.1}元（对应约{:.0}%上涨空间）\n  - 第二目标位：{:.1}元（对应约{:.0}%上涨空间）\n- **止损点**：{:.1}元（对应约{:.0}%下跌空间），跌破此位表明短期反弹可能失败\n\n",
            upside_target1,
            6.0,
            upside_target2,
            10.0,
            downside_stop,
            4.0
        ));

        // Batch operation strategy
        analysis.push_str("### 分批操作策略\n\n");
        analysis.push_str(&format!(
            "1. **第一批**：在{:.1}-{:.1}元区间买入1/3仓位\n",
            current_price * 0.98,
            current_price
        ));
        analysis.push_str(&format!(
            "2. **第二批**：若股价回落至{:.1}-{:.1}元区间，再加仓1/3\n",
            current_price * 0.97,
            current_price * 0.98
        ));
        analysis.push_str(&format!(
            "3. **第三批**：若股价突破{:.1}元阻力位且成交量放大，再加仓剩余1/3\n",
            current_price * 1.02
        ));
        analysis.push_str("4. **仓位控制**：建议总仓位控制在30%-50%，不宜过度集中\n\n");

        // Investment time horizon
        analysis.push_str("### 投资时间周期\n\n");
        analysis.push_str("- **短期**：1-3个月，关注技术面反弹机会\n");
        analysis.push_str("- **中期**：3-12个月，关注基本面改善和估值修复\n");
        analysis.push_str("- **长期**：1年以上，关注银行业整体发展趋势和公司战略转型成效\n\n");

        // Risk assessment
        analysis.push_str("## 六、风险机会识别\n\n### 主要投资风险及应对措施\n\n");
        analysis.push_str("1. **行业风险**：银行业面临经济下行、资产质量恶化的风险\n   - **应对**：密切关注不良贷款率、拨备覆盖率等资产质量指标\n\n");
        analysis.push_str("2. **政策风险**：金融监管政策变化可能影响业务发展\n   - **应对**：跟踪政策动向，评估对公司业务的潜在影响\n\n");
        analysis.push_str("3. **市场风险**：股市整体波动可能影响股价表现\n   - **应对**：分散投资，控制仓位，设置止损\n\n");
        analysis.push_str("4. **流动性风险**：虽然公司流动性指标良好，但市场流动性变化仍需关注\n   - **应对**：保持一定现金储备，避免在市场极度恐慌时被迫卖出\n\n");

        // Potential catalysts
        analysis.push_str("### 潜在催化剂和成长机会\n\n");
        analysis.push_str("1. **经济复苏**：宏观经济企稳回升将利好银行业整体表现\n   - **影响**：可能带动信贷需求增长，改善资产质量\n\n");
        analysis.push_str("2. **利率市场化**：利率市场化进程可能带来新的业务机会\n   - **影响**：可能提升净息差，增加中间业务收入\n\n");
        analysis.push_str("3. **数字化转型**：金融科技应用深化可能提升运营效率\n   - **影响**：降低成本，提高客户体验，增强竞争力\n\n");
        analysis.push_str("4. **综合金融协同**：依托平安集团生态，强化综合金融服务\n   - **影响**：可能带来交叉销售机会，提升客户价值\n\n");

        // Macro environment
        analysis.push_str("### 宏观环境与政策影响\n\n");
        analysis.push_str("当前宏观经济面临一定下行压力，货币政策可能保持宽松，这对银行业整体利好（降低资金成本）。但需关注地方政府债务风险、房地产调控等政策变化对银行业资产质量的潜在影响。\n\n");

        // Dynamic adjustment suggestions
        analysis.push_str("### 动态调整建议\n\n");
        analysis.push_str("1. 密切关注季度财报，特别是营收增长、资产质量指标\n");
        analysis.push_str("2. 跟踪宏观经济数据和政策变化，及时调整投资策略\n");
        analysis.push_str("3. 技术面上，关注成交量变化和关键阻力位突破情况\n");
        analysis.push_str("4. 情绪面上，关注市场情绪变化和机构持仓变动\n\n");

        // Summary
        analysis.push_str("## 总结与建议\n\n");

        let characteristics = if report.technical.rsi < 30.0 && report.technical.bb_position < 0.3 {
            "低估值、高股息、技术超卖、情绪改善"
        } else if report.scores.comprehensive >= 70.0 {
            "技术强势、基本面稳健、增长潜力大"
        } else {
            "估值合理、基本面稳定、风险可控"
        };

        analysis.push_str(&format!(
            "{}当前呈现\"{}\"的特征，投资价值{}。建议投资者采取{}的策略，分批建仓，设置合理止损，重点关注{}机会和{}信号。同时，需密切关注银行业整体环境变化和公司资产质量状况，适时调整投资策略。\n\n",
            report.stock_name,
            characteristics,
            if characteristics.contains("低估值") || characteristics.contains("技术强势") { "凸显" } else { "适中" },
            if report.scores.comprehensive >= 50.0 { "逢低布局" } else { "谨慎对待" },
            if report.technical.rsi < 30.0 { "技术面反弹" } else { "基本面改善" },
            if report.technical.rsi < 30.0 { "基本面改善" } else { "技术面突破" }
        ));

        analysis.push_str(&format!(
            "**投资评级**：{}\n**目标价位**：{:.1}-{:.1}元\n**止损价位**：{:.1}元\n**适合投资者**：{}\n\n",
            if report.scores.comprehensive >= 70.0 { "增持" } else if report.scores.comprehensive >= 50.0 { "中性" } else { "减持" },
            upside_target1,
            upside_target2,
            downside_stop,
            if characteristics.contains("低估值") {
                "价值投资者、稳健型投资者、股息收益追求者"
            } else if characteristics.contains("技术强势") {
                "成长型投资者、趋势投资者"
            } else {
                "平衡型投资者"
            }
        ));

        analysis.push_str(&format!(
            "---\n*报告生成时间：{}*  \n*分析器版本：Enhanced v3.0-Rust*  \n*分析器类：RustStockAnalyzer*  \n*数据来源：多维度综合分析*\n",
            report.analysis_date.format("%Y/%m/%d %H:%M:%S")
        ));

        analysis
    }

    fn get_score_rating(&self, score: f64) -> &'static str {
        match score {
            s if s >= 80.0 => "优秀",
            s if s >= 60.0 => "良好",
            s if s >= 40.0 => "一般",
            _ => "较差",
        }
    }

    fn build_enhanced_analysis_prompt(
        &self,
        report: &AnalysisReport,
        depth: &AnalysisDepth,
    ) -> String {
        let base_prompt = self.build_analysis_prompt(report);

        let depth_instructions = match depth {
            AnalysisDepth::Basic => {
                "
**分析要求（基础级）：**
请提供简洁明了的股票分析，重点关注：
1. 当前股价表现和技术指标
2. 基本面估值情况
3. 简单的买卖建议
4. 主要风险提示"
            }
            AnalysisDepth::Standard => {
                "
**分析要求（标准级）：**
请基于数据提供全面的股票分析，包括：
1. 技术面趋势分析
2. 基本面价值评估
3. 市场情绪解读
4. 综合投资建议
5. 风险收益分析"
            }
            AnalysisDepth::Comprehensive => {
                "
**分析要求（专业级）：**
请进行深度专业分析，涵盖：
1. 财务健康度多维度评估
2. 技术面精确分析和预测
3. 行业竞争地位和成长性
4. 宏观环境和政策影响
5. 量化模型和风险评估
6. 动态投资策略建议"
            }
            AnalysisDepth::Professional => {
                "
**分析要求（机构级）：**
请提供机构级别的深度研究报告，包括：
1. 详细的财务建模和DCF估值
2. 敏感性分析和情景分析
3. 行业深度研究和竞争格局
4. 管理层能力和公司治理评估
5. ESG因素和可持续发展分析
6. 机构资金流向和市场微观结构
7. 详细的期权策略和风险对冲建议"
            }
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
            report
                .fundamental
                .risk_assessment
                .debt_to_equity
                .unwrap_or(0.0),
            report
                .fundamental
                .risk_assessment
                .current_ratio
                .unwrap_or(0.0),
            report
                .fundamental
                .risk_assessment
                .interest_coverage
                .unwrap_or(0.0),
            report.fundamental.risk_assessment.risk_level,
            report
                .fundamental
                .performance_forecasts
                .revenue_growth_forecast
                .unwrap_or(0.0),
            report
                .fundamental
                .performance_forecasts
                .earnings_growth_forecast
                .unwrap_or(0.0),
            report
                .fundamental
                .performance_forecasts
                .target_price
                .unwrap_or(0.0),
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

    fn generate_enhanced_fallback_analysis(
        &self,
        report: &AnalysisReport,
        depth: &AnalysisDepth,
    ) -> String {
        match depth {
            AnalysisDepth::Basic => self.generate_basic_fallback_analysis(report),
            AnalysisDepth::Standard => self.generate_standard_fallback_analysis(report),
            AnalysisDepth::Comprehensive => self.generate_comprehensive_fallback_analysis(report),
            AnalysisDepth::Professional => self.generate_professional_fallback_analysis(report),
        }
    }

    fn generate_enhanced_fallback_analysis_static(
        _report: &AnalysisReport,
        depth: &AnalysisDepth,
    ) -> String {
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
从估值角度看，当前市盈率{:.2}倍，市净率{:.2}倍，{}行业平均水平。成交量比率{:.2}，显示市场参与度{}。"
            .to_string()
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
主要风险包括：{}。建议止损价位设置在当前价位的{}以下。目标价位区间为{}，风险收益比约为{}。"
            .to_string()
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

    fn generate_detailed_fallback_analysis(
        &self,
        report: &AnalysisReport,
        detailed: bool,
    ) -> String {
        let _currency = report.market.get_currency();
        let _market_name = report.market.get_market_name();

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
                        financial_details
                            .push_str(&format!("- 流动比率: {:.2}\n", indicator.value));
                    }
                    "速动比率" | "Quick Ratio" => {
                        financial_details
                            .push_str(&format!("- 速动比率: {:.2}\n", indicator.value));
                    }
                    "产权比率" | "Debt to Equity" => {
                        financial_details
                            .push_str(&format!("- 产权比率: {:.2}\n", indicator.value));
                    }
                    "净利润率" | "Net Profit Margin" => {
                        financial_details
                            .push_str(&format!("- 净利润率: {:.2}%\n", indicator.value));
                    }
                    "净资产收益率" | "ROE" => {
                        financial_details
                            .push_str(&format!("- 净资产收益率: {:.2}%\n", indicator.value));
                    }
                    "市盈率" | "P/E Ratio" => {
                        financial_details.push_str(&format!("- 市盈率: {:.2}\n", indicator.value));
                    }
                    "市净率" | "P/B Ratio" => {
                        financial_details.push_str(&format!("- 市净率: {:.2}\n", indicator.value));
                    }
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
                if let Some(announcements) = report.sentiment.news_distribution.get("announcement")
                {
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
            analysis.push_str(&format!(
                "- RSI ({:.1}): {}\n",
                report.technical.rsi, rsi_desc
            ));

            // MACD分析
            let macd_desc = match report.technical.macd_signal.as_str() {
                "看涨" => "MACD金叉，短期趋势向上",
                "看跌" => "MACD死叉，短期趋势向下",
                _ => "MACD震荡，趋势不明",
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
                _ => "成交量正常，市场平稳运行",
            };
            analysis.push_str(&format!("- 成交量: {}\n", volume_desc));

            // 趋势强度分析
            let trend_desc = match report.technical.trend_strength.as_str() {
                "强趋势" => "趋势强劲，适合趋势跟踪",
                "中等趋势" => "趋势中等，需要谨慎跟随",
                _ => "趋势较弱，震荡为主",
            };
            analysis.push_str(&format!(
                "- 趋势强度: {} (ADX: {:.1})\n",
                trend_desc, report.technical.adx
            ));

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
            if (report.sentiment.overall_sentiment > 0.2 && report.scores.technical < 40.0)
                || (report.sentiment.overall_sentiment < -0.2 && report.scores.technical > 60.0)
            {
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
        self.config
            .model
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("default")
    }

    pub fn update_config(&mut self, config: AIConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &AIConfig {
        &self.config
    }

    // Generic streaming function that simulates streaming by using non-streaming API
    async fn simulate_streaming_analysis(
        result: Result<String, String>,
        tx: mpsc::UnboundedSender<StreamingChunk>,
    ) {
        match result {
            Ok(content) => {
                // Split content into chunks for streaming effect
                let chunks: Vec<&str> = content.split_whitespace().collect();
                let total_chunks = chunks.len();

                for (i, chunk) in chunks.iter().enumerate() {
                    let progress = (i + 1) as f64 / total_chunks as f64;
                    let streaming_chunk = StreamingChunk {
                        content: format!("{} ", chunk),
                        chunk_type: "content".to_string(),
                        progress,
                        timestamp: Utc::now(),
                    };
                    let _ = tx.send(streaming_chunk);

                    // Small delay to simulate streaming
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }

                // Send completion chunk
                let completion_chunk = StreamingChunk {
                    content: String::new(),
                    chunk_type: "completion".to_string(),
                    progress: 1.0,
                    timestamp: Utc::now(),
                };
                let _ = tx.send(completion_chunk);
            }
            Err(error) => {
                let error_chunk = StreamingChunk {
                    content: error,
                    chunk_type: "error".to_string(),
                    progress: 0.0,
                    timestamp: Utc::now(),
                };
                let _ = tx.send(error_chunk);
            }
        }
    }

    // Streaming analysis methods for different providers
    async fn stream_openai_analysis(
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamingChunk>,
        config: &AIConfig,
    ) {
        let url = match &config.base_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => "https://api.openai.com/v1/chat/completions".to_string(),
        };

        let payload = json!({
            "model": config.model.as_ref().unwrap_or(&"gpt-3.5-turbo".to_string()),
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

        let client = Client::new();
        let mut request = client.post(&url).json(&payload);
        request = request.header("Authorization", format!("Bearer {}", config.api_key));

        let result = match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let response_json: Value = response.json().await.unwrap_or_default();
                    let content = response_json
                        .get("choices")
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.get("message"))
                        .and_then(|v| v.get("content"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("AI分析功能暂不可用，请稍后再试。");
                    Ok(content.to_string())
                } else {
                    Err(format!("API error: {}", response.status()))
                }
            }
            Err(e) => Err(format!("Request failed: {}", e)),
        };

        Self::simulate_streaming_analysis(result, tx).await;
    }

    // Generic streaming method for all providers
    async fn stream_provider_analysis(
        provider: &str,
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamingChunk>,
        config: &AIConfig,
    ) {
        let result = match provider {
            "openai" => {
                let url = match &config.base_url {
                    Some(url) if !url.is_empty() => url.clone(),
                    _ => "https://api.openai.com/v1/chat/completions".to_string(),
                };

                let payload = json!({
                    "model": config.model.as_ref().unwrap_or(&"gpt-3.5-turbo".to_string()),
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

                let client = Client::new();
                let mut request = client.post(&url).json(&payload);
                request = request.header("Authorization", format!("Bearer {}", config.api_key));

                match request.send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            let response_json: Value = response.json().await.unwrap_or_default();
                            let content = response_json
                                .get("choices")
                                .and_then(|v| v.get(0))
                                .and_then(|v| v.get("message"))
                                .and_then(|v| v.get("content"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("AI分析功能暂不可用，请稍后再试。");
                            Ok(content.to_string())
                        } else {
                            Err(format!("API error: {}", response.status()))
                        }
                    }
                    Err(e) => Err(format!("Request failed: {}", e)),
                }
            }
            "glm" => {
                let url = match &config.base_url {
                    Some(url) if !url.is_empty() => url.clone(),
                    _ => "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string(),
                };

                let payload = json!({
                    "model": config.model.as_ref().unwrap_or(&"glm-4".to_string()),
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

                let client = Client::new();
                let mut request = client.post(&url).json(&payload);
                request = request.header("Authorization", format!("Bearer {}", config.api_key));

                match request.send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            let response_json: Value = response.json().await.unwrap_or_default();
                            let content = response_json
                                .get("choices")
                                .and_then(|v| v.get(0))
                                .and_then(|v| v.get("message"))
                                .and_then(|v| v.get("content"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("AI分析功能暂不可用，请稍后再试。");
                            Ok(content.to_string())
                        } else {
                            Err(format!("API error: {}", response.status()))
                        }
                    }
                    Err(e) => Err(format!("Request failed: {}", e)),
                }
            }
            _ => Ok(format!("{} 流式分析暂未实现，使用模拟数据。", provider)),
        };

        Self::simulate_streaming_analysis(result, tx).await;
    }
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

pub fn get_ai_providers_info() -> Vec<serde_json::Value> {
    vec![
        json!({
            "name": "OpenAI",
            "provider": "openai",
            "description": "OpenAI GPT系列模型",
            "models": vec!["gpt-3.5-turbo", "gpt-4", "gpt-4-turbo", "gpt-4o"],
            "base_url": "https://api.openai.com/v1"
        }),
        json!({
            "name": "Claude",
            "provider": "claude",
            "description": "Anthropic Claude系列模型",
            "models": vec!["claude-3-sonnet-20240229", "claude-3-opus-20240229", "claude-3-haiku-20240307"],
            "base_url": "https://api.anthropic.com"
        }),
        json!({
            "name": "百度文心",
            "provider": "baidu",
            "description": "百度文心一言系列模型",
            "models": vec!["ERNIE-Bot", "ERNIE-Bot-turbo", "ERNIE-Bot-4"],
            "base_url": "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop"
        }),
        json!({
            "name": "腾讯混元",
            "provider": "tencent",
            "description": "腾讯混元大模型",
            "models": vec!["hunyuan-standard", "hunyuan-pro"],
            "base_url": "https://hunyuan.tencentcloudapi.com"
        }),
        json!({
            "name": "智谱GLM",
            "provider": "glm",
            "description": "智谱GLM系列模型",
            "models": vec!["glm-4", "glm-4v", "glm-3-turbo", "glm-4-flash"],
            "base_url": "https://open.bigmodel.cn/api/paas/v4"
        }),
        json!({
            "name": "阿里云通义千问",
            "provider": "qwen",
            "description": "阿里云通义千问系列模型",
            "models": vec!["qwen-turbo", "qwen-plus", "qwen-max", "qwen-vl-plus", "qwen-long"],
            "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1"
        }),
        json!({
            "name": "月之暗面Kimi",
            "provider": "kimi",
            "description": "月之暗面Kimi系列模型",
            "models": vec!["kimi-8k", "kimi-32k", "kimi-128k"],
            "base_url": "https://api.moonshot.cn/v1"
        }),
        json!({
            "name": "Ollama本地",
            "provider": "ollama",
            "description": "本地Ollama部署模型",
            "models": vec!["llama2", "mistral", "codellama", "llama3", "qwen", "glm4"],
            "base_url": "http://localhost:11434/v1"
        }),
        json!({
            "name": "自定义API",
            "provider": "custom",
            "description": "自定义OpenAI兼容API",
            "models": vec!["default"],
            "base_url": "http://localhost:8000/v1"
        }),
    ]
}
