use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

use crate::models::*;

/// 交易信号提醒系统
pub struct SignalAlertSystem {
    // 信号历史记录
    pub signal_history: HashMap<String, VecDeque<TradingSignal>>,
    // 活跃提醒
    pub active_alerts: HashMap<String, SignalAlert>,
    // 配置参数
    pub alert_timeout_hours: i64,      // 提醒超时时间（小时）
    pub max_history_size: usize,      // 最大历史记录数量
    pub min_signal_strength: f64,     // 最小信号强度
    pub enable_notifications: bool,    // 是否启用通知
}

impl SignalAlertSystem {
    /// 创建新的信号提醒系统
    pub fn new() -> Self {
        Self {
            signal_history: HashMap::new(),
            active_alerts: HashMap::new(),
            alert_timeout_hours: 24,        // 24小时超时
            max_history_size: 100,         // 保存最近100个信号
            min_signal_strength: 60.0,     // 60分以上才生成提醒
            enable_notifications: true,    // 默认启用通知
        }
    }

    /// 处理新的交易信号
    pub async fn process_trading_signals(
        &mut self,
        stock_code: &str,
        stock_name: &str,
        signals: Vec<TradingSignal>,
        current_price: f64,
    ) -> Vec<SignalAlert> {
        let mut new_alerts = Vec::new();
        
        for signal in signals {
            // 检查信号强度是否达到阈值
            if signal.strength < self.min_signal_strength {
                continue;
            }
            
            // 检查是否已经存在类似的活跃提醒
            if self.has_similar_active_alert(stock_code, &signal.signal_type, &signal.strategy_name) {
                continue;
            }
            
            // 创建新的提醒
            let alert = self.create_signal_alert(stock_code, stock_name, signal.clone(), current_price);
            
            // 保存到活跃提醒
            self.active_alerts.insert(alert.id.clone(), alert.clone());
            
            // 添加到历史记录
            self.add_to_signal_history(stock_code, signal.clone());
            
            new_alerts.push(alert);
        }
        
        // 清理过期的提醒
        self.cleanup_expired_alerts();
        
        new_alerts
    }

    /// 创建信号提醒
    fn create_signal_alert(
        &self,
        stock_code: &str,
        stock_name: &str,
        signal: TradingSignal,
        current_price: f64,
    ) -> SignalAlert {
        let id = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::hours(self.alert_timeout_hours);
        
        SignalAlert {
            id: id.clone(),
            stock_code: stock_code.to_string(),
            stock_name: stock_name.to_string(),
            signal_type: signal.signal_type.clone(),
            signal_strength: signal.strength,
            price: current_price,
            target_price: signal.take_profit,
            stop_loss: signal.stop_loss,
            strategy_name: signal.strategy_name,
            reason: signal.reason,
            confidence: signal.confidence,
            created_at: Utc::now(),
            expires_at,
            is_active: true,
            notification_sent: false,
        }
    }

    /// 检查是否存在类似的活跃提醒
    fn has_similar_active_alert(&self, stock_code: &str, signal_type: &str, strategy_name: &str) -> bool {
        self.active_alerts.values().any(|alert| {
            alert.stock_code == stock_code &&
            alert.signal_type == signal_type &&
            alert.strategy_name == strategy_name &&
            alert.is_active
        })
    }

    /// 添加信号到历史记录
    fn add_to_signal_history(&mut self, stock_code: &str, signal: TradingSignal) {
        let history = self.signal_history.entry(stock_code.to_string()).or_insert_with(VecDeque::new);
        history.push_back(signal);
        
        // 限制历史记录数量
        if history.len() > self.max_history_size {
            history.pop_front();
        }
    }

    /// 清理过期提醒
    fn cleanup_expired_alerts(&mut self) {
        let now = Utc::now();
        self.active_alerts.retain(|_, alert| {
            if alert.expires_at < now {
                // 将过期的提醒标记为非活跃
                false
            } else {
                true
            }
        });
    }

    /// 获取活跃提醒
    pub fn get_active_alerts(&self) -> Vec<&SignalAlert> {
        self.active_alerts.values()
            .filter(|alert| alert.is_active)
            .collect()
    }

    /// 获取特定股票的活跃提醒
    pub fn get_stock_alerts(&self, stock_code: &str) -> Vec<&SignalAlert> {
        self.active_alerts.values()
            .filter(|alert| alert.stock_code == stock_code && alert.is_active)
            .collect()
    }

    /// 获取信号历史记录
    pub fn get_signal_history(&self, stock_code: &str, limit: Option<usize>) -> Vec<&TradingSignal> {
        let limit = limit.unwrap_or(10);
        if let Some(history) = self.signal_history.get(stock_code) {
            history.iter().rev().take(limit).collect()
        } else {
            Vec::new()
        }
    }

    /// 取消提醒
    pub fn cancel_alert(&mut self, alert_id: &str) -> Result<(), String> {
        if let Some(mut alert) = self.active_alerts.get_mut(alert_id) {
            alert.is_active = false;
            Ok(())
        } else {
            Err("Alert not found".to_string())
        }
    }

    /// 更新提醒状态
    pub fn update_alert_status(&mut self, alert_id: &str, is_active: bool) -> Result<(), String> {
        if let Some(mut alert) = self.active_alerts.get_mut(alert_id) {
            alert.is_active = is_active;
            Ok(())
        } else {
            Err("Alert not found".to_string())
        }
    }

    /// 标记通知已发送
    pub fn mark_notification_sent(&mut self, alert_id: &str) -> Result<(), String> {
        if let Some(mut alert) = self.active_alerts.get_mut(alert_id) {
            alert.notification_sent = true;
            Ok(())
        } else {
            Err("Alert not found".to_string())
        }
    }

    /// 获取待发送的通知
    pub fn get_pending_notifications(&self) -> Vec<&SignalAlert> {
        self.active_alerts.values()
            .filter(|alert| alert.is_active && !alert.notification_sent && self.enable_notifications)
            .collect()
    }

    /// 分析信号频率和统计信息
    pub fn get_signal_statistics(&self, stock_code: &str) -> SignalStatistics {
        let history = if let Some(h) = self.signal_history.get(stock_code) {
            h
        } else {
            return SignalStatistics {
                total_signals: 0,
                buy_signals: 0,
                sell_signals: 0,
                avg_strength: 0.0,
                avg_confidence: 0.0,
                most_active_strategy: "无".to_string(),
                success_rate: 0.0,
                last_signal_time: None,
            };
        };
        
        let total_signals = history.len();
        let buy_signals = history.iter().filter(|s| s.signal_type == "买入" || s.signal_type == "强烈买入").count();
        let sell_signals = history.iter().filter(|s| s.signal_type == "卖出" || s.signal_type == "强烈卖出").count();
        
        let avg_strength = if total_signals > 0 {
            history.iter().map(|s| s.strength).sum::<f64>() / total_signals as f64
        } else {
            0.0
        };
        
        let avg_confidence = if total_signals > 0 {
            history.iter().map(|s| s.confidence).sum::<f64>() / total_signals as f64
        } else {
            0.0
        };
        
        let most_active_strategy = self.get_most_active_strategy(stock_code);
        let success_rate = self.calculate_success_rate(stock_code);
        
        SignalStatistics {
            total_signals,
            buy_signals,
            sell_signals,
            avg_strength,
            avg_confidence,
            most_active_strategy,
            success_rate,
            last_signal_time: history.back().map(|s| s.timestamp),
        }
    }

    /// 获取最活跃的策略
    fn get_most_active_strategy(&self, stock_code: &str) -> String {
        let history = if let Some(h) = self.signal_history.get(stock_code) {
            h
        } else {
            return "无".to_string();
        };
        
        if history.is_empty() {
            return "无".to_string();
        }
        
        let mut strategy_counts = HashMap::new();
        for signal in history {
            *strategy_counts.entry(&signal.strategy_name).or_insert(0) += 1;
        }
        
        strategy_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(strategy, _)| strategy.clone())
            .unwrap_or_else(|| "无".to_string())
    }

    /// 计算信号成功率（简化版本）
    fn calculate_success_rate(&self, stock_code: &str) -> f64 {
        // 这是一个简化的成功率计算
        // 在实际应用中，需要跟踪信号执行后的实际结果
        let history = if let Some(h) = self.signal_history.get(stock_code) {
            h
        } else {
            return 0.0;
        };
        
        if history.len() < 10 {
            return 0.0;
        }
        
        // 基于信号强度和置信度的估算
        let strong_signals = history.iter().filter(|s| s.strength >= 80.0 && s.confidence >= 80.0).count();
        
        strong_signals as f64 / history.len() as f64 * 100.0
    }

    /// 生成策略分析报告
    pub fn generate_strategy_analysis_report(
        &self,
        stock_code: &str,
        stock_name: &str,
        chip_analysis: &ChipAnalysis,
        trading_strategies: &TradingStrategies,
        signals: &[TradingSignal],
    ) -> StrategyAnalysis {
        let alerts = self.get_stock_alerts(stock_code).into_iter().cloned().collect();
        let overall_signal = self.generate_overall_signal(signals, chip_analysis);
        let recommendation = self.generate_recommendation(&overall_signal, chip_analysis);
        let risk_assessment = self.assess_risk(signals, chip_analysis);
        let market_sentiment = self.analyze_market_sentiment(chip_analysis, trading_strategies);
        let execution_plan = self.create_execution_plan(&recommendation, signals);
        
        StrategyAnalysis {
            chip_analysis: chip_analysis.clone(),
            trading_strategies: trading_strategies.clone(),
            signals: signals.to_vec(),
            alerts,
            overall_signal,
            recommendation,
            risk_assessment,
            market_sentiment,
            execution_plan,
        }
    }

    /// 生成整体信号
    fn generate_overall_signal(&self, signals: &[TradingSignal], chip_analysis: &ChipAnalysis) -> String {
        if signals.is_empty() {
            return "观望".to_string();
        }
        
        let buy_signals = signals.iter().filter(|s| s.signal_type.contains("买入")).count();
        let sell_signals = signals.iter().filter(|s| s.signal_type.contains("卖出")).count();
        
        // 结合筹码分析
        let chip_signal = &chip_analysis.chip_signal;
        
        match (buy_signals, sell_signals, chip_signal.as_str()) {
            (b, s, _) if b > s && b >= 2 => "强烈买入".to_string(),
            (b, s, _) if b > s => "买入".to_string(),
            (b, s, _) if s > b && s >= 2 => "强烈卖出".to_string(),
            (b, s, _) if s > b => "卖出".to_string(),
            (_, _, "主力建仓") => "买入".to_string(),
            (_, _, "主力出货") => "卖出".to_string(),
            _ => "观望".to_string(),
        }
    }

    /// 生成操作建议
    fn generate_recommendation(&self, overall_signal: &str, chip_analysis: &ChipAnalysis) -> String {
        match overall_signal {
            "强烈买入" => format!("建议积极买入，主力资金{}，筹码集中度{:.1}%", 
                if chip_analysis.capital_flow.net_inflow > 0.0 { "流入" } else { "流出" },
                chip_analysis.concentration_degree),
            "买入" => format!("建议适量买入，关注支撑位{:.2}", chip_analysis.support_level),
            "强烈卖出" => format!("建议立即卖出，主力资金{}，注意风险", 
                if chip_analysis.capital_flow.net_inflow > 0.0 { "流入" } else { "流出" }),
            "卖出" => format!("建议减仓，关注阻力位{:.2}", chip_analysis.resistance_level),
            _ => "建议观望，等待更好的时机".to_string(),
        }
    }

    /// 风险评估
    fn assess_risk(&self, signals: &[TradingSignal], chip_analysis: &ChipAnalysis) -> String {
        let high_risk_signals = signals.iter().filter(|s| s.risk_level == "高").count();
        let chip_concentration = chip_analysis.concentration_degree;
        
        match (high_risk_signals, chip_concentration) {
            (h, c) if h >= 2 || c > 80.0 => "高风险".to_string(),
            (h, c) if h >= 1 || c > 60.0 => "中等风险".to_string(),
            _ => "低风险".to_string(),
        }
    }

    /// 市场情绪分析
    fn analyze_market_sentiment(&self, chip_analysis: &ChipAnalysis, trading_strategies: &TradingStrategies) -> String {
        let net_inflow = chip_analysis.capital_flow.net_inflow;
        let rsi = trading_strategies.rsi.current_rsi;
        
        match (net_inflow, rsi) {
            (inflow, r) if inflow > 1000000.0 && r > 70.0 => "乐观但谨慎".to_string(),
            (inflow, r) if inflow > 1000000.0 && r < 30.0 => "乐观买入".to_string(),
            (inflow, r) if inflow < -1000000.0 && r > 70.0 => "悲观卖出".to_string(),
            (inflow, r) if inflow < -1000000.0 && r < 30.0 => "悲观但有机会".to_string(),
            _ => "中性".to_string(),
        }
    }

    /// 创建执行计划
    fn create_execution_plan(&self, recommendation: &str, signals: &[TradingSignal]) -> String {
        match recommendation {
            s if s.contains("强烈买入") => {
                let position = if signals.len() > 1 { "60%-80%" } else { "40%-60%" };
                format!("建议分批建仓，目标仓位{}，设置止损位", position)
            },
            s if s.contains("买入") => {
                format!("建议少量建仓，目标仓位30%-50%，严格止损")
            },
            s if s.contains("强烈卖出") => {
                format!("建议立即减仓或清仓，锁定利润")
            },
            s if s.contains("卖出") => {
                format!("建议逐步减仓，降低仓位至30%以下")
            },
            _ => "建议保持现有仓位，密切关注市场变化".to_string(),
        }
    }

    /// 设置配置参数
    pub fn set_config(&mut self, config: AlertConfig) {
        self.alert_timeout_hours = config.alert_timeout_hours;
        self.max_history_size = config.max_history_size;
        self.min_signal_strength = config.min_signal_strength;
        self.enable_notifications = config.enable_notifications;
    }

    /// 获取系统状态
    pub fn get_system_status(&self) -> SystemStatus {
        SystemStatus {
            active_alerts_count: self.active_alerts.values().filter(|a| a.is_active).count(),
            total_signals_processed: self.signal_history.values().map(|h| h.len()).sum(),
            pending_notifications: self.get_pending_notifications().len(),
            last_cleanup_time: Utc::now(),
            uptime_seconds: 0, // 需要在实际实现中跟踪启动时间
        }
    }
}

/// 信号统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalStatistics {
    pub total_signals: usize,                    // 总信号数
    pub buy_signals: usize,                      // 买入信号数
    pub sell_signals: usize,                     // 卖出信号数
    pub avg_strength: f64,                       // 平均信号强度
    pub avg_confidence: f64,                     // 平均置信度
    pub most_active_strategy: String,            // 最活跃策略
    pub success_rate: f64,                       // 成功率
    pub last_signal_time: Option<DateTime<Utc>>, // 最后信号时间
}

/// 提醒配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub alert_timeout_hours: i64,      // 提醒超时时间（小时）
    pub max_history_size: usize,        // 最大历史记录数量
    pub min_signal_strength: f64,       // 最小信号强度
    pub enable_notifications: bool,     // 是否启用通知
}

/// 系统状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub active_alerts_count: usize,        // 活跃提醒数量
    pub total_signals_processed: usize,     // 总处理信号数
    pub pending_notifications: usize,       // 待发送通知数
    pub last_cleanup_time: DateTime<Utc>,   // 最后清理时间
    pub uptime_seconds: u64,                 // 运行时间（秒）
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            alert_timeout_hours: 24,
            max_history_size: 100,
            min_signal_strength: 60.0,
            enable_notifications: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_signal() -> TradingSignal {
        TradingSignal {
            strategy_name: "MACD策略".to_string(),
            signal_type: "买入".to_string(),
            strength: 75.0,
            price: 10.0,
            timestamp: Utc::now(),
            reason: "MACD金叉".to_string(),
            confidence: 80.0,
            risk_level: "中等".to_string(),
            expected_profit: 0.5,
            stop_loss: 9.5,
            take_profit: 10.8,
        }
    }

    #[test]
    fn test_signal_alert_system_creation() {
        let system = SignalAlertSystem::new();
        assert_eq!(system.alert_timeout_hours, 24);
        assert_eq!(system.max_history_size, 100);
        assert_eq!(system.min_signal_strength, 60.0);
        assert!(system.enable_notifications);
    }

    #[test]
    fn test_alert_creation() {
        let system = SignalAlertSystem::new();
        let signal = create_test_signal();
        
        let alert = system.create_signal_alert("000001", "测试股票", signal, 10.0);
        
        assert!(!alert.id.is_empty());
        assert_eq!(alert.stock_code, "000001");
        assert_eq!(alert.stock_name, "测试股票");
        assert_eq!(alert.signal_type, "买入");
        assert!(alert.is_active);
        assert!(!alert.notification_sent);
    }

    #[test]
    fn test_signal_statistics() {
        let mut system = SignalAlertSystem::new();
        let signal = create_test_signal();
        
        // 添加信号到历史记录
        system.add_to_signal_history("000001", signal.clone());
        
        let stats = system.get_signal_statistics("000001");
        
        assert_eq!(stats.total_signals, 1);
        assert_eq!(stats.buy_signals, 1);
        assert_eq!(stats.sell_signals, 0);
        assert_eq!(stats.avg_strength, 75.0);
        assert_eq!(stats.avg_confidence, 80.0);
    }

    #[test]
    fn test_alert_config_default() {
        let config = AlertConfig::default();
        assert_eq!(config.alert_timeout_hours, 24);
        assert_eq!(config.max_history_size, 100);
        assert_eq!(config.min_signal_strength, 60.0);
        assert!(config.enable_notifications);
    }

    #[test]
    fn test_overall_signal_generation() {
        let system = SignalAlertSystem::new();
        
        // 测试买入信号
        let buy_signals = vec![
            TradingSignal {
                strategy_name: "MACD策略".to_string(),
                signal_type: "买入".to_string(),
                strength: 75.0,
                price: 10.0,
                timestamp: Utc::now(),
                reason: "MACD金叉".to_string(),
                confidence: 80.0,
                risk_level: "中等".to_string(),
                expected_profit: 0.5,
                stop_loss: 9.5,
                take_profit: 10.8,
            },
            TradingSignal {
                strategy_name: "RSI策略".to_string(),
                signal_type: "买入".to_string(),
                strength: 70.0,
                price: 10.0,
                timestamp: Utc::now(),
                reason: "RSI超卖".to_string(),
                confidence: 75.0,
                risk_level: "中等".to_string(),
                expected_profit: 0.4,
                stop_loss: 9.5,
                take_profit: 10.5,
            },
        ];
        
        let chip_analysis = ChipAnalysis {
            distribution: vec![],
            capital_flow: CapitalFlow {
                main_force_inflow: 1500000.0,
                main_force_outflow: 500000.0,
                retail_inflow: 300000.0,
                retail_outflow: 400000.0,
                net_inflow: 1000000.0,
                inflow_trend: "温和流入".to_string(),
                concentration_index: 0.65,
            },
            average_cost: 9.8,
            profit_ratio: 2.0,
            loss_ratio: 0.0,
            concentration_degree: 65.0,
            chip_signal: "主力建仓".to_string(),
            support_level: 9.5,
            resistance_level: 10.8,
        };
        
        let overall_signal = system.generate_overall_signal(&buy_signals, &chip_analysis);
        assert_eq!(overall_signal, "强烈买入");
    }
}