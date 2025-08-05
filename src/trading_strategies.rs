use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::*;

/// 交易策略分析器
pub struct TradingStrategiesAnalyzer {
    // 策略配置
    pub rsi_overbought: f64,        // RSI超买线
    pub rsi_oversold: f64,          // RSI超卖线
    pub macd_fast_period: i32,      // MACD快线周期
    pub macd_slow_period: i32,      // MACD慢线周期
    pub ma_short_period: i32,        // 短期均线周期
    pub ma_long_period: i32,         // 长期均线周期
    pub bb_period: i32,             // 布林带周期
    pub bb_std_dev: f64,            // 布林带标准差倍数
}

impl TradingStrategiesAnalyzer {
    /// 创建新的策略分析器
    pub fn new() -> Self {
        Self {
            rsi_overbought: 70.0,
            rsi_oversold: 30.0,
            macd_fast_period: 12,
            macd_slow_period: 26,
            ma_short_period: 5,
            ma_long_period: 20,
            bb_period: 20,
            bb_std_dev: 2.0,
        }
    }

    /// 分析所有交易策略
    pub async fn analyze_all_strategies(
        &self,
        stock_code: &str,
        price_data: &[PriceData],
    ) -> Result<TradingStrategies, Box<dyn std::error::Error>> {
        if price_data.is_empty() {
            return Err("No price data available for strategy analysis".into());
        }

        let macd_strategy = self.analyze_macd_strategy(price_data).await?;
        let rsi_strategy = self.analyze_rsi_strategy(price_data).await?;
        let ma_strategy = self.analyze_moving_average_strategy(price_data).await?;
        let bb_strategy = self.analyze_bollinger_bands_strategy(price_data).await?;
        let kline_strategy = self.analyze_kline_patterns_strategy(price_data).await?;
        let volume_strategy = self.analyze_volume_analysis_strategy(price_data).await?;

        Ok(TradingStrategies {
            macd: macd_strategy,
            rsi: rsi_strategy,
            moving_average: ma_strategy,
            bollinger_bands: bb_strategy,
            kline_patterns: kline_strategy,
            volume_analysis: volume_strategy,
        })
    }

    /// MACD策略分析
    pub async fn analyze_macd_strategy(
        &self,
        price_data: &[PriceData],
    ) -> Result<MACDStrategy, Box<dyn std::error::Error>> {
        if price_data.len() < self.macd_slow_period as usize {
            return Err("Insufficient data for MACD analysis".into());
        }

        let prices: Vec<f64> = price_data.iter().map(|p| p.close).collect();
        let (macd_line, signal_line, histogram) = self.calculate_macd(&prices);

        let current_macd = macd_line.last().unwrap_or(&0.0);
        let current_signal = signal_line.last().unwrap_or(&0.0);
        let current_histogram = histogram.last().unwrap_or(&0.0);

        let signal_type = self.generate_macd_signal(*current_macd, *current_signal, *current_histogram);
        let divergence = self.detect_macd_divergence(&prices, &macd_line);

        Ok(MACDStrategy {
            fast_period: self.macd_fast_period,
            slow_period: self.macd_slow_period,
            signal_period: 9, // 默认信号线周期
            current_macd: *current_macd,
            current_signal: *current_signal,
            histogram: *current_histogram,
            signal_type,
            divergence,
        })
    }

    /// RSI策略分析
    pub async fn analyze_rsi_strategy(
        &self,
        price_data: &[PriceData],
    ) -> Result<RSIStrategy, Box<dyn std::error::Error>> {
        if price_data.len() < 14 {
            return Err("Insufficient data for RSI analysis".into());
        }

        let prices: Vec<f64> = price_data.iter().map(|p| p.close).collect();
        let rsi_values = self.calculate_rsi(&prices, 14);
        let current_rsi = rsi_values.last().unwrap_or(&50.0);

        let signal_type = self.generate_rsi_signal(*current_rsi);
        let divergence = self.detect_rsi_divergence(&prices, &rsi_values);

        Ok(RSIStrategy {
            period: 14,
            current_rsi: *current_rsi,
            overbought: self.rsi_overbought,
            oversold: self.rsi_oversold,
            signal_type,
            divergence,
        })
    }

    /// 移动平均线策略分析
    pub async fn analyze_moving_average_strategy(
        &self,
        price_data: &[PriceData],
    ) -> Result<MovingAverageStrategy, Box<dyn std::error::Error>> {
        if price_data.len() < self.ma_long_period as usize {
            return Err("Insufficient data for moving average analysis".into());
        }

        let prices: Vec<f64> = price_data.iter().map(|p| p.close).collect();
        let short_ma = self.calculate_sma(&prices, self.ma_short_period);
        let long_ma = self.calculate_sma(&prices, self.ma_long_period);

        let current_short_ma = short_ma.last().unwrap_or(&0.0);
        let current_long_ma = long_ma.last().unwrap_or(&0.0);

        let (signal_type, golden_cross, death_cross) = self.generate_ma_signal(&short_ma, &long_ma);

        Ok(MovingAverageStrategy {
            short_period: self.ma_short_period,
            long_period: self.ma_long_period,
            short_ma: *current_short_ma,
            long_ma: *current_long_ma,
            signal_type,
            golden_cross,
            death_cross,
        })
    }

    /// 布林带策略分析
    pub async fn analyze_bollinger_bands_strategy(
        &self,
        price_data: &[PriceData],
    ) -> Result<BollingerBandsStrategy, Box<dyn std::error::Error>> {
        if price_data.len() < self.bb_period as usize {
            return Err("Insufficient data for Bollinger Bands analysis".into());
        }

        let prices: Vec<f64> = price_data.iter().map(|p| p.close).collect();
        let (upper_band, middle_band, lower_band) = self.calculate_bollinger_bands(&prices, self.bb_period, self.bb_std_dev);

        let current_price = prices.last().unwrap_or(&0.0);
        let current_upper = upper_band.last().unwrap_or(&0.0);
        let current_middle = middle_band.last().unwrap_or(&0.0);
        let current_lower = lower_band.last().unwrap_or(&0.0);

        let bandwidth = self.calculate_bandwidth(*current_upper, *current_lower, *current_middle);
        let signal_type = self.generate_bb_signal(*current_price, *current_upper, *current_lower);
        let squeeze = bandwidth < 0.1; // 简化的挤压检测

        Ok(BollingerBandsStrategy {
            period: self.bb_period,
            std_dev: self.bb_std_dev,
            upper_band: *current_upper,
            middle_band: *current_middle,
            lower_band: *current_lower,
            bandwidth,
            signal_type,
            squeeze,
        })
    }

    /// K线形态策略分析
    pub async fn analyze_kline_patterns_strategy(
        &self,
        price_data: &[PriceData],
    ) -> Result<KlinePatternsStrategy, Box<dyn std::error::Error>> {
        if price_data.len() < 5 {
            return Err("Insufficient data for K-line patterns analysis".into());
        }

        let patterns = self.detect_kline_patterns(price_data);
        let reversal_patterns = self.detect_reversal_patterns(price_data);
        let continuation_patterns = self.detect_continuation_patterns(price_data);

        let signal_type = self.generate_kline_signal(&patterns, &reversal_patterns);
        let reliability = self.calculate_pattern_reliability(&patterns);

        Ok(KlinePatternsStrategy {
            patterns,
            reversal_patterns,
            continuation_patterns,
            signal_type,
            reliability,
        })
    }

    /// 成交量分析策略
    pub async fn analyze_volume_analysis_strategy(
        &self,
        price_data: &[PriceData],
    ) -> Result<VolumeAnalysisStrategy, Box<dyn std::error::Error>> {
        if price_data.len() < 10 {
            return Err("Insufficient data for volume analysis".into());
        }

        let volume_ratio = self.calculate_volume_ratio(price_data);
        let volume_trend = self.analyze_volume_trend(price_data);
        let mfi = self.calculate_money_flow_index(price_data);
        let ad_line = self.calculate_accumulation_distribution(price_data);

        let signal_type = self.generate_volume_signal(volume_ratio, &volume_trend, mfi);
        let breakouts = self.detect_volume_breakouts(price_data);

        Ok(VolumeAnalysisStrategy {
            volume_ratio,
            volume_trend,
            money_flow_index: mfi,
            accumulation_distribution: ad_line,
            signal_type,
            breakouts,
        })
    }

    /// 生成交易信号
    pub fn generate_trading_signals(
        &self,
        strategies: &TradingStrategies,
        current_price: f64,
    ) -> Vec<TradingSignal> {
        let mut signals = Vec::new();

        // MACD信号
        if strategies.macd.signal_type != "持有" {
            signals.push(TradingSignal {
                strategy_name: "MACD策略".to_string(),
                signal_type: strategies.macd.signal_type.clone(),
                strength: self.calculate_signal_strength(&strategies.macd.signal_type),
                price: current_price,
                timestamp: Utc::now(),
                reason: format!("MACD信号: {}线与信号线交叉", strategies.macd.signal_type),
                confidence: self.calculate_macd_confidence(&strategies.macd),
                risk_level: self.calculate_risk_level(&strategies.macd.signal_type),
                expected_profit: self.calculate_expected_profit(&strategies.macd.signal_type, current_price),
                stop_loss: self.calculate_stop_loss(&strategies.macd.signal_type, current_price),
                take_profit: self.calculate_take_profit(&strategies.macd.signal_type, current_price),
            });
        }

        // RSI信号
        if strategies.rsi.signal_type != "持有" {
            signals.push(TradingSignal {
                strategy_name: "RSI策略".to_string(),
                signal_type: strategies.rsi.signal_type.clone(),
                strength: self.calculate_signal_strength(&strategies.rsi.signal_type),
                price: current_price,
                timestamp: Utc::now(),
                reason: format!("RSI超买超卖信号: {:.1}", strategies.rsi.current_rsi),
                confidence: self.calculate_rsi_confidence(&strategies.rsi),
                risk_level: self.calculate_risk_level(&strategies.rsi.signal_type),
                expected_profit: self.calculate_expected_profit(&strategies.rsi.signal_type, current_price),
                stop_loss: self.calculate_stop_loss(&strategies.rsi.signal_type, current_price),
                take_profit: self.calculate_take_profit(&strategies.rsi.signal_type, current_price),
            });
        }

        // 移动平均线信号
        if strategies.moving_average.signal_type != "持有" {
            signals.push(TradingSignal {
                strategy_name: "均线策略".to_string(),
                signal_type: strategies.moving_average.signal_type.clone(),
                strength: self.calculate_signal_strength(&strategies.moving_average.signal_type),
                price: current_price,
                timestamp: Utc::now(),
                reason: format!("均线交叉信号: {}日均线与{}日均线", 
                    strategies.moving_average.short_period, strategies.moving_average.long_period),
                confidence: self.calculate_ma_confidence(&strategies.moving_average),
                risk_level: self.calculate_risk_level(&strategies.moving_average.signal_type),
                expected_profit: self.calculate_expected_profit(&strategies.moving_average.signal_type, current_price),
                stop_loss: self.calculate_stop_loss(&strategies.moving_average.signal_type, current_price),
                take_profit: self.calculate_take_profit(&strategies.moving_average.signal_type, current_price),
            });
        }

        // 布林带信号
        if strategies.bollinger_bands.signal_type != "持有" {
            signals.push(TradingSignal {
                strategy_name: "布林带策略".to_string(),
                signal_type: strategies.bollinger_bands.signal_type.clone(),
                strength: self.calculate_signal_strength(&strategies.bollinger_bands.signal_type),
                price: current_price,
                timestamp: Utc::now(),
                reason: "布林带突破信号".to_string(),
                confidence: self.calculate_bb_confidence(&strategies.bollinger_bands),
                risk_level: self.calculate_risk_level(&strategies.bollinger_bands.signal_type),
                expected_profit: self.calculate_expected_profit(&strategies.bollinger_bands.signal_type, current_price),
                stop_loss: self.calculate_stop_loss(&strategies.bollinger_bands.signal_type, current_price),
                take_profit: self.calculate_take_profit(&strategies.bollinger_bands.signal_type, current_price),
            });
        }

        signals
    }

    // MACD计算函数
    fn calculate_macd(&self, prices: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let ema_fast = self.calculate_ema(prices, self.macd_fast_period);
        let ema_slow = self.calculate_ema(prices, self.macd_slow_period);
        
        let macd_line: Vec<f64> = ema_fast.iter().zip(ema_slow.iter())
            .map(|(fast, slow)| fast - slow)
            .collect();
        
        let signal_line = self.calculate_ema(&macd_line, 9);
        let histogram: Vec<f64> = macd_line.iter().zip(signal_line.iter())
            .map(|(macd, signal)| macd - signal)
            .collect();
        
        (macd_line, signal_line, histogram)
    }

    // RSI计算函数
    fn calculate_rsi(&self, prices: &[f64], period: usize) -> Vec<f64> {
        let mut rsi_values = Vec::new();
        
        for i in period..prices.len() {
            let window = &prices[i - period..i];
            let mut gains = 0.0;
            let mut losses = 0.0;
            
            for j in 1..window.len() {
                let change = window[j] - window[j - 1];
                if change > 0.0 {
                    gains += change;
                } else {
                    losses += change.abs();
                }
            }
            
            let avg_gain = gains / period as f64;
            let avg_loss = losses / period as f64;
            
            if avg_loss == 0.0 {
                rsi_values.push(100.0);
            } else {
                let rs = avg_gain / avg_loss;
                let rsi = 100.0 - (100.0 / (1.0 + rs));
                rsi_values.push(rsi);
            }
        }
        
        rsi_values
    }

    // 简单移动平均线
    fn calculate_sma(&self, prices: &[f64], period: i32) -> Vec<f64> {
        let mut sma_values = Vec::new();
        
        for i in period as usize..prices.len() {
            let window = &prices[i - period as usize..i];
            let sum: f64 = window.iter().sum();
            sma_values.push(sum / period as f64);
        }
        
        sma_values
    }

    // 指数移动平均线
    fn calculate_ema(&self, prices: &[f64], period: i32) -> Vec<f64> {
        let mut ema_values = Vec::new();
        let multiplier = 2.0 / (period as f64 + 1.0);
        
        if prices.is_empty() {
            return ema_values;
        }
        
        ema_values.push(prices[0]);
        
        for i in 1..prices.len() {
            let ema = (prices[i] - ema_values[i - 1]) * multiplier + ema_values[i - 1];
            ema_values.push(ema);
        }
        
        ema_values
    }

    // 布林带计算
    fn calculate_bollinger_bands(&self, prices: &[f64], period: i32, std_dev: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut upper_band = Vec::new();
        let mut middle_band = Vec::new();
        let mut lower_band = Vec::new();
        
        for i in period as usize..prices.len() {
            let window = &prices[i - period as usize..i];
            let sum: f64 = window.iter().sum();
            let mean = sum / period as f64;
            
            let variance: f64 = window.iter().map(|price| (price - mean).powi(2)).sum::<f64>() / period as f64;
            let deviation = variance.sqrt();
            
            upper_band.push(mean + std_dev * deviation);
            middle_band.push(mean);
            lower_band.push(mean - std_dev * deviation);
        }
        
        (upper_band, middle_band, lower_band)
    }

    // MACD信号生成
    fn generate_macd_signal(&self, macd: f64, signal: f64, histogram: f64) -> String {
        if macd > signal && histogram > 0.0 {
            "买入".to_string()
        } else if macd < signal && histogram < 0.0 {
            "卖出".to_string()
        } else {
            "持有".to_string()
        }
    }

    // RSI信号生成
    fn generate_rsi_signal(&self, rsi: f64) -> String {
        if rsi >= self.rsi_overbought {
            "卖出".to_string()
        } else if rsi <= self.rsi_oversold {
            "买入".to_string()
        } else {
            "持有".to_string()
        }
    }

    // 移动平均线信号生成
    fn generate_ma_signal(&self, short_ma: &[f64], long_ma: &[f64]) -> (String, bool, bool) {
        if short_ma.len() < 2 || long_ma.len() < 2 {
            return ("持有".to_string(), false, false);
        }
        
        let prev_short = short_ma[short_ma.len() - 2];
        let curr_short = short_ma[short_ma.len() - 1];
        let prev_long = long_ma[long_ma.len() - 2];
        let curr_long = long_ma[long_ma.len() - 1];
        
        let golden_cross = prev_short <= prev_long && curr_short > curr_long;
        let death_cross = prev_short >= prev_long && curr_short < curr_long;
        
        let signal_type = if golden_cross {
            "买入".to_string()
        } else if death_cross {
            "卖出".to_string()
        } else {
            "持有".to_string()
        };
        
        (signal_type, golden_cross, death_cross)
    }

    // 布林带信号生成
    fn generate_bb_signal(&self, price: f64, upper: f64, lower: f64) -> String {
        if price >= upper {
            "卖出".to_string()
        } else if price <= lower {
            "买入".to_string()
        } else {
            "持有".to_string()
        }
    }

    // K线形态检测
    fn detect_kline_patterns(&self, price_data: &[PriceData]) -> Vec<String> {
        let mut patterns = Vec::new();
        
        if price_data.len() >= 3 {
            // 检测锤子线
            if self.is_hammer_pattern(&price_data[price_data.len() - 1]) {
                patterns.push("锤子线".to_string());
            }
            
            // 检测吊颈线
            if self.is_hanging_man_pattern(&price_data[price_data.len() - 1]) {
                patterns.push("吊颈线".to_string());
            }
            
            // 检测启明星
            if price_data.len() >= 3 && self.is_morning_star_pattern(&price_data[price_data.len() - 3..]) {
                patterns.push("启明星".to_string());
            }
        }
        
        patterns
    }

    // 反转形态检测
    fn detect_reversal_patterns(&self, price_data: &[PriceData]) -> Vec<String> {
        let mut patterns = Vec::new();
        
        if price_data.len() >= 3 {
            // 检测头肩顶
            if self.is_head_and_shoulders_pattern(price_data) {
                patterns.push("头肩顶".to_string());
            }
            
            // 检测头肩底
            if self.is_inverse_head_and_shoulders_pattern(price_data) {
                patterns.push("头肩底".to_string());
            }
        }
        
        patterns
    }

    // 持续形态检测
    fn detect_continuation_patterns(&self, price_data: &[PriceData]) -> Vec<String> {
        let mut patterns = Vec::new();
        
        if price_data.len() >= 3 {
            // 检测旗形
            if self.is_flag_pattern(price_data) {
                patterns.push("旗形".to_string());
            }
            
            // 检测三角形
            if self.is_triangle_pattern(price_data) {
                patterns.push("三角形".to_string());
            }
        }
        
        patterns
    }

    // 成交量比率计算
    fn calculate_volume_ratio(&self, price_data: &[PriceData]) -> f64 {
        if price_data.len() < 10 {
            return 1.0;
        }
        
        let recent_volume: f64 = price_data.iter().rev().take(5).map(|p| p.volume as f64).sum::<f64>() / 5.0;
        let avg_volume: f64 = price_data.iter().rev().take(10).map(|p| p.volume as f64).sum::<f64>() / 10.0;
        
        if avg_volume > 0.0 {
            recent_volume / avg_volume
        } else {
            1.0
        }
    }

    // 成交量趋势分析
    fn analyze_volume_trend(&self, price_data: &[PriceData]) -> String {
        if price_data.len() < 5 {
            return "未知".to_string();
        }
        
        let volumes: Vec<f64> = price_data.iter().rev().take(5).map(|p| p.volume as f64).collect();
        let trend = self.calculate_linear_trend(&volumes);
        
        match trend {
            t if t > 0.1 => "放量".to_string(),
            t if t < -0.1 => "缩量".to_string(),
            _ => "平量".to_string(),
        }
    }

    // 资金流量指数计算
    fn calculate_money_flow_index(&self, price_data: &[PriceData]) -> f64 {
        if price_data.len() < 14 {
            return 50.0;
        }
        
        let mut positive_flow = 0.0;
        let mut negative_flow = 0.0;
        
        for i in 1..price_data.len() {
            let typical_price = (price_data[i].high + price_data[i].low + price_data[i].close) / 3.0;
            let prev_typical_price = (price_data[i-1].high + price_data[i-1].low + price_data[i-1].close) / 3.0;
            
            let money_flow = typical_price * price_data[i].volume as f64;
            
            if typical_price > prev_typical_price {
                positive_flow += money_flow;
            } else if typical_price < prev_typical_price {
                negative_flow += money_flow;
            }
        }
        
        if negative_flow == 0.0 {
            100.0
        } else {
            let money_ratio = positive_flow / negative_flow;
            100.0 - (100.0 / (1.0 + money_ratio))
        }
    }

    // 累积/派发线计算
    fn calculate_accumulation_distribution(&self, price_data: &[PriceData]) -> f64 {
        let mut ad_line = 0.0;
        
        for i in 1..price_data.len() {
            let close = price_data[i].close;
            let low = price_data[i].low;
            let high = price_data[i].high;
            let volume = price_data[i].volume as f64;
            
            let clv = if high != low {
                ((close - low) - (high - close)) / (high - low)
            } else {
                0.0
            };
            
            ad_line += clv * volume;
        }
        
        ad_line
    }

    // 信号强度计算
    fn calculate_signal_strength(&self, signal_type: &str) -> f64 {
        match signal_type {
            "买入" => 75.0,
            "卖出" => 75.0,
            "强烈买入" => 90.0,
            "强烈卖出" => 90.0,
            _ => 50.0,
        }
    }

    // 风险等级计算
    fn calculate_risk_level(&self, signal_type: &str) -> String {
        match signal_type {
            "买入" | "卖出" => "中等".to_string(),
            "强烈买入" | "强烈卖出" => "高".to_string(),
            _ => "低".to_string(),
        }
    }

    // 预期盈利计算
    fn calculate_expected_profit(&self, signal_type: &str, current_price: f64) -> f64 {
        match signal_type {
            "买入" => current_price * 0.05,  // 5%预期盈利
            "强烈买入" => current_price * 0.08, // 8%预期盈利
            "卖出" => current_price * 0.05,   // 5%预期盈利
            "强烈卖出" => current_price * 0.08,  // 8%预期盈利
            _ => 0.0,
        }
    }

    // 止损计算
    fn calculate_stop_loss(&self, signal_type: &str, current_price: f64) -> f64 {
        match signal_type {
            "买入" | "强烈买入" => current_price * 0.95,  // 5%止损
            "卖出" | "强烈卖出" => current_price * 1.05,  // 5%止损
            _ => current_price,
        }
    }

    // 止盈计算
    fn calculate_take_profit(&self, signal_type: &str, current_price: f64) -> f64 {
        match signal_type {
            "买入" | "强烈买入" => current_price * 1.08,  // 8%止盈
            "卖出" | "强烈卖出" => current_price * 0.92,  // 8%止盈
            _ => current_price,
        }
    }

    // 线性趋势计算
    fn calculate_linear_trend(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        
        let n = values.len() as f64;
        let sum_x = n * (n - 1.0) / 2.0;
        let sum_y = values.iter().sum::<f64>();
        let sum_xy = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum::<f64>();
        let sum_x2 = (0..values.len()).map(|i| (i as f64).powi(2)).sum::<f64>();
        
        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = n * sum_x2 - sum_x.powi(2);
        
        if denominator != 0.0 {
            numerator / denominator
        } else {
            0.0
        }
    }

    // 形态检测辅助函数
    fn is_hammer_pattern(&self, candle: &PriceData) -> bool {
        let body = (candle.close - candle.open).abs();
        let lower_shadow = candle.open.min(candle.close) - candle.low;
        let upper_shadow = candle.high - candle.open.max(candle.close);
        
        lower_shadow > 2.0 * body && upper_shadow < 0.1 * body
    }

    fn is_hanging_man_pattern(&self, candle: &PriceData) -> bool {
        self.is_hammer_pattern(candle) // 形态相同，但出现在上涨趋势中
    }

    fn is_morning_star_pattern(&self, candles: &[PriceData]) -> bool {
        if candles.len() < 3 {
            return false;
        }
        
        let first = &candles[0];
        let second = &candles[1];
        let third = &candles[2];
        
        // 第一根是阴线，第二根是小实体，第三根是阳线
        first.close < first.open && 
        (second.close - second.open).abs() < (first.close - first.open).abs() * 0.5 &&
        third.close > third.open &&
        third.close > first.open
    }

    fn is_head_and_shoulders_pattern(&self, price_data: &[PriceData]) -> bool {
        // 简化的头肩顶检测
        if price_data.len() < 5 {
            return false;
        }
        
        let prices: Vec<f64> = price_data.iter().map(|p| p.high).collect();
        let left_shoulder = prices[prices.len() - 5];
        let head = prices[prices.len() - 3];
        let right_shoulder = prices[prices.len() - 1];
        
        head > left_shoulder && head > right_shoulder && (left_shoulder - right_shoulder).abs() < left_shoulder * 0.1
    }

    fn is_inverse_head_and_shoulders_pattern(&self, price_data: &[PriceData]) -> bool {
        // 简化的头肩底检测
        if price_data.len() < 5 {
            return false;
        }
        
        let prices: Vec<f64> = price_data.iter().map(|p| p.low).collect();
        let left_shoulder = prices[prices.len() - 5];
        let head = prices[prices.len() - 3];
        let right_shoulder = prices[prices.len() - 1];
        
        head < left_shoulder && head < right_shoulder && (left_shoulder - right_shoulder).abs() < left_shoulder * 0.1
    }

    fn is_flag_pattern(&self, price_data: &[PriceData]) -> bool {
        // 简化的旗形检测
        price_data.len() >= 3 && price_data.iter().rev().take(3).all(|p| (p.close - p.open).abs() < p.open * 0.02)
    }

    fn is_triangle_pattern(&self, price_data: &[PriceData]) -> bool {
        // 简化的三角形检测
        if price_data.len() < 5 {
            return false;
        }
        
        let highs: Vec<f64> = price_data.iter().rev().take(5).map(|p| p.high).collect();
        let lows: Vec<f64> = price_data.iter().rev().take(5).map(|p| p.low).collect();
        
        let high_trend = self.calculate_linear_trend(&highs);
        let low_trend = self.calculate_linear_trend(&lows);
        
        high_trend < -0.01 && low_trend > 0.01
    }

    // 布林带带宽计算
    fn calculate_bandwidth(&self, upper: f64, lower: f64, middle: f64) -> f64 {
        if middle != 0.0 {
            (upper - lower) / middle
        } else {
            0.0
        }
    }

    // 布林带挤压检测
    fn detect_bb_squeeze(&self, bandwidth: &[f64]) -> bool {
        if bandwidth.len() < 20 {
            return false;
        }
        
        let current_bandwidth = bandwidth.last().unwrap_or(&0.0);
        let avg_bandwidth = bandwidth.iter().rev().take(20).sum::<f64>() / 20.0;
        
        *current_bandwidth < avg_bandwidth * 0.8
    }

    // 成交量突破检测
    fn detect_volume_breakouts(&self, price_data: &[PriceData]) -> bool {
        if price_data.len() < 10 {
            return false;
        }
        
        let recent_volume = price_data.last().unwrap().volume;
        let avg_volume = price_data.iter().rev().take(10).map(|p| p.volume).sum::<i64>() / 10;
        
        recent_volume > avg_volume * 2
    }

    // K线信号生成
    fn generate_kline_signal(&self, patterns: &[String], reversal_patterns: &[String]) -> String {
        if !reversal_patterns.is_empty() {
            "反转信号".to_string()
        } else if !patterns.is_empty() {
            "形态信号".to_string()
        } else {
            "持有".to_string()
        }
    }

    // 成交量信号生成
    fn generate_volume_signal(&self, volume_ratio: f64, volume_trend: &str, mfi: f64) -> String {
        if volume_ratio > 2.0 && volume_trend == "放量" {
            "买入".to_string()
        } else if volume_ratio > 2.0 && volume_trend == "缩量" {
            "卖出".to_string()
        } else if mfi > 80.0 {
            "卖出".to_string()
        } else if mfi < 20.0 {
            "买入".to_string()
        } else {
            "持有".to_string()
        }
    }

    // 形态可靠性计算
    fn calculate_pattern_reliability(&self, patterns: &[String]) -> f64 {
        if patterns.is_empty() {
            return 0.0;
        }
        
        let mut reliability = 0.0;
        for pattern in patterns {
            match pattern.as_str() {
                "锤子线" | "启明星" => reliability += 70.0,
                "头肩顶" | "头肩底" => reliability += 80.0,
                "旗形" | "三角形" => reliability += 60.0,
                _ => reliability += 50.0,
            }
        }
        
        reliability / patterns.len() as f64
    }

    // MACD背离检测
    fn detect_macd_divergence(&self, prices: &[f64], macd_line: &[f64]) -> bool {
        if prices.len() < 5 || macd_line.len() < 5 {
            return false;
        }
        
        let price_trend = self.calculate_linear_trend(&prices[prices.len() - 5..]);
        let macd_trend = self.calculate_linear_trend(&macd_line[macd_line.len() - 5..]);
        
        price_trend > 0.0 && macd_trend < 0.0 || price_trend < 0.0 && macd_trend > 0.0
    }

    // RSI背离检测
    fn detect_rsi_divergence(&self, prices: &[f64], rsi_values: &[f64]) -> bool {
        if prices.len() < 5 || rsi_values.len() < 5 {
            return false;
        }
        
        let price_trend = self.calculate_linear_trend(&prices[prices.len() - 5..]);
        let rsi_trend = self.calculate_linear_trend(&rsi_values[rsi_values.len() - 5..]);
        
        price_trend > 0.0 && rsi_trend < 0.0 || price_trend < 0.0 && rsi_trend > 0.0
    }

    // 置信度计算函数
    fn calculate_macd_confidence(&self, macd: &MACDStrategy) -> f64 {
        let base_confidence = 70.0;
        let divergence_bonus = if macd.divergence { 15.0 } else { 0.0 };
        let histogram_bonus = (macd.histogram.abs() * 10.0).min(15.0);
        
        (base_confidence + divergence_bonus + histogram_bonus).min(100.0)
    }

    fn calculate_rsi_confidence(&self, rsi: &RSIStrategy) -> f64 {
        let base_confidence = 65.0;
        let divergence_bonus = if rsi.divergence { 20.0 } else { 0.0 };
        let extremity_bonus = if rsi.current_rsi > 80.0 || rsi.current_rsi < 20.0 { 10.0 } else { 5.0 };
        
        f64::min(base_confidence + divergence_bonus + extremity_bonus, 100.0)
    }

    fn calculate_ma_confidence(&self, ma: &MovingAverageStrategy) -> f64 {
        let base_confidence = 60.0;
        let cross_bonus = if ma.golden_cross || ma.death_cross { 25.0 } else { 0.0 };
        let spread_bonus = ((ma.short_ma - ma.long_ma).abs() / ma.long_ma * 100.0).min(15.0);
        
        (base_confidence + cross_bonus + spread_bonus).min(100.0)
    }

    fn calculate_bb_confidence(&self, bb: &BollingerBandsStrategy) -> f64 {
        let base_confidence = 65.0;
        let squeeze_bonus = if bb.squeeze { 20.0 } else { 0.0 };
        let bandwidth_bonus = (bb.bandwidth * 10.0).min(15.0);
        
        (base_confidence + squeeze_bonus + bandwidth_bonus).min(100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_price_data() -> Vec<PriceData> {
        vec![
            PriceData {
                date: Utc::now(),
                open: 10.0,
                close: 10.5,
                high: 10.8,
                low: 9.8,
                volume: 100000,
                change_pct: 5.0,
                turnover: 1050000.0,
                turnover_rt: 2.5,
            },
            PriceData {
                date: Utc::now(),
                open: 10.5,
                close: 11.0,
                high: 11.2,
                low: 10.3,
                volume: 120000,
                change_pct: 4.8,
                turnover: 1320000.0,
                turnover_rt: 2.8,
            },
        ]
    }

    #[test]
    fn test_trading_strategies_analyzer_creation() {
        let analyzer = TradingStrategiesAnalyzer::new();
        assert_eq!(analyzer.rsi_overbought, 70.0);
        assert_eq!(analyzer.rsi_oversold, 30.0);
        assert_eq!(analyzer.macd_fast_period, 12);
        assert_eq!(analyzer.macd_slow_period, 26);
    }

    #[test]
    fn test_sma_calculation() {
        let analyzer = TradingStrategiesAnalyzer::new();
        let prices = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let sma = analyzer.calculate_sma(&prices, 3);
        
        assert_eq!(sma.len(), 4);
        assert!((sma[0] - 11.0).abs() < 0.001);
        assert!((sma[1] - 12.0).abs() < 0.001);
        assert!((sma[2] - 13.0).abs() < 0.001);
        assert!((sma[3] - 14.0).abs() < 0.001);
    }

    #[test]
    fn test_ema_calculation() {
        let analyzer = TradingStrategiesAnalyzer::new();
        let prices = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let ema = analyzer.calculate_ema(&prices, 3);
        
        assert_eq!(ema.len(), 6);
        assert_eq!(ema[0], 10.0);
    }

    #[test]
    fn test_macd_signal_generation() {
        let analyzer = TradingStrategiesAnalyzer::new();
        
        // 测试买入信号
        let buy_signal = analyzer.generate_macd_signal(1.0, 0.8, 0.2);
        assert_eq!(buy_signal, "买入");
        
        // 测试卖出信号
        let sell_signal = analyzer.generate_macd_signal(-1.0, -0.8, -0.2);
        assert_eq!(sell_signal, "卖出");
        
        // 测试持有信号
        let hold_signal = analyzer.generate_macd_signal(0.5, 0.6, -0.1);
        assert_eq!(hold_signal, "持有");
    }

    #[test]
    fn test_rsi_signal_generation() {
        let analyzer = TradingStrategiesAnalyzer::new();
        
        // 测试超买信号
        let overbought_signal = analyzer.generate_rsi_signal(75.0);
        assert_eq!(overbought_signal, "卖出");
        
        // 测试超卖信号
        let oversold_signal = analyzer.generate_rsi_signal(25.0);
        assert_eq!(oversold_signal, "买入");
        
        // 测试正常信号
        let normal_signal = analyzer.generate_rsi_signal(50.0);
        assert_eq!(normal_signal, "持有");
    }

    #[test]
    fn test_hammer_pattern_detection() {
        let analyzer = TradingStrategiesAnalyzer::new();
        
        // 创建锤子线
        let hammer = PriceData {
            date: Utc::now(),
            open: 10.0,
            close: 10.2,
            high: 10.3,
            low: 9.0,  // 长下影线
            volume: 100000,
            change_pct: 2.0,
            turnover: 1020000.0,
            turnover_rt: 2.0,
        };
        
        assert!(analyzer.is_hammer_pattern(&hammer));
    }
}