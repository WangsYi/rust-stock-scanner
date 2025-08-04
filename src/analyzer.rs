use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ai_service::AIService;
use crate::data_fetcher::DataFetcher;
use crate::database::Database;
use crate::models::Market;
use crate::models::*;

pub struct StockAnalyzer {
    data_fetcher: Box<dyn DataFetcher>,
    config: AnalysisConfig,
    ai_service: Arc<RwLock<AIService>>,
    database: Option<Arc<Database>>,
}

impl StockAnalyzer {
    pub fn new(
        data_fetcher: Box<dyn DataFetcher>,
        config: AnalysisConfig,
        ai_service: Arc<RwLock<AIService>>,
    ) -> Self {
        Self {
            data_fetcher,
            config,
            ai_service,
            database: None,
        }
    }

    pub fn with_database(
        data_fetcher: Box<dyn DataFetcher>,
        config: AnalysisConfig,
        ai_service: Arc<RwLock<AIService>>,
        database: Arc<Database>,
    ) -> Self {
        Self {
            data_fetcher,
            config,
            ai_service,
            database: Some(database),
        }
    }

    pub fn data_fetcher(&self) -> &dyn DataFetcher {
        self.data_fetcher.as_ref()
    }

    pub async fn analyze_single_stock(
        &self,
        stock_code: &str,
        enable_ai: bool,
    ) -> Result<AnalysisReport, String> {
        let market = Market::from_stock_code(stock_code);

        // Use concurrent data fetching for better performance
        let (price_data, fundamental_data, (news_data, sentiment_data), stock_name) = self
            .data_fetcher
            .get_all_data_concurrent(stock_code, self.config.parameters.technical_period_days)
            .await?;

        let technical = self.calculate_technical_analysis(&price_data);
        let price_info = self.calculate_price_info(&price_data);

        let technical_score = self.calculate_technical_score(&technical, &price_data);
        let fundamental_score = self.calculate_fundamental_score(&fundamental_data, &market);
        let sentiment_score = self.calculate_sentiment_score(&sentiment_data);

        let comprehensive_score = technical_score * self.config.weights.technical
            + fundamental_score * self.config.weights.fundamental
            + sentiment_score * self.config.weights.sentiment;

        let scores = AnalysisScores {
            technical: technical_score,
            fundamental: fundamental_score,
            sentiment: sentiment_score,
            comprehensive: comprehensive_score,
        };

        let recommendation = self.generate_recommendation(&scores, &technical);

        let (ai_analysis, fallback_used, fallback_reason) = if enable_ai {
            let ai_service = self.ai_service.read().await;
            let report_for_ai = AnalysisReport {
                stock_code: stock_code.to_string(),
                stock_name: stock_name.clone(),
                market: market.clone(),
                analysis_date: Utc::now(),
                price_info: price_info.clone(),
                technical: technical.clone(),
                fundamental: fundamental_data.clone(),
                sentiment: sentiment_data.clone(),
                scores: scores.clone(),
                recommendation: recommendation.clone(),
                ai_analysis: String::new(),
                data_quality: DataQuality {
                    financial_indicators_count: fundamental_data.financial_indicators.len() as i32,
                    total_news_count: news_data.len() as i32,
                    analysis_completeness: "完整".to_string(),
                },
                fallback_used: false,
                fallback_reason: None,
            };

            match ai_service.generate_analysis(&report_for_ai).await {
                Ok(analysis) => (analysis, false, None),
                Err(err) => {
                    log::error!("Failed to generate AI analysis: {}", err);
                    let reason = format!("AI分析失败: {}", err);
                    let mut fallback_report = report_for_ai.clone();
                    fallback_report.fallback_used = true;
                    fallback_report.fallback_reason = Some(reason.clone());
                    let fallback_analysis = ai_service.generate_fallback_analysis(&fallback_report);
                    (fallback_analysis, true, Some(reason))
                }
            }
        } else {
            // Even when AI is disabled, use the detailed fallback analysis from AI service
            let ai_service = self.ai_service.read().await;
            let reason = "AI分析已禁用，使用备用分析".to_string();
            let report_for_ai = AnalysisReport {
                stock_code: stock_code.to_string(),
                stock_name: stock_name.clone(),
                market: market.clone(),
                analysis_date: Utc::now(),
                price_info: price_info.clone(),
                technical: technical.clone(),
                fundamental: fundamental_data.clone(),
                sentiment: sentiment_data.clone(),
                scores: scores.clone(),
                recommendation: recommendation.clone(),
                ai_analysis: String::new(),
                data_quality: DataQuality {
                    financial_indicators_count: fundamental_data.financial_indicators.len() as i32,
                    total_news_count: news_data.len() as i32,
                    analysis_completeness: "完整".to_string(),
                },
                fallback_used: true,
                fallback_reason: Some(reason.clone()),
            };
            let fallback_analysis = ai_service.generate_fallback_analysis(&report_for_ai);
            (fallback_analysis, true, Some(reason))
        };

        let report = AnalysisReport {
            stock_code: stock_code.to_string(),
            stock_name,
            market,
            analysis_date: Utc::now(),
            price_info,
            technical,
            fundamental: fundamental_data.clone(),
            sentiment: sentiment_data,
            scores,
            recommendation,
            ai_analysis,
            data_quality: DataQuality {
                financial_indicators_count: fundamental_data.financial_indicators.len() as i32,
                total_news_count: news_data.len() as i32,
                analysis_completeness: "完整".to_string(),
            },
            fallback_used,
            fallback_reason,
        };

        // Save analysis to database if available
        if let Some(database) = &self.database {
            let ai_service_guard = self.ai_service.read().await;
            let ai_provider = Some(ai_service_guard.get_provider().to_string());
            let ai_model = Some(ai_service_guard.get_model().to_string());
            drop(ai_service_guard);

            if let Err(e) = database.save_analysis(&report, ai_provider, ai_model).await {
                log::warn!("Failed to save analysis to database: {}", e);
            }
        }

        Ok(report)
    }

    fn calculate_technical_analysis(&self, price_data: &[PriceData]) -> TechnicalAnalysis {
        if price_data.is_empty() {
            return TechnicalAnalysis::default();
        }

        let prices: Vec<f64> = price_data.iter().map(|p| p.close).collect();
        let volumes: Vec<f64> = price_data.iter().map(|p| p.volume as f64).collect();
        let highs: Vec<f64> = price_data.iter().map(|p| p.high).collect();
        let lows: Vec<f64> = price_data.iter().map(|p| p.low).collect();

        // Moving Averages
        let ma5 = self.calculate_ma(&prices, 5);
        let ma10 = self.calculate_ma(&prices, 10);
        let ma20 = self.calculate_ma(&prices, 20);
        let ma60 = self.calculate_ma(&prices, 60);
        let ma120 = self.calculate_ma(&prices, 120);

        // Momentum Indicators
        let rsi = self.calculate_rsi(&prices, 14);
        let (macd_signal, macd_line, macd_histogram) = self.calculate_macd(&prices);

        // Volatility Indicators
        let (bb_position, bb_upper, bb_middle, bb_lower) = self.calculate_bollinger_bands(&prices);
        let atr = self.calculate_atr(&highs, &lows, &prices, 14);

        // Additional Indicators
        let williams_r = self.calculate_williams_r(&highs, &lows, &prices, 14);
        let cci = self.calculate_cci(&highs, &lows, &prices, 20);
        let (stochastic_k, stochastic_d) = self.calculate_stochastic(&highs, &lows, &prices, 14, 3);

        // Volume and Trend
        let current_volume = *volumes.last().unwrap_or(&0.0);
        let avg_volume = volumes.iter().sum::<f64>() / volumes.len() as f64;
        let volume_status = if current_volume > avg_volume * 1.5 {
            "放量"
        } else if current_volume < avg_volume * 0.5 {
            "缩量"
        } else {
            "正常"
        };

        let ma_trend = if *prices.last().unwrap_or(&0.0) > ma20 {
            "相对强势".to_string()
        } else {
            "相对弱势".to_string()
        };

        let adx = self.calculate_adx(&highs, &lows, &prices, 14);
        let trend_strength = if adx > 25.0 {
            if adx > 50.0 {
                "强趋势".to_string()
            } else {
                "中等趋势".to_string()
            }
        } else {
            "弱趋势".to_string()
        };

        TechnicalAnalysis {
            // Moving Averages
            ma5,
            ma10,
            ma20,
            ma60,
            ma120,

            // Momentum Indicators
            rsi,
            macd_signal,
            macd_line,
            macd_histogram,

            // Volatility Indicators
            bb_position,
            bb_upper,
            bb_middle,
            bb_lower,
            atr,

            // Additional Indicators
            williams_r,
            cci,
            stochastic_k,
            stochastic_d,

            // Volume and Trend
            volume_status: volume_status.to_string(),
            ma_trend,
            adx,
            trend_strength,
        }
    }

    fn calculate_ma(&self, data: &[f64], period: usize) -> f64 {
        if data.len() < period {
            return data.iter().sum::<f64>() / data.len() as f64;
        }
        let slice = &data[data.len() - period..];
        slice.iter().sum::<f64>() / period as f64
    }

    fn calculate_rsi(&self, data: &[f64], period: usize) -> f64 {
        if data.len() < period + 1 {
            return 50.0;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in data.len() - period..data.len() {
            let change = data[i] - data[i - 1];
            if change > 0.0 {
                gains += change;
            } else {
                losses -= change;
            }
        }

        let rs = if losses == 0.0 { 100.0 } else { gains / losses };
        100.0 - (100.0 / (1.0 + rs))
    }

    fn calculate_std_dev(&self, data: &[f64], period: usize) -> f64 {
        if data.len() < period {
            return 1.0;
        }

        let slice = &data[data.len() - period..];
        let mean = slice.iter().sum::<f64>() / period as f64;
        let variance = slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / period as f64;
        variance.sqrt()
    }

    // Enhanced MACD calculation with histogram
    fn calculate_macd(&self, data: &[f64]) -> (String, f64, f64) {
        let short_ma = self.calculate_ma(data, 12.min(data.len()));
        let long_ma = self.calculate_ma(data, 26.min(data.len()));
        let macd_line = short_ma - long_ma;

        // Calculate signal line (9-period EMA of MACD line)
        let signal_line = self.calculate_ema(&[macd_line], 9);
        let macd_histogram = macd_line - signal_line;

        let macd_signal = if macd_line > signal_line {
            "看涨"
        } else {
            "看跌"
        };

        (macd_signal.to_string(), macd_line, macd_histogram)
    }

    // Bollinger Bands calculation with full band information
    fn calculate_bollinger_bands(&self, data: &[f64]) -> (f64, f64, f64, f64) {
        let period = 20.min(data.len());
        let middle = self.calculate_ma(data, period);
        let std_dev = self.calculate_std_dev(data, period);

        let upper = middle + 2.0 * std_dev;
        let lower = middle - 2.0 * std_dev;
        let current = *data.last().unwrap_or(&0.0);

        let position = if upper - lower > 0.0 {
            (current - lower) / (upper - lower)
        } else {
            0.5
        };

        (position, upper, middle, lower)
    }

    // Average True Range (ATR) calculation
    fn calculate_atr(&self, highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> f64 {
        if highs.len() < period || lows.len() < period || closes.len() < period {
            return 0.0;
        }

        let mut true_ranges = Vec::new();
        for i in 1..closes.len() {
            let high = highs[i];
            let low = lows[i];
            let prev_close = closes[i - 1];

            let tr = high - low;
            let tr1 = (high - prev_close).abs();
            let tr2 = (low - prev_close).abs();

            true_ranges.push(tr.max(tr1).max(tr2));
        }

        let slice = &true_ranges[true_ranges.len() - period.min(true_ranges.len())..];
        slice.iter().sum::<f64>() / slice.len() as f64
    }

    // Williams %R calculation
    fn calculate_williams_r(
        &self,
        highs: &[f64],
        lows: &[f64],
        closes: &[f64],
        period: usize,
    ) -> f64 {
        if highs.len() < period || lows.len() < period || closes.len() < period {
            return -50.0;
        }

        let highest = highs.iter().take(period).fold(0.0_f64, |a, &b| a.max(b));
        let lowest = lows
            .iter()
            .take(period)
            .fold(f64::INFINITY, |a, &b| a.min(b));
        let current = *closes.last().unwrap_or(&0.0);

        if highest - lowest > 0.0 {
            -100.0 * (highest - current) / (highest - lowest)
        } else {
            -50.0
        }
    }

    // Commodity Channel Index (CCI) calculation
    fn calculate_cci(&self, highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> f64 {
        if highs.len() < period || lows.len() < period || closes.len() < period {
            return 0.0;
        }

        let mut typical_prices = Vec::new();
        for i in 0..closes.len() {
            let tp = (highs[i] + lows[i] + closes[i]) / 3.0;
            typical_prices.push(tp);
        }

        let sma = self.calculate_ma(&typical_prices, period);
        let mean_deviation = {
            let slice = &typical_prices[typical_prices.len() - period..];
            let mean = slice.iter().sum::<f64>() / period as f64;
            slice.iter().map(|x| (x - mean).abs()).sum::<f64>() / period as f64
        };

        if mean_deviation > 0.0 {
            (sma - mean_deviation) / (0.015 * mean_deviation)
        } else {
            0.0
        }
    }

    // Stochastic Oscillator calculation
    fn calculate_stochastic(
        &self,
        highs: &[f64],
        lows: &[f64],
        closes: &[f64],
        k_period: usize,
        d_period: usize,
    ) -> (f64, f64) {
        if highs.len() < k_period || lows.len() < k_period || closes.len() < k_period {
            return (50.0, 50.0);
        }

        let mut k_values = Vec::new();
        for i in k_period - 1..closes.len() {
            let start_idx = i.saturating_sub(k_period - 1);
            let highest = highs[start_idx..=i].iter().fold(0.0_f64, |a, &b| a.max(b));
            let lowest = lows[start_idx..=i]
                .iter()
                .fold(f64::INFINITY, |a, &b| a.min(b));
            let current = closes[i];

            if highest - lowest > 0.0 {
                let k = 100.0 * (current - lowest) / (highest - lowest);
                k_values.push(k);
            } else {
                k_values.push(50.0);
            }
        }

        let k = *k_values.last().unwrap_or(&50.0);
        let d = self.calculate_ma(&k_values, d_period.min(k_values.len()));

        (k, d)
    }

    // Average Directional Index (ADX) calculation
    fn calculate_adx(&self, highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> f64 {
        if highs.len() < period + 1 || lows.len() < period + 1 || closes.len() < period + 1 {
            return 25.0;
        }

        let mut plus_dms = Vec::new();
        let mut minus_dms = Vec::new();
        let mut trs = Vec::new();

        for i in 1..closes.len() {
            let up_move = highs[i] - highs[i - 1];
            let down_move = lows[i - 1] - lows[i];

            let plus_dm = if up_move > down_move && up_move > 0.0 {
                up_move
            } else {
                0.0
            };
            let minus_dm = if down_move > up_move && down_move > 0.0 {
                down_move
            } else {
                0.0
            };

            plus_dms.push(plus_dm);
            minus_dms.push(minus_dm);

            let tr = (highs[i] - lows[i])
                .max((highs[i] - closes[i - 1]).abs())
                .max((lows[i] - closes[i - 1]).abs());
            trs.push(tr);
        }

        let smooth_plus_dm = self.calculate_wilder_smoothing(&plus_dms, period);
        let smooth_minus_dm = self.calculate_wilder_smoothing(&minus_dms, period);
        let smooth_tr = self.calculate_wilder_smoothing(&trs, period);

        if smooth_tr > 0.0 {
            let plus_di = 100.0 * smooth_plus_dm / smooth_tr;
            let minus_di = 100.0 * smooth_minus_dm / smooth_tr;
            let dx = (plus_di - minus_di).abs() / (plus_di + minus_di) * 100.0;
            dx
        } else {
            25.0
        }
    }

    // Wilder's smoothing method for ADX calculation
    fn calculate_wilder_smoothing(&self, data: &[f64], period: usize) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let sum = data.iter().take(period.min(data.len())).sum::<f64>();
        let mut smoothed = sum / period as f64;

        for i in period..data.len() {
            smoothed = (smoothed * (period - 1) as f64 + data[i]) / period as f64;
        }

        smoothed
    }

    // Exponential Moving Average (EMA) calculation
    fn calculate_ema(&self, data: &[f64], period: usize) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let multiplier = 2.0 / (period as f64 + 1.0);
        let mut ema = data[0];

        for i in 1..data.len() {
            ema = data[i] * multiplier + ema * (1.0 - multiplier);
        }

        ema
    }

    fn calculate_price_info(&self, price_data: &[PriceData]) -> PriceInfo {
        if price_data.is_empty() {
            return PriceInfo::default();
        }

        let current_price = price_data.last().unwrap().close;
        let price_change = if price_data.len() > 1 {
            current_price - price_data[price_data.len() - 2].close
        } else {
            0.0
        };

        let recent_volumes: Vec<f64> = price_data
            .iter()
            .rev()
            .take(5)
            .map(|p| p.volume as f64)
            .collect();

        let avg_volume = recent_volumes.iter().sum::<f64>() / recent_volumes.len() as f64;
        let volume_ratio = if avg_volume > 0.0 {
            recent_volumes.first().unwrap_or(&0.0) / avg_volume
        } else {
            1.0
        };

        let prices: Vec<f64> = price_data.iter().map(|p| p.close).collect();
        let volatility = self.calculate_std_dev(&prices, 20.min(prices.len()));

        PriceInfo {
            current_price,
            price_change,
            volume_ratio,
            volatility,
        }
    }

    fn calculate_technical_score(
        &self,
        technical: &TechnicalAnalysis,
        _price_data: &[PriceData],
    ) -> f64 {
        let mut score: f64 = 50.0;

        // RSI impact
        if technical.rsi > 70.0 {
            score -= 8.0;
        } else if technical.rsi < 30.0 {
            score += 8.0;
        } else if technical.rsi > 50.0 {
            score += 4.0;
        } else {
            score -= 4.0;
        }

        // MACD signal impact
        match technical.macd_signal.as_str() {
            "看涨" => score += 8.0,
            "看跌" => score -= 8.0,
            _ => {}
        }

        // MA trend impact
        match technical.ma_trend.as_str() {
            "相对强势" => score += 6.0,
            "相对弱势" => score -= 6.0,
            _ => {}
        }

        // Bollinger Bands position
        if technical.bb_position > 0.8 {
            score -= 5.0; // Overbought
        } else if technical.bb_position < 0.2 {
            score += 5.0; // Oversold
        } else if technical.bb_position > 0.6 {
            score -= 2.0;
        } else if technical.bb_position < 0.4 {
            score += 2.0;
        }

        // Williams %R impact
        if technical.williams_r < -80.0 {
            score += 6.0; // Oversold
        } else if technical.williams_r > -20.0 {
            score -= 6.0; // Overbought
        }

        // CCI impact
        if technical.cci > 100.0 {
            score -= 5.0; // Overbought
        } else if technical.cci < -100.0 {
            score += 5.0; // Oversold
        }

        // Stochastic Oscillator impact
        if technical.stochastic_k > 80.0 {
            score -= 4.0; // Overbought
        } else if technical.stochastic_k < 20.0 {
            score += 4.0; // Oversold
        }

        // Trend strength impact
        match technical.trend_strength.as_str() {
            "强趋势" => score += 8.0,
            "中等趋势" => score += 4.0,
            "弱趋势" => score -= 2.0,
            _ => {}
        }

        // Volume status impact
        match technical.volume_status.as_str() {
            "放量" => score += 3.0,
            "缩量" => score -= 3.0,
            _ => {}
        }

        score.min(100.0).max(0.0)
    }

    fn calculate_fundamental_score(&self, fundamental: &FundamentalData, market: &Market) -> f64 {
        let mut score: f64 = 50.0;

        // Market-specific fundamental analysis
        for indicator in &fundamental.financial_indicators {
            match indicator.name.as_str() {
                // Profit indicators
                "净利润率" | "Net Profit Margin" | "Profit Margin" => {
                    if indicator.value > 20.0 {
                        score += 10.0;
                    } else if indicator.value > 10.0 {
                        score += 6.0;
                    } else if indicator.value < 5.0 {
                        score -= 8.0;
                    }
                }
                // Return indicators
                "净资产收益率" | "ROE" | "Return on Equity" => {
                    if indicator.value > 15.0 {
                        score += 10.0;
                    } else if indicator.value > 10.0 {
                        score += 6.0;
                    } else if indicator.value < 8.0 {
                        score -= 8.0;
                    }
                }
                // Valuation ratios - market specific
                "市盈率" | "P/E Ratio" | "PE Ratio" => match market {
                    Market::ASHARES => {
                        if indicator.value > 0.0 && indicator.value < 20.0 {
                            score += 8.0;
                        } else if indicator.value > 50.0 {
                            score -= 10.0;
                        }
                    }
                    Market::US => {
                        if indicator.value > 0.0 && indicator.value < 25.0 {
                            score += 8.0;
                        } else if indicator.value > 40.0 {
                            score -= 8.0;
                        }
                    }
                    Market::HONGKONG => {
                        if indicator.value > 0.0 && indicator.value < 15.0 {
                            score += 10.0;
                        } else if indicator.value > 30.0 {
                            score -= 10.0;
                        }
                    }
                    Market::UNKNOWN => {
                        if indicator.value > 0.0 && indicator.value < 20.0 {
                            score += 6.0;
                        } else if indicator.value > 50.0 {
                            score -= 8.0;
                        }
                    }
                },
                "市净率" | "P/B Ratio" | "PB Ratio" => match market {
                    Market::ASHARES => {
                        if indicator.value > 0.0 && indicator.value < 3.0 {
                            score += 8.0;
                        } else if indicator.value > 5.0 {
                            score -= 8.0;
                        }
                    }
                    Market::US => {
                        if indicator.value > 0.0 && indicator.value < 4.0 {
                            score += 6.0;
                        } else if indicator.value > 8.0 {
                            score -= 6.0;
                        }
                    }
                    Market::HONGKONG => {
                        if indicator.value > 0.0 && indicator.value < 2.5 {
                            score += 10.0;
                        } else if indicator.value > 4.0 {
                            score -= 10.0;
                        }
                    }
                    Market::UNKNOWN => {
                        if indicator.value > 0.0 && indicator.value < 3.0 {
                            score += 6.0;
                        } else if indicator.value > 6.0 {
                            score -= 6.0;
                        }
                    }
                },
                // Additional indicators
                "股息率" | "Dividend Yield" => {
                    if indicator.value > 3.0 {
                        score += 6.0;
                    } else if indicator.value > 1.5 {
                        score += 3.0;
                    }
                }
                "营收增长率" | "Revenue Growth" => {
                    if indicator.value > 20.0 {
                        score += 8.0;
                    } else if indicator.value > 10.0 {
                        score += 5.0;
                    } else if indicator.value < 0.0 {
                        score -= 8.0;
                    }
                }
                _ => {}
            }
        }

        // Performance forecasts impact
        if let Some(revenue_growth) = fundamental.performance_forecasts.revenue_growth_forecast {
            if revenue_growth > 15.0 {
                score += 8.0;
            } else if revenue_growth > 10.0 {
                score += 5.0;
            } else if revenue_growth < 0.0 {
                score -= 6.0;
            }
        }

        if let Some(earnings_growth) = fundamental.performance_forecasts.earnings_growth_forecast {
            if earnings_growth > 12.0 {
                score += 8.0;
            } else if earnings_growth > 8.0 {
                score += 5.0;
            } else if earnings_growth < 0.0 {
                score -= 6.0;
            }
        }

        // Analyst rating impact
        match fundamental.performance_forecasts.analyst_rating.as_str() {
            "买入" | "Buy" | "Strong Buy" => score += 10.0,
            "增持" | "Overweight" => score += 6.0,
            "中性" | "Hold" => score += 0.0,
            "减持" | "Underweight" => score -= 6.0,
            "卖出" | "Sell" => score -= 10.0,
            _ => {}
        }

        // Risk assessment impact
        match fundamental.risk_assessment.risk_level.as_str() {
            "低风险" | "Low Risk" => score += 8.0,
            "中等风险" | "Medium Risk" => score += 0.0,
            "高风险" | "High Risk" => score -= 8.0,
            _ => {}
        }

        // Beta impact
        if let Some(beta) = fundamental.risk_assessment.beta {
            if beta < 0.8 {
                score += 4.0; // Low volatility
            } else if beta > 1.2 {
                score -= 4.0; // High volatility
            }
        }

        // Financial health impact
        score += (fundamental.financial_health.overall_health_score - 50.0) * 0.3;

        // Liquidity ratios impact
        if let Some(current_ratio) = fundamental.risk_assessment.current_ratio {
            if current_ratio > 2.0 {
                score += 4.0;
            } else if current_ratio < 1.0 {
                score -= 6.0;
            }
        }

        if let Some(debt_to_equity) = fundamental.risk_assessment.debt_to_equity {
            if debt_to_equity < 0.5 {
                score += 4.0;
            } else if debt_to_equity > 2.0 {
                score -= 6.0;
            }
        }

        score.min(100.0).max(0.0)
    }

    fn calculate_sentiment_score(&self, sentiment: &SentimentAnalysis) -> f64 {
        let mut score = 50.0;

        // Overall sentiment impact
        if sentiment.overall_sentiment > 0.3 {
            score += 20.0;
        } else if sentiment.overall_sentiment > 0.1 {
            score += 10.0;
        } else if sentiment.overall_sentiment < -0.3 {
            score -= 20.0;
        } else if sentiment.overall_sentiment < -0.1 {
            score -= 10.0;
        }

        // Confidence score impact
        score += (sentiment.confidence_score - 0.5) * 20.0;

        score.min(100.0).max(0.0)
    }

    fn generate_recommendation(
        &self,
        scores: &AnalysisScores,
        _technical: &TechnicalAnalysis,
    ) -> String {
        match scores.comprehensive {
            score if score >= 80.0 => "强烈推荐买入",
            score if score >= 70.0 => "建议买入",
            score if score >= 60.0 => "可以考虑买入",
            score if score >= 40.0 => "观望",
            score if score >= 30.0 => "建议卖出",
            _ => "强烈建议卖出",
        }
        .to_string()
    }

    fn generate_fallback_analysis(
        &self,
        stock_code: &str,
        price_info: &PriceInfo,
        fundamental: &FundamentalData,
        sentiment: &SentimentAnalysis,
        market: &Market,
    ) -> String {
        let currency = market.get_currency();
        let market_name = market.get_market_name();

        let mut analysis = format!(
            "基于对{}（{}）的综合分析：\n\n交易市场：{}\n计价货币：{}\n当前股价：{:.2} {}，近期涨跌幅：{:.2}%\n\n基本面亮点：\n",
            stock_code,
            market,
            market_name,
            currency,
            price_info.current_price,
            currency,
            price_info.price_change
        );

        // 添加财务指标信息
        if !fundamental.financial_indicators.is_empty() {
            let mut key_indicators = Vec::new();
            for indicator in &fundamental.financial_indicators {
                match indicator.name.as_str() {
                    "流动比率" | "Current Ratio" => {
                        key_indicators.push(format!("流动比率: {:.2}", indicator.value))
                    }
                    "净资产收益率" | "ROE" => {
                        key_indicators.push(format!("ROE: {:.2}%", indicator.value))
                    }
                    "市盈率" | "P/E Ratio" => {
                        key_indicators.push(format!("市盈率: {:.2}", indicator.value))
                    }
                    "净利润率" | "Net Profit Margin" => {
                        key_indicators.push(format!("净利润率: {:.2}%", indicator.value))
                    }
                    _ => {}
                }
            }

            if !key_indicators.is_empty() {
                analysis.push_str(&key_indicators.join(", "));
                analysis.push('\n');
            }
        }

        // 添加市场情绪信息
        if sentiment.overall_sentiment != 0.0 {
            let sentiment_desc = if sentiment.overall_sentiment > 0.1 {
                "偏向积极"
            } else if sentiment.overall_sentiment < -0.1 {
                "偏向消极"
            } else {
                "中性"
            };
            analysis.push_str(&format!(
                "\n市场情绪：{} (得分: {:.3})\n",
                sentiment_desc, sentiment.overall_sentiment
            ));
        }

        // 添加技术面简述
        analysis.push_str(&format!(
            "\n技术面：当前价格波动率 {:.2}%，建议关注技术指标变化\n",
            price_info.volatility
        ));

        analysis
    }
}

impl Default for TechnicalAnalysis {
    fn default() -> Self {
        TechnicalAnalysis {
            // Moving Averages
            ma5: 0.0,
            ma10: 0.0,
            ma20: 0.0,
            ma60: 0.0,
            ma120: 0.0,

            // Momentum Indicators
            rsi: 50.0,
            macd_signal: "观望".to_string(),
            macd_line: 0.0,
            macd_histogram: 0.0,

            // Volatility Indicators
            bb_position: 0.5,
            bb_upper: 0.0,
            bb_middle: 0.0,
            bb_lower: 0.0,
            atr: 0.0,

            // Additional Indicators
            williams_r: -50.0,
            cci: 0.0,
            stochastic_k: 50.0,
            stochastic_d: 50.0,

            // Volume and Trend
            volume_status: "正常".to_string(),
            ma_trend: "中性".to_string(),
            adx: 25.0,
            trend_strength: "弱趋势".to_string(),
        }
    }
}

impl Default for PriceInfo {
    fn default() -> Self {
        PriceInfo {
            current_price: 0.0,
            price_change: 0.0,
            volume_ratio: 1.0,
            volatility: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_service::AIService;
    use crate::data_fetcher::MockDataFetcher;

    #[tokio::test]
    async fn test_analyze_single_stock() {
        let data_fetcher = Box::new(MockDataFetcher);
        let config = AnalysisConfig::default();
        let ai_service = Arc::new(RwLock::new(AIService::new(AIConfig::default())));

        let analyzer = StockAnalyzer::new(data_fetcher, config, ai_service);

        let result = analyzer.analyze_single_stock("000001", false).await;
        assert!(result.is_ok());
    }
}
