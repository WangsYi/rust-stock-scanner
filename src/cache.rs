use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::data_fetcher::DataFetcher;
use crate::models::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub hit_count: u64,
    pub last_accessed: DateTime<Utc>,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl_seconds: i64) -> Self {
        let now = Utc::now();
        Self {
            data,
            created_at: now,
            expires_at: now + Duration::seconds(ttl_seconds),
            hit_count: 0,
            last_accessed: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_stale(&self, stale_seconds: i64) -> bool {
        Utc::now() > self.created_at + Duration::seconds(stale_seconds)
    }

    pub fn record_access(&mut self) {
        self.hit_count += 1;
        self.last_accessed = Utc::now();
    }
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub price_data_ttl: i64,       // TTL for price data in seconds
    pub fundamental_data_ttl: i64, // TTL for fundamental data in seconds
    pub news_data_ttl: i64,        // TTL for news data in seconds
    pub stock_name_ttl: i64,       // TTL for stock names in seconds
    pub max_entries: usize,        // Maximum entries per cache type
    pub cleanup_interval: i64,     // Cleanup interval in seconds
    pub enable_stats: bool,        // Enable cache statistics
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            price_data_ttl: 300,        // 5 minutes for price data
            fundamental_data_ttl: 3600, // 1 hour for fundamental data
            news_data_ttl: 1800,        // 30 minutes for news data
            stock_name_ttl: 86400,      // 24 hours for stock names
            max_entries: 1000,          // Max 1000 entries per cache type
            cleanup_interval: 60,       // Cleanup every minute
            enable_stats: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub price_hits: u64,
    pub price_misses: u64,
    pub fundamental_hits: u64,
    pub fundamental_misses: u64,
    pub news_hits: u64,
    pub news_misses: u64,
    pub name_hits: u64,
    pub name_misses: u64,
    pub evictions: u64,
    pub total_entries: usize,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            price_hits: 0,
            price_misses: 0,
            fundamental_hits: 0,
            fundamental_misses: 0,
            news_hits: 0,
            news_misses: 0,
            name_hits: 0,
            name_misses: 0,
            evictions: 0,
            total_entries: 0,
        }
    }
}

pub struct DataCache {
    config: CacheConfig,
    price_cache: Arc<RwLock<HashMap<String, CacheEntry<Vec<PriceData>>>>>,
    fundamental_cache: Arc<RwLock<HashMap<String, CacheEntry<FundamentalData>>>>,
    news_cache: Arc<RwLock<HashMap<String, CacheEntry<(Vec<News>, SentimentAnalysis)>>>>,
    name_cache: Arc<RwLock<HashMap<String, CacheEntry<String>>>>,
    stats: Arc<RwLock<CacheStats>>,
    cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

impl DataCache {
    pub fn new(config: CacheConfig) -> Self {
        let cache = Self {
            config: config.clone(),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            fundamental_cache: Arc::new(RwLock::new(HashMap::new())),
            news_cache: Arc::new(RwLock::new(HashMap::new())),
            name_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            cleanup_task: None,
        };

        // Start cleanup task if enabled
        if config.cleanup_interval > 0 {
            cache.start_cleanup_task();
        }

        cache
    }

    fn start_cleanup_task(&self) {
        let price_cache = self.price_cache.clone();
        let fundamental_cache = self.fundamental_cache.clone();
        let news_cache = self.news_cache.clone();
        let name_cache = self.name_cache.clone();
        let stats = self.stats.clone();
        let interval = self.config.cleanup_interval;

        let _cleanup_task = tokio::spawn(async move {
            let mut interval_timer =
                tokio::time::interval(tokio::time::Duration::from_secs(interval as u64));

            loop {
                interval_timer.tick().await;

                let mut evictions = 0;

                // Clean price cache
                {
                    let mut cache = price_cache.write().await;
                    let before = cache.len();
                    cache.retain(|_, entry| !entry.is_expired());
                    evictions += before - cache.len();
                }

                // Clean fundamental cache
                {
                    let mut cache = fundamental_cache.write().await;
                    let before = cache.len();
                    cache.retain(|_, entry| !entry.is_expired());
                    evictions += before - cache.len();
                }

                // Clean news cache
                {
                    let mut cache = news_cache.write().await;
                    let before = cache.len();
                    cache.retain(|_, entry| !entry.is_expired());
                    evictions += before - cache.len();
                }

                // Clean name cache
                {
                    let mut cache = name_cache.write().await;
                    let before = cache.len();
                    cache.retain(|_, entry| !entry.is_expired());
                    evictions += before - cache.len();
                }

                // Update stats
                if evictions > 0 {
                    let mut stats_guard = stats.write().await;
                    stats_guard.evictions += evictions as u64;
                    stats_guard.total_entries = price_cache.read().await.len()
                        + fundamental_cache.read().await.len()
                        + news_cache.read().await.len()
                        + name_cache.read().await.len();
                }

                log::debug!("Cache cleanup completed, evicted {} entries", evictions);
            }
        });

        // Note: In a real implementation, you'd store the cleanup task handle
        // For now, we'll let it run in the background
    }

    pub async fn get_price_data(&self, stock_code: &str, days: i32) -> Option<Vec<PriceData>> {
        let key = self.generate_price_key(stock_code, days);
        let mut cache = self.price_cache.write().await;

        if let Some(entry) = cache.get_mut(&key) {
            if !entry.is_expired() {
                entry.record_access();
                if self.config.enable_stats {
                    let mut stats = self.stats.write().await;
                    stats.price_hits += 1;
                }
                return Some(entry.data.clone());
            } else {
                // Remove expired entry
                cache.remove(&key);
                if self.config.enable_stats {
                    let mut stats = self.stats.write().await;
                    stats.evictions += 1;
                    stats.total_entries = cache.len();
                }
            }
        }

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.price_misses += 1;
        }
        None
    }

    pub async fn set_price_data(&self, stock_code: &str, days: i32, data: Vec<PriceData>) {
        let key = self.generate_price_key(stock_code, days);
        let mut cache = self.price_cache.write().await;

        // Enforce max entries limit
        if cache.len() >= self.config.max_entries {
            self.evict_lru_price_cache(&mut cache).await;
        }

        cache.insert(key, CacheEntry::new(data, self.config.price_data_ttl));

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.total_entries = cache.len()
                + self.fundamental_cache.read().await.len()
                + self.news_cache.read().await.len()
                + self.name_cache.read().await.len();
        }
    }

    pub async fn get_fundamental_data(&self, stock_code: &str) -> Option<FundamentalData> {
        let key = self.generate_fundamental_key(stock_code);
        let mut cache = self.fundamental_cache.write().await;

        if let Some(entry) = cache.get_mut(&key) {
            if !entry.is_expired() {
                entry.record_access();
                if self.config.enable_stats {
                    let mut stats = self.stats.write().await;
                    stats.fundamental_hits += 1;
                }
                return Some(entry.data.clone());
            } else {
                cache.remove(&key);
                if self.config.enable_stats {
                    let mut stats = self.stats.write().await;
                    stats.evictions += 1;
                    stats.total_entries = self.price_cache.read().await.len()
                        + cache.len()
                        + self.news_cache.read().await.len()
                        + self.name_cache.read().await.len();
                }
            }
        }

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.fundamental_misses += 1;
        }
        None
    }

    pub async fn set_fundamental_data(&self, stock_code: &str, data: FundamentalData) {
        let key = self.generate_fundamental_key(stock_code);
        let mut cache = self.fundamental_cache.write().await;

        if cache.len() >= self.config.max_entries {
            self.evict_lru_fundamental_cache(&mut cache).await;
        }

        cache.insert(key, CacheEntry::new(data, self.config.fundamental_data_ttl));

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.total_entries = self.price_cache.read().await.len()
                + cache.len()
                + self.news_cache.read().await.len()
                + self.name_cache.read().await.len();
        }
    }

    pub async fn get_news_data(
        &self,
        stock_code: &str,
        days: i32,
    ) -> Option<(Vec<News>, SentimentAnalysis)> {
        let key = self.generate_news_key(stock_code, days);
        let mut cache = self.news_cache.write().await;

        if let Some(entry) = cache.get_mut(&key) {
            if !entry.is_expired() {
                entry.record_access();
                if self.config.enable_stats {
                    let mut stats = self.stats.write().await;
                    stats.news_hits += 1;
                }
                return Some(entry.data.clone());
            } else {
                cache.remove(&key);
                if self.config.enable_stats {
                    let mut stats = self.stats.write().await;
                    stats.evictions += 1;
                    stats.total_entries = self.price_cache.read().await.len()
                        + self.fundamental_cache.read().await.len()
                        + cache.len()
                        + self.name_cache.read().await.len();
                }
            }
        }

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.news_misses += 1;
        }
        None
    }

    pub async fn set_news_data(
        &self,
        stock_code: &str,
        days: i32,
        data: (Vec<News>, SentimentAnalysis),
    ) {
        let key = self.generate_news_key(stock_code, days);
        let mut cache = self.news_cache.write().await;

        if cache.len() >= self.config.max_entries {
            self.evict_lru_news_cache(&mut cache).await;
        }

        cache.insert(key, CacheEntry::new(data, self.config.news_data_ttl));

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.total_entries = self.price_cache.read().await.len()
                + self.fundamental_cache.read().await.len()
                + cache.len()
                + self.name_cache.read().await.len();
        }
    }

    pub async fn get_stock_name(&self, stock_code: &str) -> Option<String> {
        let key = self.generate_name_key(stock_code);
        let mut cache = self.name_cache.write().await;

        if let Some(entry) = cache.get_mut(&key) {
            if !entry.is_expired() {
                entry.record_access();
                if self.config.enable_stats {
                    let mut stats = self.stats.write().await;
                    stats.name_hits += 1;
                }
                return Some(entry.data.clone());
            } else {
                cache.remove(&key);
                if self.config.enable_stats {
                    let mut stats = self.stats.write().await;
                    stats.evictions += 1;
                    stats.total_entries = self.price_cache.read().await.len()
                        + self.fundamental_cache.read().await.len()
                        + self.news_cache.read().await.len()
                        + cache.len();
                }
            }
        }

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.name_misses += 1;
        }
        None
    }

    pub async fn set_stock_name(&self, stock_code: &str, name: String) {
        let key = self.generate_name_key(stock_code);
        let mut cache = self.name_cache.write().await;

        if cache.len() >= self.config.max_entries {
            self.evict_lru_name_cache(&mut cache).await;
        }

        cache.insert(key, CacheEntry::new(name, self.config.stock_name_ttl));

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.total_entries = self.price_cache.read().await.len()
                + self.fundamental_cache.read().await.len()
                + self.news_cache.read().await.len()
                + cache.len();
        }
    }

    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    pub async fn clear(&self) {
        self.price_cache.write().await.clear();
        self.fundamental_cache.write().await.clear();
        self.news_cache.write().await.clear();
        self.name_cache.write().await.clear();

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.evictions += stats.total_entries as u64;
            stats.total_entries = 0;
        }
    }

    async fn evict_lru_price_cache(&self, cache: &mut HashMap<String, CacheEntry<Vec<PriceData>>>) {
        if let Some((lru_key, _)) = cache.iter().min_by_key(|(_, entry)| entry.last_accessed) {
            let lru_key = lru_key.clone();
            cache.remove(&lru_key);
        }
    }

    async fn evict_lru_fundamental_cache(
        &self,
        cache: &mut HashMap<String, CacheEntry<FundamentalData>>,
    ) {
        if let Some((lru_key, _)) = cache.iter().min_by_key(|(_, entry)| entry.last_accessed) {
            let lru_key = lru_key.clone();
            cache.remove(&lru_key);
        }
    }

    async fn evict_lru_news_cache(
        &self,
        cache: &mut HashMap<String, CacheEntry<(Vec<News>, SentimentAnalysis)>>,
    ) {
        if let Some((lru_key, _)) = cache.iter().min_by_key(|(_, entry)| entry.last_accessed) {
            let lru_key = lru_key.clone();
            cache.remove(&lru_key);
        }
    }

    async fn evict_lru_name_cache(&self, cache: &mut HashMap<String, CacheEntry<String>>) {
        if let Some((lru_key, _)) = cache.iter().min_by_key(|(_, entry)| entry.last_accessed) {
            let lru_key = lru_key.clone();
            cache.remove(&lru_key);
        }
    }

    fn generate_price_key(&self, stock_code: &str, days: i32) -> String {
        format!("price_{}_{}", stock_code, days)
    }

    fn generate_fundamental_key(&self, stock_code: &str) -> String {
        format!("fundamental_{}", stock_code)
    }

    fn generate_news_key(&self, stock_code: &str, days: i32) -> String {
        format!("news_{}_{}", stock_code, days)
    }

    fn generate_name_key(&self, stock_code: &str) -> String {
        format!("name_{}", stock_code)
    }
}

impl Drop for DataCache {
    fn drop(&mut self) {
        // Cleanup task will be automatically cancelled when dropped
        log::info!("Data cache dropped");
    }
}

#[async_trait::async_trait]
pub trait CachedDataFetcher: DataFetcher + Send + Sync {}

pub struct CachedDataFetcherWrapper<T: CachedDataFetcher> {
    inner: Arc<T>,
    cache: Arc<DataCache>,
}

impl<T: CachedDataFetcher> Clone for CachedDataFetcherWrapper<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl<T: CachedDataFetcher> CachedDataFetcherWrapper<T> {
    pub fn new(inner: T, cache: Arc<DataCache>) -> Self {
        Self {
            inner: Arc::new(inner),
            cache,
        }
    }
}

#[async_trait::async_trait]
impl<T: CachedDataFetcher + 'static> DataFetcher for CachedDataFetcherWrapper<T> {
    async fn get_stock_data(&self, stock_code: &str, days: i32) -> Result<Vec<PriceData>, String> {
        // Try cache first
        if let Some(cached_data) = self.cache.get_price_data(stock_code, days).await {
            log::debug!("Cache hit for price data: {}", stock_code);
            return Ok(cached_data);
        }

        log::debug!(
            "Cache miss for price data: {}, fetching from source",
            stock_code
        );

        // Fetch from source
        let data = self.inner.get_stock_data(stock_code, days).await?;

        // Cache the result
        self.cache
            .set_price_data(stock_code, days, data.clone())
            .await;

        Ok(data)
    }

    async fn get_fundamental_data(&self, stock_code: &str) -> Result<FundamentalData, String> {
        // Try cache first
        if let Some(cached_data) = self.cache.get_fundamental_data(stock_code).await {
            log::debug!("Cache hit for fundamental data: {}", stock_code);
            return Ok(cached_data);
        }

        log::debug!(
            "Cache miss for fundamental data: {}, fetching from source",
            stock_code
        );

        // Fetch from source
        let data = self.inner.get_fundamental_data(stock_code).await?;

        // Cache the result
        self.cache
            .set_fundamental_data(stock_code, data.clone())
            .await;

        Ok(data)
    }

    async fn get_news_data(
        &self,
        stock_code: &str,
        days: i32,
    ) -> Result<(Vec<News>, SentimentAnalysis), String> {
        // Try cache first
        if let Some(cached_data) = self.cache.get_news_data(stock_code, days).await {
            log::debug!("Cache hit for news data: {}", stock_code);
            return Ok(cached_data);
        }

        log::debug!(
            "Cache miss for news data: {}, fetching from source",
            stock_code
        );

        // Fetch from source
        let data = self.inner.get_news_data(stock_code, days).await?;

        // Cache the result
        self.cache
            .set_news_data(stock_code, days, data.clone())
            .await;

        Ok(data)
    }

    async fn get_stock_name(&self, stock_code: &str) -> String {
        // Try cache first
        if let Some(cached_name) = self.cache.get_stock_name(stock_code).await {
            log::debug!("Cache hit for stock name: {}", stock_code);
            return cached_name;
        }

        log::debug!(
            "Cache miss for stock name: {}, fetching from source",
            stock_code
        );

        // Fetch from source
        let name = self.inner.get_stock_name(stock_code).await;

        // Cache the result
        self.cache.set_stock_name(stock_code, name.clone()).await;

        name
    }

    fn clone(&self) -> Box<dyn DataFetcher> {
        Box::new(CachedDataFetcherWrapper {
            inner: self.inner.clone(),
            cache: self.cache.clone(),
        })
    }
}
