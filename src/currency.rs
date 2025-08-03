use chrono::{DateTime, Utc, NaiveDate, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::Market;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: f64,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct CurrencyConverter {
    rates: Arc<RwLock<HashMap<String, f64>>>,
    last_updated: Arc<RwLock<DateTime<Utc>>>,
    base_currency: String,
    cache_ttl_seconds: i64,
}

impl CurrencyConverter {
    pub fn new(base_currency: String, cache_ttl_seconds: i64) -> Self {
        let mut rates = HashMap::new();
        rates.insert(base_currency.clone(), 1.0); // Base currency to itself is 1.0
        
        // Initialize with some common exchange rates (in a real app, these would come from an API)
        rates.insert("CNY".to_string(), 0.14); // USD to CNY
        rates.insert("HKD".to_string(), 0.13); // USD to HKD
        rates.insert("EUR".to_string(), 1.08); // USD to EUR
        rates.insert("GBP".to_string(), 1.27); // USD to GBP
        rates.insert("JPY".to_string(), 0.0064); // USD to JPY
        
        Self {
            rates: Arc::new(RwLock::new(rates)),
            last_updated: Arc::new(RwLock::new(Utc::now())),
            base_currency,
            cache_ttl_seconds,
        }
    }

    pub async fn get_exchange_rate(&self, from_currency: &str, to_currency: &str) -> Result<f64, String> {
        if from_currency == to_currency {
            return Ok(1.0);
        }

        let rates = self.rates.read().await;
        
        // If both currencies are in our cache
        if let (Some(&from_rate), Some(&to_rate)) = (rates.get(from_currency), rates.get(to_currency)) {
            return Ok(to_rate / from_rate);
        }

        // If we have the inverse rate
        if let (Some(&to_rate), Some(&from_rate)) = (rates.get(to_currency), rates.get(from_currency)) {
            return Ok(from_rate / to_rate);
        }

        Err(format!("Exchange rate not found for {} to {}", from_currency, to_currency))
    }

    pub async fn convert_amount(&self, amount: f64, from_currency: &str, to_currency: &str) -> Result<f64, String> {
        let rate = self.get_exchange_rate(from_currency, to_currency).await?;
        Ok(amount * rate)
    }

    pub async fn convert_to_base(&self, amount: f64, currency: &str) -> Result<f64, String> {
        self.convert_amount(amount, currency, &self.base_currency).await
    }

    pub async fn convert_from_base(&self, amount: f64, currency: &str) -> Result<f64, String> {
        self.convert_amount(amount, &self.base_currency, currency).await
    }

    pub async fn convert_between_markets(&self, amount: f64, from_market: &Market, to_market: &Market) -> Result<f64, String> {
        let from_currency = from_market.get_currency();
        let to_currency = to_market.get_currency();
        self.convert_amount(amount, from_currency, to_currency).await
    }

    pub async fn update_rates(&self, new_rates: HashMap<String, f64>) -> Result<(), String> {
        let mut rates = self.rates.write().await;
        let mut last_updated = self.last_updated.write().await;
        
        *rates = new_rates;
        *last_updated = Utc::now();
        
        Ok(())
    }

    pub async fn get_all_rates(&self) -> HashMap<String, f64> {
        self.rates.read().await.clone()
    }

    pub async fn get_last_updated(&self) -> DateTime<Utc> {
        *self.last_updated.read().await
    }

    pub async fn is_cache_expired(&self) -> bool {
        let last_updated = self.last_updated.read().await;
        Utc::now().signed_duration_since(*last_updated).num_seconds() > self.cache_ttl_seconds
    }

    pub async fn get_supported_currencies(&self) -> Vec<String> {
        let rates = self.rates.read().await;
        rates.keys().cloned().collect()
    }

    pub async fn format_currency(&self, amount: f64, currency: &str) -> String {
        let symbol = match currency {
            "USD" => "$",
            "CNY" => "¥",
            "HKD" => "HK$",
            "EUR" => "€",
            "GBP" => "£",
            "JPY" => "¥",
            _ => "$",
        };

        format!("{}{:.2}", symbol, amount)
    }

    pub async fn get_currency_info(&self, currency: &str) -> CurrencyInfo {
        match currency {
            "USD" => CurrencyInfo {
                name: "美元".to_string(),
                symbol: "$".to_string(),
                code: "USD".to_string(),
                country: "美国".to_string(),
                number_of_decimals: 2,
            },
            "CNY" => CurrencyInfo {
                name: "人民币".to_string(),
                symbol: "¥".to_string(),
                code: "CNY".to_string(),
                country: "中国".to_string(),
                number_of_decimals: 2,
            },
            "HKD" => CurrencyInfo {
                name: "港币".to_string(),
                symbol: "HK$".to_string(),
                code: "HKD".to_string(),
                country: "香港".to_string(),
                number_of_decimals: 2,
            },
            "EUR" => CurrencyInfo {
                name: "欧元".to_string(),
                symbol: "€".to_string(),
                code: "EUR".to_string(),
                country: "欧盟".to_string(),
                number_of_decimals: 2,
            },
            "GBP" => CurrencyInfo {
                name: "英镑".to_string(),
                symbol: "£".to_string(),
                code: "GBP".to_string(),
                country: "英国".to_string(),
                number_of_decimals: 2,
            },
            "JPY" => CurrencyInfo {
                name: "日元".to_string(),
                symbol: "¥".to_string(),
                code: "JPY".to_string(),
                country: "日本".to_string(),
                number_of_decimals: 0,
            },
            _ => CurrencyInfo {
                name: "未知货币".to_string(),
                symbol: "$".to_string(),
                code: currency.to_string(),
                country: "未知".to_string(),
                number_of_decimals: 2,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyInfo {
    pub name: String,
    pub symbol: String,
    pub code: String,
    pub country: String,
    pub number_of_decimals: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTimeInfo {
    pub market: Market,
    pub is_open: bool,
    pub current_time: DateTime<Utc>,
    pub local_time: String,
    pub next_session_open: Option<DateTime<Utc>>,
    pub next_session_close: Option<DateTime<Utc>>,
    pub is_trading_day: bool,
    pub current_session: Option<String>,
}

impl MarketTimeInfo {
    pub fn new(market: Market, current_time: DateTime<Utc>) -> Self {
        let is_trading_day = market.is_trading_day(current_time.date_naive());
        let is_open = if is_trading_day {
            market.is_market_open(current_time)
        } else {
            false
        };

        let sessions = market.get_trading_sessions();
        let current_session = if is_open {
            sessions.iter()
                .find(|(open, close)| {
                    let current_hour = current_time.hour();
                    let current_min = current_time.minute();
                    let current_time_minutes = current_hour * 60 + current_min;
                    
                    let open_hour = open[..2].parse::<u32>().unwrap_or(9);
                    let open_min = open[3..].parse::<u32>().unwrap_or(30);
                    let close_hour = close[..2].parse::<u32>().unwrap_or(16);
                    let close_min = close[3..].parse::<u32>().unwrap_or(0);
                    
                    let open_minutes = open_hour * 60 + open_min;
                    let close_minutes = close_hour * 60 + close_min;
                    
                    current_time_minutes >= open_minutes && current_time_minutes <= close_minutes
                })
                .map(|(open, close)| format!("{}-{}", open, close))
        } else {
            None
        };

        // Calculate next session times
        let (next_open, next_close) = if is_open {
            // Current session times - calculate when it closes
            if let Some((_, close)) = sessions.iter()
                .find(|(open, close)| {
                    let current_hour = current_time.hour();
                    let current_min = current_time.minute();
                    let current_time_minutes = current_hour * 60 + current_min;
                    
                    let open_hour = open[..2].parse::<u32>().unwrap_or(9);
                    let open_min = open[3..].parse::<u32>().unwrap_or(30);
                    let close_hour = close[..2].parse::<u32>().unwrap_or(16);
                    let close_min = close[3..].parse::<u32>().unwrap_or(0);
                    
                    let open_minutes = open_hour * 60 + open_min;
                    let close_minutes = close_hour * 60 + close_min;
                    
                    current_time_minutes >= open_minutes && current_time_minutes <= close_minutes
                }) {
                // Current session close time
                let close_hour = close[..2].parse::<u32>().unwrap_or(16);
                let close_min = close[3..].parse::<u32>().unwrap_or(0);
                let close_time = current_time.date_naive().and_hms_opt(close_hour, close_min, 0)
                    .map(|dt| dt.and_utc());
                (None, close_time)
            } else {
                (None, None)
            }
        } else {
            // Next trading day session times
            let next_trading_day = market.get_next_trading_day(current_time.date_naive());
            
            if let Some((open, close)) = sessions.first() {
                let open_hour = open[..2].parse::<u32>().unwrap_or(9);
                let open_min = open[3..].parse::<u32>().unwrap_or(30);
                let close_hour = close[..2].parse::<u32>().unwrap_or(16);
                let close_min = close[3..].parse::<u32>().unwrap_or(0);
                
                let next_open = next_trading_day.and_hms_opt(open_hour, open_min, 0)
                    .map(|dt| dt.and_utc());
                let next_close = next_trading_day.and_hms_opt(close_hour, close_min, 0)
                    .map(|dt| dt.and_utc());
                
                (next_open, next_close)
            } else {
                (None, None)
            }
        };

        // Format local time
        let local_time = current_time
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S %Z")
            .to_string();

        Self {
            market,
            is_open,
            current_time,
            local_time,
            next_session_open: next_open,
            next_session_close: next_close,
            is_trading_day,
            current_session,
        }
    }

    pub fn get_status_emoji(&self) -> &'static str {
        if self.is_open {
            "🟢"
        } else if self.is_trading_day {
            "🟡"
        } else {
            "🔴"
        }
    }

    pub fn get_status_text(&self) -> &'static str {
        if self.is_open {
            "交易中"
        } else if self.is_trading_day {
            "已收盘"
        } else {
            "休市"
        }
    }
}

// Default converter instance
pub fn get_default_converter() -> CurrencyConverter {
    CurrencyConverter::new("USD".to_string(), 3600) // 1 hour TTL
}