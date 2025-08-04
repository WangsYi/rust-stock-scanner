use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use crate::cache::CachedDataFetcher;
use crate::models::Market;
use crate::models::*;

// Rate limiter for API calls (max 10 requests per second)
pub struct RateLimiter {
    request_times: Arc<tokio::sync::Mutex<Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(_max_requests: usize) -> Self {
        Self {
            request_times: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    pub async fn acquire(&self) -> RateLimiterPermit {
        let mut times = self.request_times.lock().await;
        let now = Instant::now();

        // Clean up old requests (older than 1 second)
        times.retain(|&time| now.duration_since(time) < StdDuration::from_secs(1));

        // If we have too many recent requests, wait
        if times.len() >= 10 {
            if let Some(&oldest_time) = times.first() {
                let wait_time =
                    StdDuration::from_secs(1).saturating_sub(now.duration_since(oldest_time));
                if wait_time > StdDuration::from_millis(0) {
                    drop(times);
                    tokio::time::sleep(wait_time).await;
                    times = self.request_times.lock().await;
                }
            }
        }

        // Record this request time
        times.push(now);

        RateLimiterPermit
    }
}

pub struct RateLimiterPermit;

#[async_trait::async_trait]
pub trait DataFetcher: Send + Sync {
    async fn get_stock_data(&self, stock_code: &str, days: i32) -> Result<Vec<PriceData>, String>;
    async fn get_fundamental_data(&self, stock_code: &str) -> Result<FundamentalData, String>;
    async fn get_news_data(
        &self,
        stock_code: &str,
        days: i32,
    ) -> Result<(Vec<News>, SentimentAnalysis), String>;
    async fn get_stock_name(&self, stock_code: &str) -> String;

    // New method for concurrent data fetching
    async fn get_all_data_concurrent(
        &self,
        stock_code: &str,
        days: i32,
    ) -> Result<
        (
            Vec<PriceData>,
            FundamentalData,
            (Vec<News>, SentimentAnalysis),
            String,
        ),
        String,
    > {
        let stock_code_clone = stock_code.to_string();

        // Spawn all three requests concurrently
        let price_future = tokio::spawn({
            let fetcher = self.clone();
            async move { fetcher.get_stock_data(&stock_code_clone, days).await }
        });

        let fundamental_future = tokio::spawn({
            let stock_code_clone = stock_code.to_string();
            let fetcher = self.clone();
            async move { fetcher.get_fundamental_data(&stock_code_clone).await }
        });

        let news_future = tokio::spawn({
            let stock_code_clone = stock_code.to_string();
            let fetcher = self.clone();
            async move { fetcher.get_news_data(&stock_code_clone, days).await }
        });

        let name_future = tokio::spawn({
            let stock_code_clone = stock_code.to_string();
            let fetcher = self.clone();
            async move { fetcher.get_stock_name(&stock_code_clone).await }
        });

        // Wait for all results
        let (price_result, fundamental_result, news_result, name_result) =
            tokio::join!(price_future, fundamental_future, news_future, name_future);

        let price_data = price_result.map_err(|e| format!("Price task failed: {}", e))??;
        let fundamental_data =
            fundamental_result.map_err(|e| format!("Fundamental task failed: {}", e))??;
        let news_data = news_result.map_err(|e| format!("News task failed: {}", e))??;
        let stock_name = name_result.map_err(|e| format!("Name task failed: {}", e))?;

        Ok((price_data, fundamental_data, news_data, stock_name))
    }

    // Helper method for cloning
    fn clone(&self) -> Box<dyn DataFetcher>;
}

pub struct AkshareProxy {
    client: Client,
    base_url: String,
    timeout: std::time::Duration,
    rate_limiter: Arc<RateLimiter>,
}

impl AkshareProxy {
    pub fn new(base_url: String, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url,
            timeout: std::time::Duration::from_secs(timeout_secs),
            rate_limiter: Arc::new(RateLimiter::new(10)), // Max 10 requests per second
        }
    }

    async fn make_request(&self, endpoint: &str) -> Result<Value, String> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await;

        let url = format!("{}/{}", self.base_url, endpoint);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| format!("JSON parse failed: {}", e))
    }
}

impl Clone for AkshareProxy {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            timeout: self.timeout,
            rate_limiter: self.rate_limiter.clone(),
        }
    }
}

#[async_trait::async_trait]
impl DataFetcher for AkshareProxy {
    async fn get_stock_data(&self, stock_code: &str, days: i32) -> Result<Vec<PriceData>, String> {
        let market = Market::from_stock_code(stock_code);
        let endpoint = match market {
            Market::ASHARES => format!("api/stock/{}/price?days={}", stock_code, days),
            Market::HONGKONG => format!("api/stock/hk/{}/price?days={}", stock_code, days),
            Market::US => format!("api/stock/us/{}/price?days={}", stock_code, days),
            Market::UNKNOWN => format!("api/stock/{}/price?days={}", stock_code, days),
        };

        match self.make_request(&endpoint).await {
            Ok(data) => {
                let mut prices = Vec::new();

                if let Some(items) = data.as_array() {
                    for item in items {
                        let date_str = item["date"].as_str().unwrap_or("");
                        let date = DateTime::parse_from_str(date_str, "%Y-%m-%d")
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now());

                        prices.push(PriceData {
                            date,
                            open: item["open"].as_f64().unwrap_or(0.0),
                            close: item["close"].as_f64().unwrap_or(0.0),
                            high: item["high"].as_f64().unwrap_or(0.0),
                            low: item["low"].as_f64().unwrap_or(0.0),
                            volume: item["volume"].as_i64().unwrap_or(0),
                            change_pct: 0.0,  // Will be calculated
                            turnover: 0.0,    // Will be calculated
                            turnover_rt: 0.0, // Will be calculated
                        });
                    }

                    // Sort by date ascending
                    prices.sort_by(|a, b| a.date.cmp(&b.date));

                    // Calculate additional fields
                    let mut change_pcts = Vec::with_capacity(prices.len());
                    let mut turnovers = Vec::with_capacity(prices.len());
                    let mut turnover_rts = Vec::with_capacity(prices.len());

                    for i in 0..prices.len() {
                        if i > 0 {
                            let prev_close = prices[i - 1].close;
                            if prev_close > 0.0 {
                                change_pcts
                                    .push(((prices[i].close - prev_close) / prev_close) * 100.0);
                            } else {
                                change_pcts.push(0.0);
                            }
                        } else {
                            change_pcts.push(0.0);
                        }
                        turnovers.push(prices[i].volume as f64 * prices[i].close);
                        turnover_rts.push(prices[i].volume as f64 / 100_000_000.0);
                    }

                    // Apply calculated values
                    for (i, price) in prices.iter_mut().enumerate() {
                        price.change_pct = change_pcts[i];
                        price.turnover = turnovers[i];
                        price.turnover_rt = turnover_rts[i];
                    }
                }

                Ok(prices)
            }
            Err(_) => {
                // Fallback to mock data
                self.get_mock_stock_data(stock_code, days, &market)
            }
        }
    }

    async fn get_fundamental_data(&self, stock_code: &str) -> Result<FundamentalData, String> {
        let market = Market::from_stock_code(stock_code);
        let endpoint = match market {
            Market::ASHARES => format!("api/stock/{}/fundamental", stock_code),
            Market::HONGKONG => format!("api/stock/hk/{}/fundamental", stock_code),
            Market::US => format!("api/stock/us/{}/fundamental", stock_code),
            Market::UNKNOWN => format!("api/stock/{}/fundamental", stock_code),
        };

        match self.make_request(&endpoint).await {
            Ok(data) => {
                let mut indicators = Vec::new();

                if let Some(indicator_array) = data["financial_indicators"].as_array() {
                    for indicator in indicator_array {
                        indicators.push(FinancialIndicator {
                            name: indicator["name"].as_str().unwrap_or("").to_string(),
                            value: indicator["value"].as_f64().unwrap_or(0.0),
                            unit: indicator["unit"].as_str().unwrap_or("").to_string(),
                        });
                    }
                }

                let mut valuation = std::collections::HashMap::new();
                if let Some(val) = data["valuation"].as_object() {
                    for (k, v) in val {
                        if let Some(num) = v.as_f64() {
                            valuation.insert(k.clone(), num);
                        }
                    }
                }

                // Enhanced fundamental data
                let performance_forecasts = PerformanceForecasts {
                    revenue_growth_forecast: data["performance_forecasts"]
                        ["revenue_growth_forecast"]
                        .as_f64(),
                    earnings_growth_forecast: data["performance_forecasts"]
                        ["earnings_growth_forecast"]
                        .as_f64(),
                    target_price: data["performance_forecasts"]["target_price"].as_f64(),
                    analyst_rating: data["performance_forecasts"]["analyst_rating"]
                        .as_str()
                        .unwrap_or("未评级")
                        .to_string(),
                    forecast_period: data["performance_forecasts"]["forecast_period"]
                        .as_str()
                        .unwrap_or("12个月")
                        .to_string(),
                };

                let risk_assessment = RiskAssessment {
                    beta: data["risk_assessment"]["beta"].as_f64(),
                    debt_to_equity: data["risk_assessment"]["debt_to_equity"].as_f64(),
                    current_ratio: data["risk_assessment"]["current_ratio"].as_f64(),
                    quick_ratio: data["risk_assessment"]["quick_ratio"].as_f64(),
                    interest_coverage: data["risk_assessment"]["interest_coverage"].as_f64(),
                    risk_level: data["risk_assessment"]["risk_level"]
                        .as_str()
                        .unwrap_or("中等")
                        .to_string(),
                };

                let financial_health = FinancialHealth {
                    profitability_score: data["financial_health"]["profitability_score"]
                        .as_f64()
                        .unwrap_or(50.0),
                    liquidity_score: data["financial_health"]["liquidity_score"]
                        .as_f64()
                        .unwrap_or(50.0),
                    solvency_score: data["financial_health"]["solvency_score"]
                        .as_f64()
                        .unwrap_or(50.0),
                    efficiency_score: data["financial_health"]["efficiency_score"]
                        .as_f64()
                        .unwrap_or(50.0),
                    overall_health_score: data["financial_health"]["overall_health_score"]
                        .as_f64()
                        .unwrap_or(50.0),
                };

                Ok(FundamentalData {
                    financial_indicators: indicators,
                    valuation,
                    industry: data["industry"].as_str().unwrap_or("未知").to_string(),
                    sector: data["sector"].as_str().unwrap_or("未知").to_string(),
                    performance_forecasts,
                    risk_assessment,
                    financial_health,
                })
            }
            Err(_) => {
                // Fallback to mock data
                self.get_mock_fundamental_data(stock_code, &market)
            }
        }
    }

    async fn get_news_data(
        &self,
        stock_code: &str,
        days: i32,
    ) -> Result<(Vec<News>, SentimentAnalysis), String> {
        let market = Market::from_stock_code(stock_code);
        let endpoint = match market {
            Market::ASHARES => format!("api/stock/{}/news?days={}", stock_code, days),
            Market::HONGKONG => format!("api/stock/hk/{}/news?days={}", stock_code, days),
            Market::US => format!("api/stock/us/{}/news?days={}", stock_code, days),
            Market::UNKNOWN => format!("api/stock/{}/news?days={}", stock_code, days),
        };

        match self.make_request(&endpoint).await {
            Ok(data) => {
                let mut news = Vec::new();

                if let Some(news_array) = data["news"].as_array() {
                    for item in news_array {
                        let date_str = item["date"].as_str().unwrap_or("");
                        let date = DateTime::parse_from_str(date_str, "%Y-%m-%d")
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now());

                        news.push(News {
                            title: item["title"].as_str().unwrap_or("").to_string(),
                            content: item["content"].as_str().unwrap_or("").to_string(),
                            date,
                            source: item["source"].as_str().unwrap_or("未知").to_string(),
                            news_type: item["type"].as_str().unwrap_or("company_news").to_string(),
                            relevance: item["relevance"].as_f64().unwrap_or(0.8),
                            sentiment: item["sentiment"].as_f64().unwrap_or(0.0),
                        });
                    }
                }

                let sentiment_data = &data["sentiment"];
                let mut sentiment_by_type = HashMap::new();
                let mut news_distribution = HashMap::new();

                if let Some(map) = sentiment_data["sentiment_by_type"].as_object() {
                    for (k, v) in map {
                        if let Some(num) = v.as_f64() {
                            sentiment_by_type.insert(k.clone(), num);
                        }
                    }
                }

                if let Some(map) = sentiment_data["news_distribution"].as_object() {
                    for (k, v) in map {
                        if let Some(num) = v.as_i64() {
                            news_distribution.insert(k.clone(), num as i32);
                        }
                    }
                }

                let sentiment_analysis = SentimentAnalysis {
                    overall_sentiment: sentiment_data["overall_sentiment"].as_f64().unwrap_or(0.0),
                    sentiment_trend: sentiment_data["sentiment_trend"]
                        .as_str()
                        .unwrap_or("中性")
                        .to_string(),
                    confidence_score: sentiment_data["confidence_score"].as_f64().unwrap_or(0.75),
                    total_analyzed: sentiment_data["total_analyzed"].as_i64().unwrap_or(0) as i32,
                    sentiment_by_type,
                    news_distribution,
                };

                Ok((news, sentiment_analysis))
            }
            Err(_) => {
                // Fallback to mock data
                self.get_mock_news_data(stock_code, days, &market)
            }
        }
    }

    async fn get_stock_name(&self, stock_code: &str) -> String {
        let endpoint = format!("api/stock/{}/name", stock_code);

        match self.make_request(&endpoint).await {
            Ok(data) => data["name"].as_str().unwrap_or("").to_string(),
            Err(_) => format!("{}股票", stock_code),
        }
    }

    fn clone(&self) -> Box<dyn DataFetcher> {
        Box::new(Clone::clone(self))
    }
}

impl AkshareProxy {
    fn get_mock_stock_data(
        &self,
        stock_code: &str,
        days: i32,
        market: &Market,
    ) -> Result<Vec<PriceData>, String> {
        let _rng = rand::thread_rng();

        // Market-specific base price ranges
        let base_price = match market {
            Market::ASHARES => {
                10.0 + (stock_code.chars().map(|c| c as u32).sum::<u32>() % 100) as f64
            }
            Market::HONGKONG => {
                50.0 + (stock_code.chars().map(|c| c as u32).sum::<u32>() % 200) as f64
            }
            Market::US => 100.0 + (stock_code.chars().map(|c| c as u32).sum::<u32>() % 400) as f64,
            Market::UNKNOWN => {
                50.0 + (stock_code.chars().map(|c| c as u32).sum::<u32>() % 100) as f64
            }
        };

        let mut prices = Vec::new();
        let mut current_price = base_price;

        for i in (0..days).rev() {
            let date = Utc::now() - Duration::days(i as i64);

            // Market-specific volatility
            let volatility_factor = match market {
                Market::ASHARES => 0.1,
                Market::HONGKONG => 0.15,
                Market::US => 0.08,
                Market::UNKNOWN => 0.12,
            };

            let change = (rand::random::<f64>() - 0.5) * volatility_factor;
            let open = current_price * (1.0 + (rand::random::<f64>() - 0.5) * 0.02);
            let close = open * (1.0 + change);
            let high = open.max(close) * (1.0 + rand::random::<f64>() * 0.03);
            let low = open.min(close) * (1.0 - rand::random::<f64>() * 0.03);

            // Market-specific volume ranges
            let volume = match market {
                Market::ASHARES => 1_000_000 + rand::random::<i64>().rem_euclid(5_000_000),
                Market::HONGKONG => 500_000 + rand::random::<i64>().rem_euclid(2_000_000),
                Market::US => 100_000 + rand::random::<i64>().rem_euclid(1_000_000),
                Market::UNKNOWN => 500_000 + rand::random::<i64>().rem_euclid(2_000_000),
            };

            prices.push(PriceData {
                date,
                open,
                close,
                high,
                low,
                volume,
                change_pct: ((close - open) / open) * 100.0,
                turnover: volume as f64 * close,
                turnover_rt: volume as f64 / 100_000_000.0,
            });

            current_price = close;
        }

        Ok(prices)
    }

    fn get_mock_fundamental_data(
        &self,
        stock_code: &str,
        market: &Market,
    ) -> Result<FundamentalData, String> {
        let hash = stock_code.chars().map(|c| c as u32).sum::<u32>();

        let (indicators, industry, sector) = match market {
            Market::ASHARES => {
                let indicators = vec![
                    FinancialIndicator {
                        name: "净利润率".to_string(),
                        value: 15.2 + (hash % 20) as f64,
                        unit: "%".to_string(),
                    },
                    FinancialIndicator {
                        name: "净资产收益率".to_string(),
                        value: 12.5 + (hash % 15) as f64,
                        unit: "%".to_string(),
                    },
                    FinancialIndicator {
                        name: "市盈率".to_string(),
                        value: 25.3 + (hash % 15) as f64,
                        unit: "倍".to_string(),
                    },
                    FinancialIndicator {
                        name: "市净率".to_string(),
                        value: 3.2 + ((hash % 5) as f64) / 10.0,
                        unit: "倍".to_string(),
                    },
                ];
                (indicators, "科技".to_string(), "信息技术".to_string())
            }
            Market::HONGKONG => {
                let indicators = vec![
                    FinancialIndicator {
                        name: "Net Profit Margin".to_string(),
                        value: 18.5 + (hash % 25) as f64,
                        unit: "%".to_string(),
                    },
                    FinancialIndicator {
                        name: "ROE".to_string(),
                        value: 14.2 + (hash % 12) as f64,
                        unit: "%".to_string(),
                    },
                    FinancialIndicator {
                        name: "P/E Ratio".to_string(),
                        value: 18.7 + (hash % 10) as f64,
                        unit: "x".to_string(),
                    },
                    FinancialIndicator {
                        name: "P/B Ratio".to_string(),
                        value: 2.1 + ((hash % 4) as f64) / 10.0,
                        unit: "x".to_string(),
                    },
                ];
                (indicators, "金融".to_string(), "金融服务".to_string())
            }
            Market::US => {
                let indicators = vec![
                    FinancialIndicator {
                        name: "Profit Margin".to_string(),
                        value: 22.3 + (hash % 18) as f64,
                        unit: "%".to_string(),
                    },
                    FinancialIndicator {
                        name: "Return on Equity".to_string(),
                        value: 16.8 + (hash % 14) as f64,
                        unit: "%".to_string(),
                    },
                    FinancialIndicator {
                        name: "PE Ratio".to_string(),
                        value: 28.4 + (hash % 20) as f64,
                        unit: "x".to_string(),
                    },
                    FinancialIndicator {
                        name: "PB Ratio".to_string(),
                        value: 4.2 + ((hash % 6) as f64) / 10.0,
                        unit: "x".to_string(),
                    },
                ];
                (
                    indicators,
                    "Technology".to_string(),
                    "Information Technology".to_string(),
                )
            }
            Market::UNKNOWN => {
                let indicators = vec![
                    FinancialIndicator {
                        name: "Net Profit Margin".to_string(),
                        value: 15.0 + (hash % 20) as f64,
                        unit: "%".to_string(),
                    },
                    FinancialIndicator {
                        name: "ROE".to_string(),
                        value: 12.0 + (hash % 15) as f64,
                        unit: "%".to_string(),
                    },
                    FinancialIndicator {
                        name: "P/E Ratio".to_string(),
                        value: 20.0 + (hash % 15) as f64,
                        unit: "x".to_string(),
                    },
                    FinancialIndicator {
                        name: "P/B Ratio".to_string(),
                        value: 3.0 + ((hash % 5) as f64) / 10.0,
                        unit: "x".to_string(),
                    },
                ];
                (indicators, "Unknown".to_string(), "General".to_string())
            }
        };

        let mut valuation = std::collections::HashMap::new();
        for indicator in &indicators {
            if indicator.name.contains("P/E") || indicator.name.contains("PE") {
                valuation.insert("pe_ratio".to_string(), indicator.value);
            } else if indicator.name.contains("P/B") || indicator.name.contains("PB") {
                valuation.insert("pb_ratio".to_string(), indicator.value);
            }
        }

        // Enhanced mock data
        let performance_forecasts = match market {
            Market::ASHARES => PerformanceForecasts {
                revenue_growth_forecast: Some(15.0 + (hash % 20) as f64),
                earnings_growth_forecast: Some(12.0 + (hash % 15) as f64),
                target_price: Some(50.0 + (hash % 100) as f64),
                analyst_rating: "买入".to_string(),
                forecast_period: "12个月".to_string(),
            },
            Market::HONGKONG => PerformanceForecasts {
                revenue_growth_forecast: Some(18.0 + (hash % 25) as f64),
                earnings_growth_forecast: Some(14.0 + (hash % 18) as f64),
                target_price: Some(200.0 + (hash % 200) as f64),
                analyst_rating: "买入".to_string(),
                forecast_period: "12个月".to_string(),
            },
            Market::US => PerformanceForecasts {
                revenue_growth_forecast: Some(20.0 + (hash % 30) as f64),
                earnings_growth_forecast: Some(16.0 + (hash % 20) as f64),
                target_price: Some(300.0 + (hash % 400) as f64),
                analyst_rating: "Buy".to_string(),
                forecast_period: "12 months".to_string(),
            },
            Market::UNKNOWN => PerformanceForecasts {
                revenue_growth_forecast: Some(15.0 + (hash % 20) as f64),
                earnings_growth_forecast: Some(12.0 + (hash % 15) as f64),
                target_price: Some(100.0 + (hash % 100) as f64),
                analyst_rating: "Hold".to_string(),
                forecast_period: "12 months".to_string(),
            },
        };

        let risk_assessment = RiskAssessment {
            beta: Some(1.0 + (hash % 50) as f64 / 100.0),
            debt_to_equity: Some(0.5 + (hash % 20) as f64 / 10.0),
            current_ratio: Some(1.5 + (hash % 10) as f64 / 10.0),
            quick_ratio: Some(1.2 + (hash % 8) as f64 / 10.0),
            interest_coverage: Some(3.0 + (hash % 15) as f64),
            risk_level: if hash % 100 < 30 {
                "低风险".to_string()
            } else if hash % 100 < 70 {
                "中等风险".to_string()
            } else {
                "高风险".to_string()
            },
        };

        let financial_health = FinancialHealth {
            profitability_score: 50.0 + (hash % 40) as f64,
            liquidity_score: 50.0 + (hash % 35) as f64,
            solvency_score: 50.0 + (hash % 30) as f64,
            efficiency_score: 50.0 + (hash % 25) as f64,
            overall_health_score: 50.0 + (hash % 30) as f64,
        };

        Ok(FundamentalData {
            financial_indicators: indicators,
            valuation,
            industry,
            sector,
            performance_forecasts,
            risk_assessment,
            financial_health,
        })
    }

    fn get_mock_news_data(
        &self,
        stock_code: &str,
        days: i32,
        market: &Market,
    ) -> Result<(Vec<News>, SentimentAnalysis), String> {
        let _rng = rand::thread_rng();
        let hash = stock_code.chars().map(|c| c as u32).sum::<u32>();

        // Market-specific news sources
        let (news_sources, market_prefix) = match market {
            Market::ASHARES => (vec!["新浪财经", "东方财富", "证券时报"], "A股"),
            Market::HONGKONG => (vec!["香港经济日报", "信报", "南华早报"], "港股"),
            Market::US => (vec!["Bloomberg", "Reuters", "Wall Street Journal"], "美股"),
            Market::UNKNOWN => (vec!["Financial Times", "MarketWatch"], "股市"),
        };

        let mut news = Vec::new();
        for i in 0..20 {
            let date = Utc::now() - Duration::days((i % days.max(1) as u32) as i64);
            let sentiment = ((hash + i as u32) % 200) as f64 / 100.0 - 1.0;
            let source = news_sources[i as usize % news_sources.len()];

            news.push(News {
                title: format!("{}{}相关新闻{}", market_prefix, stock_code, i + 1),
                content: format!(
                    "这是{}{}的第{}条新闻内容，来自{}",
                    market_prefix,
                    stock_code,
                    i + 1,
                    source
                ),
                date,
                source: source.to_string(),
                news_type: "company_news".to_string(),
                relevance: 0.8,
                sentiment,
            });
        }

        let overall_sentiment = news.iter().map(|n| n.sentiment).sum::<f64>() / news.len() as f64;

        let mut sentiment_by_type = std::collections::HashMap::new();
        sentiment_by_type.insert("company_news".to_string(), overall_sentiment);

        let mut news_distribution = std::collections::HashMap::new();
        news_distribution.insert("company_news".to_string(), news.len() as i32);

        let sentiment_analysis = SentimentAnalysis {
            overall_sentiment,
            sentiment_trend: if overall_sentiment > 0.3 {
                "非常积极".to_string()
            } else if overall_sentiment > 0.1 {
                "偏向积极".to_string()
            } else if overall_sentiment > -0.1 {
                "相对中性".to_string()
            } else if overall_sentiment > -0.3 {
                "偏向消极".to_string()
            } else {
                "非常消极".to_string()
            },
            confidence_score: 0.75,
            total_analyzed: news.len() as i32,
            sentiment_by_type,
            news_distribution,
        };

        Ok((news, sentiment_analysis))
    }
}

// Mock data fetcher for development
pub struct MockDataFetcher;

#[async_trait::async_trait]
impl CachedDataFetcher for AkshareProxy {}

#[async_trait::async_trait]
impl CachedDataFetcher for MockDataFetcher {}

#[async_trait::async_trait]
impl DataFetcher for MockDataFetcher {
    async fn get_stock_data(&self, stock_code: &str, days: i32) -> Result<Vec<PriceData>, String> {
        let market = Market::from_stock_code(stock_code);
        AkshareProxy::new("http://localhost:5000".to_string(), 30)
            .get_mock_stock_data(stock_code, days, &market)
    }

    async fn get_fundamental_data(&self, stock_code: &str) -> Result<FundamentalData, String> {
        let market = Market::from_stock_code(stock_code);
        AkshareProxy::new("http://localhost:5000".to_string(), 30)
            .get_mock_fundamental_data(stock_code, &market)
    }

    async fn get_news_data(
        &self,
        stock_code: &str,
        days: i32,
    ) -> Result<(Vec<News>, SentimentAnalysis), String> {
        let market = Market::from_stock_code(stock_code);
        AkshareProxy::new("http://localhost:5000".to_string(), 30)
            .get_mock_news_data(stock_code, days, &market)
    }

    async fn get_stock_name(&self, stock_code: &str) -> String {
        let market = Market::from_stock_code(stock_code);
        let stock_names = [
            // A-shares
            ("000001", "平安银行"),
            ("600036", "招商银行"),
            ("300019", "硅宝科技"),
            ("000525", "红太阳"),
            ("000858", "五粮液"),
            ("600519", "贵州茅台"),
            ("000002", "万科A"),
            ("600000", "浦发银行"),
            ("601398", "工商银行"),
            ("000651", "格力电器"),
            // Hong Kong stocks
            ("00700", "腾讯控股"),
            ("00941", "中国移动"),
            ("03690", "美团"),
            ("00388", "香港交易所"),
            ("00939", "建设银行"),
            ("00005", "汇丰控股"),
            ("01299", "友邦保险"),
            ("00883", "中国海洋石油"),
            ("00960", "龙湖集团"),
            ("00772", "中国联通"),
            // US stocks
            ("AAPL", "Apple Inc."),
            ("MSFT", "Microsoft Corporation"),
            ("GOOGL", "Alphabet Inc."),
            ("AMZN", "Amazon.com Inc."),
            ("TSLA", "Tesla Inc."),
            ("META", "Meta Platforms Inc."),
            ("NVDA", "NVIDIA Corporation"),
            ("JPM", "JPMorgan Chase & Co."),
            ("JNJ", "Johnson & Johnson"),
            ("V", "Visa Inc."),
        ];

        for (code, name) in stock_names {
            if code == stock_code {
                return name.to_string();
            }
        }

        // Market-specific fallback names
        match market {
            Market::ASHARES => format!("{}股票", stock_code),
            Market::HONGKONG => format!("{}控股", stock_code),
            Market::US => format!("{} Corp.", stock_code),
            Market::UNKNOWN => format!("{}股票", stock_code),
        }
    }

    fn clone(&self) -> Box<dyn DataFetcher> {
        Box::new(MockDataFetcher)
    }
}
