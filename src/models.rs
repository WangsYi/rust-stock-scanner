use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;

// Database models use String for UUID to maintain compatibility
// Application layer converts between String and Uuid as needed

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Market {
    ASHARES,  // A股
    HONGKONG, // 港股
    US,       // 美股
    UNKNOWN,
}

impl std::fmt::Display for Market {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Market::ASHARES => write!(f, "A股"),
            Market::HONGKONG => write!(f, "港股"),
            Market::US => write!(f, "美股"),
            Market::UNKNOWN => write!(f, "未知"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub date: DateTime<Utc>,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: i64,
    pub change_pct: f64,
    pub turnover: f64,
    pub turnover_rt: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialIndicator {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundamentalData {
    pub financial_indicators: Vec<FinancialIndicator>,
    pub valuation: HashMap<String, f64>,
    pub industry: String,
    pub sector: String,

    // Enhanced fundamental analysis
    pub performance_forecasts: PerformanceForecasts,
    pub risk_assessment: RiskAssessment,
    pub financial_health: FinancialHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceForecasts {
    pub revenue_growth_forecast: Option<f64>,
    pub earnings_growth_forecast: Option<f64>,
    pub target_price: Option<f64>,
    pub analyst_rating: String,
    pub forecast_period: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub beta: Option<f64>,
    pub debt_to_equity: Option<f64>,
    pub current_ratio: Option<f64>,
    pub quick_ratio: Option<f64>,
    pub interest_coverage: Option<f64>,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialHealth {
    pub profitability_score: f64,
    pub liquidity_score: f64,
    pub solvency_score: f64,
    pub efficiency_score: f64,
    pub overall_health_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalAnalysis {
    // Moving Averages
    pub ma5: f64,
    pub ma10: f64,
    pub ma20: f64,
    pub ma60: f64,
    pub ma120: f64,

    // Momentum Indicators
    pub rsi: f64,
    pub macd_signal: String,
    pub macd_line: f64,
    pub macd_histogram: f64,

    // Volatility Indicators
    pub bb_position: f64,
    pub bb_upper: f64,
    pub bb_middle: f64,
    pub bb_lower: f64,
    pub atr: f64,

    // Additional Indicators
    pub williams_r: f64,
    pub cci: f64,
    pub stochastic_k: f64,
    pub stochastic_d: f64,

    // Volume and Trend
    pub volume_status: String,
    pub ma_trend: String,
    pub adx: f64,
    pub trend_strength: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentAnalysis {
    pub overall_sentiment: f64,
    pub sentiment_trend: String,
    pub confidence_score: f64,
    pub total_analyzed: i32,
    pub sentiment_by_type: HashMap<String, f64>,
    pub news_distribution: HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct News {
    pub title: String,
    pub content: String,
    pub date: DateTime<Utc>,
    pub source: String,
    pub news_type: String,
    pub relevance: f64,
    pub sentiment: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInfo {
    pub current_price: f64,
    pub price_change: f64,
    pub volume_ratio: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisScores {
    pub technical: f64,
    pub fundamental: f64,
    pub sentiment: f64,
    pub comprehensive: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQuality {
    pub financial_indicators_count: i32,
    pub total_news_count: i32,
    pub analysis_completeness: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub stock_code: String,
    pub stock_name: String,
    pub market: Market,
    pub analysis_date: DateTime<Utc>,
    pub price_info: PriceInfo,
    pub technical: TechnicalAnalysis,
    pub fundamental: FundamentalData,
    pub sentiment: SentimentAnalysis,
    pub scores: AnalysisScores,
    pub recommendation: String,
    pub ai_analysis: String,
    pub data_quality: DataQuality,
    pub strategy_analysis: Option<StrategyAnalysis>, // 新增策略分析
    pub fallback_used: bool,
    pub fallback_reason: Option<String>,
}

impl Market {
    pub fn from_stock_code(stock_code: &str) -> Self {
        // A-share codes: 6-digit numbers starting with 0, 3, 6
        if stock_code.chars().all(|c| c.is_ascii_digit()) && stock_code.len() == 6 {
            match &stock_code[0..1] {
                "0" | "3" | "6" => Market::ASHARES,
                _ => Market::UNKNOWN,
            }
        }
        // Hong Kong codes: 5-digit numbers starting with 0, but different prefix
        else if stock_code.chars().all(|c| c.is_ascii_digit()) && stock_code.len() == 5 {
            Market::HONGKONG
        }
        // US stocks: typically 1-4 letters, sometimes with suffix
        else if stock_code.chars().all(|c| c.is_ascii_alphabetic()) && stock_code.len() <= 4 {
            Market::US
        }
        // Some US stocks have dot notation
        else if stock_code.contains('.') {
            let parts: Vec<&str> = stock_code.split('.').collect();
            if parts.len() == 2 {
                match parts[1].to_uppercase().as_str() {
                    "US" | "NASDAQ" | "NYSE" | "AMEX" => Market::US,
                    "HK" | "HKEX" => Market::HONGKONG,
                    "SH" | "SZ" | "SS" | "SZSE" => Market::ASHARES,
                    _ => Market::UNKNOWN,
                }
            } else {
                Market::UNKNOWN
            }
        }
        // Some special cases
        else {
            match stock_code.to_uppercase().as_str() {
                "AAPL" | "MSFT" | "GOOGL" | "AMZN" | "TSLA" | "META" | "NVDA" | "JPM" | "JNJ"
                | "V" => Market::US,
                "00700" | "00941" | "03690" | "00388" | "00939" | "00005" | "01299" | "00883"
                | "00960" | "00772" => Market::HONGKONG,
                _ => Market::UNKNOWN,
            }
        }
    }

    pub fn get_currency(&self) -> &'static str {
        match self {
            Market::ASHARES => "CNY",
            Market::HONGKONG => "HKD",
            Market::US => "USD",
            Market::UNKNOWN => "USD",
        }
    }

    pub fn get_timezone(&self) -> &'static str {
        match self {
            Market::ASHARES => "Asia/Shanghai",
            Market::HONGKONG => "Asia/Hong_Kong",
            Market::US => "America/New_York",
            Market::UNKNOWN => "UTC",
        }
    }

    pub fn get_trading_hours(&self) -> (&'static str, &'static str) {
        match self {
            Market::ASHARES => ("09:30", "15:00"),
            Market::HONGKONG => ("09:30", "16:00"),
            Market::US => ("09:30", "16:00"),
            Market::UNKNOWN => ("00:00", "23:59"),
        }
    }

    pub fn get_market_name(&self) -> &'static str {
        match self {
            Market::ASHARES => "上海/深圳证券交易所",
            Market::HONGKONG => "香港交易所",
            Market::US => "纽约证券交易所/纳斯达克",
            Market::UNKNOWN => "未知市场",
        }
    }

    pub fn get_currency_symbol(&self) -> &'static str {
        match self {
            Market::ASHARES => "¥",
            Market::HONGKONG => "HK$",
            Market::US => "$",
            Market::UNKNOWN => "$",
        }
    }

    pub fn get_currency_name(&self) -> &'static str {
        match self {
            Market::ASHARES => "人民币",
            Market::HONGKONG => "港币",
            Market::US => "美元",
            Market::UNKNOWN => "美元",
        }
    }

    pub fn is_trading_day(&self, date: NaiveDate) -> bool {
        // Basic trading day logic (weekdays only for now)
        let weekday = date.weekday();
        match weekday {
            chrono::Weekday::Sat | chrono::Weekday::Sun => false,
            _ => true,
        }
    }

    pub fn is_market_open(&self, time: chrono::DateTime<chrono::Utc>) -> bool {
        let (open_time, close_time) = self.get_trading_hours();
        let market_time = time.with_timezone(&chrono::Local);

        let open_hour = open_time[..2].parse::<u32>().unwrap_or(9);
        let open_min = open_time[3..].parse::<u32>().unwrap_or(30);
        let close_hour = close_time[..2].parse::<u32>().unwrap_or(15);
        let close_min = close_time[3..].parse::<u32>().unwrap_or(0);

        let current_hour = market_time.hour();
        let current_min = market_time.minute();

        let current_time = current_hour * 60 + current_min;
        let open_minutes = open_hour * 60 + open_min;
        let close_minutes = close_hour * 60 + close_min;

        current_time >= open_minutes && current_time <= close_minutes
    }

    pub fn get_next_trading_day(&self, date: NaiveDate) -> NaiveDate {
        let mut next_day = date.succ_opt().unwrap_or(date);

        while !self.is_trading_day(next_day) {
            next_day = next_day.succ_opt().unwrap_or(next_day);
        }

        next_day
    }

    pub fn get_holidays(&self, year: i32) -> Vec<NaiveDate> {
        // Basic holiday list - in a real implementation, this would come from an API
        match self {
            Market::ASHARES => {
                vec![
                    // Chinese New Year (simplified)
                    NaiveDate::from_ymd_opt(year, 2, 10).unwrap(),
                    NaiveDate::from_ymd_opt(year, 2, 11).unwrap(),
                    NaiveDate::from_ymd_opt(year, 2, 12).unwrap(),
                    // National Day
                    NaiveDate::from_ymd_opt(year, 10, 1).unwrap(),
                    NaiveDate::from_ymd_opt(year, 10, 2).unwrap(),
                    NaiveDate::from_ymd_opt(year, 10, 3).unwrap(),
                ]
            }
            Market::HONGKONG => {
                vec![
                    // Some Hong Kong holidays
                    NaiveDate::from_ymd_opt(year, 1, 1).unwrap(), // New Year
                    NaiveDate::from_ymd_opt(year, 12, 25).unwrap(), // Christmas
                    NaiveDate::from_ymd_opt(year, 12, 26).unwrap(), // Boxing Day
                ]
            }
            Market::US => {
                vec![
                    // US holidays
                    NaiveDate::from_ymd_opt(year, 1, 1).unwrap(), // New Year
                    NaiveDate::from_ymd_opt(year, 7, 4).unwrap(), // Independence Day
                    NaiveDate::from_ymd_opt(year, 12, 25).unwrap(), // Christmas
                ]
            }
            Market::UNKNOWN => vec![],
        }
    }

    pub fn get_market_indicators(&self) -> Vec<&'static str> {
        match self {
            Market::ASHARES => vec!["上证指数", "深证成指", "创业板指", "科创50", "北证50"],
            Market::HONGKONG => vec!["恒生指数", "国企指数", "红筹指数", "恒生科技指数"],
            Market::US => vec!["道琼斯指数", "标普500指数", "纳斯达克指数", "罗素2000指数"],
            Market::UNKNOWN => vec!["未知指数"],
        }
    }

    pub fn get_trading_sessions(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Market::ASHARES => vec![("09:30", "11:30"), ("13:00", "15:00")],
            Market::HONGKONG => vec![("09:30", "12:00"), ("13:00", "16:00")],
            Market::US => vec![("09:30", "16:00")],
            Market::UNKNOWN => vec![("00:00", "23:59")],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleAnalysisRequest {
    pub stock_code: String,
    pub enable_ai: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAnalysisRequest {
    pub stock_codes: Vec<String>,
    pub enable_ai: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    pub task_id: String,
    pub status: String,
    pub progress: f64,
    pub total_stocks: i32,
    pub completed: i32,
    pub failed: i32,
    pub current_stock: Option<String>,
    pub start_time: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub task_id: String,
    pub current: i32,
    pub total: i32,
    pub percentage: f64,
    pub status: String,
    pub current_stock: Option<String>,
    pub message: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub analysis_report: Option<AnalysisReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    pub fn error(error: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error),
            message: None,
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub analysis: AnalysisConfig,
    pub akshare: AkshareConfig,
    pub ai: AIConfig,
    pub auth: AuthConfig,
    pub database: DatabaseConfig,
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub provider: String, // "openai", "claude", "baidu", "tencent", etc.
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub enabled: bool,
    pub timeout_seconds: u64,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: "".to_string(),
            base_url: None,
            model: None,
            enabled: true,
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub secret_key: String,
    pub session_timeout: u64,
    pub bcrypt_cost: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_admin: bool,
    pub api_usage: i64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_admin: bool,
    pub api_usage: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub max_workers: usize,
    pub timeout_seconds: u64,
    pub weights: AnalysisWeights,
    pub parameters: AnalysisParameters,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            max_workers: 10,
            timeout_seconds: 30,
            weights: AnalysisWeights {
                technical: 0.5,
                fundamental: 0.3,
                sentiment: 0.2,
            },
            parameters: AnalysisParameters {
                technical_period_days: 60,
                sentiment_period_days: 30,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisWeights {
    pub technical: f64,
    pub fundamental: f64,
    pub sentiment: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisParameters {
    pub technical_period_days: i32,
    pub sentiment_period_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AkshareConfig {
    pub proxy_url: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub enable_migrations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub price_data_ttl: i64,
    pub fundamental_data_ttl: i64,
    pub news_data_ttl: i64,
    pub stock_name_ttl: i64,
    pub max_entries: usize,
    pub cleanup_interval: i64,
    pub enable_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            price_data_ttl: 300,
            fundamental_data_ttl: 3600,
            news_data_ttl: 1800,
            stock_name_ttl: 86400,
            max_entries: 1000,
            cleanup_interval: 60,
            enable_stats: true,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: Some(4),
            },
            analysis: AnalysisConfig {
                max_workers: 10,
                timeout_seconds: 30,
                weights: AnalysisWeights {
                    technical: 0.5,
                    fundamental: 0.3,
                    sentiment: 0.2,
                },
                parameters: AnalysisParameters {
                    technical_period_days: 60,
                    sentiment_period_days: 30,
                },
            },
            akshare: AkshareConfig {
                proxy_url: "http://localhost:5000".to_string(),
                timeout_seconds: 30,
            },
            ai: AIConfig {
                provider: "openai".to_string(),
                api_key: "".to_string(),
                base_url: None,
                model: Some("gpt-3.5-turbo".to_string()),
                enabled: true,
                timeout_seconds: 30,
            },
            auth: AuthConfig {
                enabled: false,
                secret_key: "your-secret-key-change-this".to_string(),
                session_timeout: 86400,
                bcrypt_cost: 12,
            },
            database: DatabaseConfig {
                url: "postgres://localhost:5432/stock_analyzer".to_string(),
                max_connections: 5,
                enable_migrations: true,
            },
            cache: CacheConfig::default(),
        }
    }
}

pub type TaskId = String;
pub type StockCode = String;

impl Default for PerformanceForecasts {
    fn default() -> Self {
        PerformanceForecasts {
            revenue_growth_forecast: None,
            earnings_growth_forecast: None,
            target_price: None,
            analyst_rating: "未评级".to_string(),
            forecast_period: "12个月".to_string(),
        }
    }
}

impl Default for RiskAssessment {
    fn default() -> Self {
        RiskAssessment {
            beta: None,
            debt_to_equity: None,
            current_ratio: None,
            quick_ratio: None,
            interest_coverage: None,
            risk_level: "中等".to_string(),
        }
    }
}

impl Default for FinancialHealth {
    fn default() -> Self {
        FinancialHealth {
            profitability_score: 50.0,
            liquidity_score: 50.0,
            solvency_score: 50.0,
            efficiency_score: 50.0,
            overall_health_score: 50.0,
        }
    }
}

// Database models for persistent storage

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SavedAnalysis {
    pub id: String,
    pub stock_code: String,
    pub stock_name: String,
    pub analysis_date: DateTime<Utc>,
    pub price_info: serde_json::Value,
    pub technical: serde_json::Value,
    pub fundamental: serde_json::Value,
    pub sentiment: serde_json::Value,
    pub scores: serde_json::Value,
    pub recommendation: String,
    pub ai_analysis: String,
    pub data_quality: serde_json::Value,
    pub ai_provider: Option<String>,
    pub ai_model: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SavedConfiguration {
    pub id: String,
    pub config_type: String,
    pub config_name: String,
    pub config_data: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryQuery {
    pub stock_code: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub analyses: Vec<SavedAnalysis>,
    pub total: i64,
    pub query: HistoryQuery,
}

// Currency conversion query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyConversionQuery {
    pub amount: f64,
    pub from_currency: String,
    pub to_currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateQuery {
    pub from_currency: String,
    pub to_currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTimeQuery {
    pub stock_code: String,
}

// Currency conversion response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyConversionResponse {
    pub original_amount: f64,
    pub from_currency: String,
    pub to_currency: String,
    pub converted_amount: f64,
    pub exchange_rate: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRateResponse {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: f64,
    pub timestamp: DateTime<Utc>,
}

// 主力筹码监控相关数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipDistribution {
    pub price_range: String,      // 价格区间
    pub chip_percentage: f64,     // 筹码占比
    pub volume: i64,              // 成交量
    pub turnover_rate: f64,      // 换手率
    pub avg_cost: f64,            // 平均成本
    pub concentration: f64,       // 集中度
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalFlow {
    pub main_force_inflow: f64,   // 主力资金流入
    pub main_force_outflow: f64,  // 主力资金流出
    pub retail_inflow: f64,       // 散户资金流入
    pub retail_outflow: f64,      // 散户资金流出
    pub net_inflow: f64,          // 净流入
    pub inflow_trend: String,     // 流入趋势
    pub concentration_index: f64, // 资金集中度指数
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipAnalysis {
    pub distribution: Vec<ChipDistribution>,     // 筹码分布
    pub capital_flow: CapitalFlow,              // 资金流向
    pub average_cost: f64,                       // 平均持仓成本
    pub profit_ratio: f64,                       // 盈利比例
    pub loss_ratio: f64,                         // 亏损比例
    pub concentration_degree: f64,              // 筹码集中度
    pub chip_signal: String,                     // 筹码信号
    pub support_level: f64,                      // 支撑位
    pub resistance_level: f64,                  // 阻力位
}

// 交易策略相关数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSignal {
    pub strategy_name: String,      // 策略名称
    pub signal_type: String,       // 信号类型: "买入", "卖出", "持有"
    pub strength: f64,             // 信号强度 (0-100)
    pub price: f64,                // 信号价格
    pub timestamp: DateTime<Utc>,   // 信号时间
    pub reason: String,             // 信号原因
    pub confidence: f64,           // 置信度 (0-100)
    pub risk_level: String,        // 风险等级
    pub expected_profit: f64,      // 预期盈利
    pub stop_loss: f64,            // 止损位
    pub take_profit: f64,          // 止盈位
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub name: String,              // 策略名称
    pub enabled: bool,             // 是否启用
    pub parameters: serde_json::Value, // 策略参数
    pub risk_tolerance: f64,       // 风险容忍度
    pub max_position: f64,         // 最大仓位
    pub stop_loss_ratio: f64,     // 止损比例
    pub take_profit_ratio: f64,    // 止盈比例
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MACDStrategy {
    pub fast_period: i32,          // 快线周期
    pub slow_period: i32,          // 慢线周期
    pub signal_period: i32,        // 信号线周期
    pub current_macd: f64,         // 当前MACD值
    pub current_signal: f64,       // 当前信号线值
    pub histogram: f64,            // 当前柱状图值
    pub signal_type: String,        // 信号类型
    pub divergence: bool,          // 是否背离
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSIStrategy {
    pub period: i32,               // 计算周期
    pub current_rsi: f64,          // 当前RSI值
    pub overbought: f64,           // 超买线
    pub oversold: f64,             // 超卖线
    pub signal_type: String,       // 信号类型
    pub divergence: bool,          // 是否背离
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovingAverageStrategy {
    pub short_period: i32,         // 短期均线周期
    pub long_period: i32,          // 长期均线周期
    pub short_ma: f64,             // 短期均线值
    pub long_ma: f64,              // 长期均线值
    pub signal_type: String,       // 信号类型
    pub golden_cross: bool,        // 是否金叉
    pub death_cross: bool,         // 是否死叉
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingStrategies {
    pub macd: MACDStrategy,        // MACD策略
    pub rsi: RSIStrategy,          // RSI策略
    pub moving_average: MovingAverageStrategy, // 均线策略
    pub bollinger_bands: BollingerBandsStrategy, // 布林带策略
    pub kline_patterns: KlinePatternsStrategy,   // K线形态策略
    pub volume_analysis: VolumeAnalysisStrategy, // 成交量分析策略
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BollingerBandsStrategy {
    pub period: i32,               // 计算周期
    pub std_dev: f64,              // 标准差倍数
    pub upper_band: f64,           // 上轨
    pub middle_band: f64,          // 中轨
    pub lower_band: f64,           // 下轨
    pub bandwidth: f64,            // 带宽
    pub signal_type: String,       // 信号类型
    pub squeeze: bool,              // 是否挤压
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlinePatternsStrategy {
    pub patterns: Vec<String>,     // 识别到的形态
    pub reversal_patterns: Vec<String>, // 反转形态
    pub continuation_patterns: Vec<String>, // 持续形态
    pub signal_type: String,       // 信号类型
    pub reliability: f64,         // 可靠性评分
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeAnalysisStrategy {
    pub volume_ratio: f64,         // 成交量比率
    pub volume_trend: String,      // 成交量趋势
    pub money_flow_index: f64,     // 资金流量指数
    pub accumulation_distribution: f64, // 累积/派发线
    pub signal_type: String,       // 信号类型
    pub breakouts: bool,           // 是否突破
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalAlert {
    pub id: String,                // 唯一标识
    pub stock_code: String,        // 股票代码
    pub stock_name: String,        // 股票名称
    pub signal_type: String,       // 信号类型
    pub signal_strength: f64,      // 信号强度
    pub price: f64,                // 当前价格
    pub target_price: f64,         // 目标价格
    pub stop_loss: f64,            // 止损价格
    pub strategy_name: String,      // 策略名称
    pub reason: String,            // 信号原因
    pub confidence: f64,           // 置信度
    pub created_at: DateTime<Utc>, // 创建时间
    pub expires_at: DateTime<Utc>,  // 过期时间
    pub is_active: bool,           // 是否激活
    pub notification_sent: bool,   // 通知已发送
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAnalysis {
    pub chip_analysis: ChipAnalysis,                    // 筹码分析
    pub trading_strategies: TradingStrategies,          // 交易策略
    pub signals: Vec<TradingSignal>,                    // 交易信号
    pub alerts: Vec<SignalAlert>,                       // 信号提醒
    pub overall_signal: String,                          // 整体信号
    pub recommendation: String,                         // 操作建议
    pub risk_assessment: String,                        // 风险评估
    pub market_sentiment: String,                       // 市场情绪
    pub execution_plan: String,                         // 执行计划
}
