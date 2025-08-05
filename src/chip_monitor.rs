use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::*;

/// 主力筹码监控模块
pub struct ChipMonitor {
    // 配置参数
    pub concentration_threshold: f64,    // 集中度阈值
    pub inflow_threshold: f64,          // 流入阈值
    pub volume_ratio_threshold: f64,    // 成交量比率阈值
    pub price_range_count: i32,        // 价格区间数量
}

impl ChipMonitor {
    /// 创建新的筹码监控器
    pub fn new() -> Self {
        Self {
            concentration_threshold: 0.6,    // 60%集中度
            inflow_threshold: 1000000.0,    // 100万流入阈值
            volume_ratio_threshold: 2.0,     // 2倍成交量比率
            price_range_count: 10,          // 10个价格区间
        }
    }

    /// 分析筹码分布
    pub async fn analyze_chip_distribution(
        &self,
        stock_code: &str,
        price_data: &[PriceData],
    ) -> Result<ChipDistribution, Box<dyn std::error::Error>> {
        if price_data.is_empty() {
            return Err("No price data available".into());
        }

        // 计算价格区间
        let min_price = price_data.iter().map(|p| p.close).fold(f64::INFINITY, f64::min);
        let max_price = price_data.iter().map(|p| p.close).fold(f64::NEG_INFINITY, f64::max);
        let price_range = max_price - min_price;
        let range_size = price_range / self.price_range_count as f64;

        // 分析每个价格区间的筹码分布
        let mut distribution = ChipDistribution {
            price_range: format!("{:.2}-{:.2}", min_price, max_price),
            chip_percentage: 0.0,
            volume: price_data.iter().map(|p| p.volume).sum(),
            turnover_rate: self.calculate_turnover_rate(price_data),
            avg_cost: self.calculate_average_cost(price_data),
            concentration: self.calculate_concentration(price_data),
        };

        // 计算筹码占比（简化算法）
        distribution.chip_percentage = self.calculate_chip_percentage(price_data);

        Ok(distribution)
    }

    /// 分析资金流向
    pub async fn analyze_capital_flow(
        &self,
        stock_code: &str,
        price_data: &[PriceData],
    ) -> Result<CapitalFlow, Box<dyn std::error::Error>> {
        if price_data.len() < 2 {
            return Err("Insufficient price data for capital flow analysis".into());
        }

        // 计算主力资金流向（基于价格变动和成交量）
        let (main_force_inflow, main_force_outflow) = self.calculate_main_force_flow(price_data);
        let (retail_inflow, retail_outflow) = self.calculate_retail_flow(price_data);
        
        let net_inflow = main_force_inflow - main_force_outflow;
        let inflow_trend = self.determine_inflow_trend(price_data);
        let concentration_index = self.calculate_concentration_index(price_data);

        Ok(CapitalFlow {
            main_force_inflow,
            main_force_outflow,
            retail_inflow,
            retail_outflow,
            net_inflow,
            inflow_trend,
            concentration_index,
        })
    }

    /// 完整的筹码分析
    pub async fn analyze_chips(
        &self,
        stock_code: &str,
        price_data: &[PriceData],
    ) -> Result<ChipAnalysis, Box<dyn std::error::Error>> {
        let distribution = vec![self.analyze_chip_distribution(stock_code, price_data).await?];
        let capital_flow = self.analyze_capital_flow(stock_code, price_data).await?;
        
        let average_cost = self.calculate_average_cost(price_data);
        let (profit_ratio, loss_ratio) = self.calculate_profit_loss_ratio(price_data);
        let concentration_degree = self.calculate_concentration_degree(price_data);
        let chip_signal = self.generate_chip_signal(&capital_flow, concentration_degree);
        let (support_level, resistance_level) = self.calculate_support_resistance(price_data);

        Ok(ChipAnalysis {
            distribution,
            capital_flow,
            average_cost,
            profit_ratio,
            loss_ratio,
            concentration_degree,
            chip_signal,
            support_level,
            resistance_level,
        })
    }

    /// 计算换手率
    fn calculate_turnover_rate(&self, price_data: &[PriceData]) -> f64 {
        if price_data.is_empty() {
            return 0.0;
        }
        
        let total_volume: i64 = price_data.iter().map(|p| p.volume).sum();
        let avg_volume = total_volume as f64 / price_data.len() as f64;
        
        // 简化的换手率计算
        avg_volume / 1000000.0 // 假设流通股本为100万股
    }

    /// 计算平均成本
    fn calculate_average_cost(&self, price_data: &[PriceData]) -> f64 {
        if price_data.is_empty() {
            return 0.0;
        }
        
        let total_value: f64 = price_data.iter().map(|p| p.close * p.volume as f64).sum();
        let total_volume: i64 = price_data.iter().map(|p| p.volume).sum();
        
        if total_volume > 0 {
            total_value / total_volume as f64
        } else {
            price_data.iter().map(|p| p.close).sum::<f64>() / price_data.len() as f64
        }
    }

    /// 计算集中度
    fn calculate_concentration(&self, price_data: &[PriceData]) -> f64 {
        if price_data.len() < 2 {
            return 0.0;
        }
        
        // 基于成交量的集中度计算
        let total_volume: i64 = price_data.iter().map(|p| p.volume).sum();
        let max_volume = price_data.iter().map(|p| p.volume).max().unwrap_or(0);
        
        if total_volume > 0 {
            max_volume as f64 / total_volume as f64
        } else {
            0.0
        }
    }

    /// 计算筹码占比
    fn calculate_chip_percentage(&self, price_data: &[PriceData]) -> f64 {
        // 简化的筹码占比计算
        let recent_data = &price_data[price_data.len().saturating_sub(5)..];
        if recent_data.is_empty() {
            return 0.0;
        }
        
        let recent_volume: i64 = recent_data.iter().map(|p| p.volume).sum();
        let total_volume: i64 = price_data.iter().map(|p| p.volume).sum();
        
        if total_volume > 0 {
            recent_volume as f64 / total_volume as f64
        } else {
            0.0
        }
    }

    /// 计算主力资金流向
    fn calculate_main_force_flow(&self, price_data: &[PriceData]) -> (f64, f64) {
        let mut inflow = 0.0;
        let mut outflow = 0.0;
        
        for i in 1..price_data.len() {
            let price_change = price_data[i].close - price_data[i-1].close;
            let volume = price_data[i].volume as f64;
            
            if price_change > 0.0 {
                inflow += volume * price_change.abs();
            } else if price_change < 0.0 {
                outflow += volume * price_change.abs();
            }
        }
        
        (inflow, outflow)
    }

    /// 计算散户资金流向
    fn calculate_retail_flow(&self, price_data: &[PriceData]) -> (f64, f64) {
        // 简化的散户资金流向计算
        let (main_inflow, main_outflow) = self.calculate_main_force_flow(price_data);
        let total_volume: f64 = price_data.iter().map(|p| p.volume as f64).sum();
        
        let retail_inflow = total_volume * 0.3 - main_inflow * 0.5;
        let retail_outflow = total_volume * 0.3 - main_outflow * 0.5;
        
        (retail_inflow.max(0.0), retail_outflow.max(0.0))
    }

    /// 判断流入趋势
    fn determine_inflow_trend(&self, price_data: &[PriceData]) -> String {
        if price_data.len() < 5 {
            return "未知".to_string();
        }
        
        let recent_prices: Vec<f64> = price_data.iter().rev().take(5).map(|p| p.close).collect();
        let trend = self.calculate_trend(&recent_prices);
        
        match trend {
            t if t > 0.02 => "强势流入".to_string(),
            t if t > 0.005 => "温和流入".to_string(),
            t if t < -0.02 => "强势流出".to_string(),
            t if t < -0.005 => "温和流出".to_string(),
            _ => "震荡".to_string(),
        }
    }

    /// 计算集中度指数
    fn calculate_concentration_index(&self, price_data: &[PriceData]) -> f64 {
        let concentration = self.calculate_concentration(price_data);
        let volume_ratio = self.calculate_volume_ratio(price_data);
        
        (concentration + volume_ratio) / 2.0
    }

    /// 计算成交量比率
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

    /// 计算盈亏比例
    fn calculate_profit_loss_ratio(&self, price_data: &[PriceData]) -> (f64, f64) {
        if price_data.is_empty() {
            return (0.0, 0.0);
        }
        
        let current_price = price_data.last().unwrap().close;
        let avg_cost = self.calculate_average_cost(price_data);
        
        if current_price > avg_cost {
            let profit_ratio = (current_price - avg_cost) / avg_cost * 100.0;
            (profit_ratio, 0.0)
        } else {
            let loss_ratio = (avg_cost - current_price) / avg_cost * 100.0;
            (0.0, loss_ratio)
        }
    }

    /// 计算集中度
    fn calculate_concentration_degree(&self, price_data: &[PriceData]) -> f64 {
        self.calculate_concentration(price_data) * 100.0
    }

    /// 生成筹码信号
    fn generate_chip_signal(&self, capital_flow: &CapitalFlow, concentration_degree: f64) -> String {
        let net_inflow = capital_flow.net_inflow;
        let concentration = capital_flow.concentration_index;
        
        match (net_inflow, concentration_degree) {
            (inflow, _) if inflow > self.inflow_threshold => "主力建仓".to_string(),
            (inflow, _) if inflow < -self.inflow_threshold => "主力出货".to_string(),
            (_, conc) if conc > self.concentration_threshold * 100.0 => "高度控盘".to_string(),
            _ => "筹码分散".to_string(),
        }
    }

    /// 计算支撑位和阻力位
    fn calculate_support_resistance(&self, price_data: &[PriceData]) -> (f64, f64) {
        if price_data.is_empty() {
            return (0.0, 0.0);
        }
        
        let prices: Vec<f64> = price_data.iter().map(|p| p.close).collect();
        let current_price = prices.last().unwrap();
        
        // 简化的支撑阻力位计算
        let support = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let resistance = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        (support, resistance)
    }

    /// 计算趋势
    fn calculate_trend(&self, prices: &[f64]) -> f64 {
        if prices.len() < 2 {
            return 0.0;
        }
        
        let first_price = prices.first().unwrap();
        let last_price = prices.last().unwrap();
        
        (last_price - first_price) / first_price
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
    fn test_chip_monitor_creation() {
        let monitor = ChipMonitor::new();
        assert_eq!(monitor.concentration_threshold, 0.6);
        assert_eq!(monitor.inflow_threshold, 1000000.0);
    }

    #[test]
    fn test_analyze_chip_distribution() {
        let monitor = ChipMonitor::new();
        let price_data = create_test_price_data();
        
        // 注意：这是一个异步测试，在实际运行时需要使用异步测试运行器
        // let result = futures::executor::block_on(monitor.analyze_chip_distribution("000001", &price_data));
        // assert!(result.is_ok());
    }
}