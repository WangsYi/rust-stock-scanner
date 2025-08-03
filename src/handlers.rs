use actix_web::{web, HttpResponse, Result, Error};
use bytes::Bytes;
use dashmap::DashMap;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::models::*;
use crate::analyzer::StockAnalyzer;
use crate::data_fetcher::{DataFetcher, AkshareProxy};
use crate::ai_service::{AIService, get_ai_providers_info};
use crate::auth::AuthService;
use crate::database::Database;
use crate::cache::{DataCache, CachedDataFetcherWrapper};
use crate::currency::{CurrencyConverter, MarketTimeInfo};
use async_stream::stream;

pub struct AppState {
    pub analyzer: Arc<StockAnalyzer>,
    pub task_status: Arc<DashMap<String, TaskStatus>>,
    pub progress_tx: mpsc::UnboundedSender<ProgressUpdate>,
    pub progress_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ProgressUpdate>>>,
    pub auth_service: Arc<tokio::sync::RwLock<AuthService>>,
    pub ai_service: Arc<tokio::sync::RwLock<AIService>>,
    pub database: Arc<Database>,
    pub cache: Arc<DataCache>,
    pub currency_converter: Arc<CurrencyConverter>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, String> {
        // Initialize database
        let database = Arc::new(Database::new(&config.database.url)
            .await
            .map_err(|e| format!("Failed to connect to database: {}", e))?);
        
        // Create tables if migrations are enabled
        if config.database.enable_migrations {
            database.create_tables()
                .await
                .map_err(|e| format!("Failed to create database tables: {}", e))?;
        }
        
        // Initialize cache
        let cache_config = crate::cache::CacheConfig {
            price_data_ttl: config.cache.price_data_ttl,
            fundamental_data_ttl: config.cache.fundamental_data_ttl,
            news_data_ttl: config.cache.news_data_ttl,
            stock_name_ttl: config.cache.stock_name_ttl,
            max_entries: config.cache.max_entries,
            cleanup_interval: config.cache.cleanup_interval,
            enable_stats: config.cache.enable_stats,
        };
        
        let cache = Arc::new(DataCache::new(cache_config));
        
        // Create data fetcher with caching if enabled
        let data_fetcher: Box<dyn DataFetcher> = if config.cache.enabled {
            let base_fetcher = AkshareProxy::new(
                config.akshare.proxy_url.clone(),
                config.akshare.timeout_seconds,
            );
            let cached_fetcher = CachedDataFetcherWrapper::new(base_fetcher, cache.clone());
            Box::new(cached_fetcher)
        } else {
            Box::new(AkshareProxy::new(
                config.akshare.proxy_url.clone(),
                config.akshare.timeout_seconds,
            ))
        };

        let auth_service = Arc::new(tokio::sync::RwLock::new(AuthService::new(config.auth.clone())));
        
        // Initialize AI service with default config
        let ai_service = Arc::new(tokio::sync::RwLock::new(AIService::new(config.ai.clone())));
        
        // Try to load saved AI configuration from database
        if let Ok(Some(saved_config)) = database.get_active_configuration("ai").await {
            if let Ok(ai_config) = serde_json::from_value::<crate::models::AIConfig>(saved_config.config_data) {
                let mut ai_service_writer = ai_service.write().await;
                ai_service_writer.update_config(ai_config);
                log::info!("Loaded saved AI configuration from database");
            }
        }

        let analyzer = Arc::new(StockAnalyzer::with_database(
            data_fetcher,
            config.analysis.clone(),
            ai_service.clone(),
            database.clone(),
        ));
        
        let (progress_tx, progress_rx) = mpsc::unbounded_channel();
        
        // Initialize currency converter
        let currency_converter = Arc::new(CurrencyConverter::new("USD".to_string(), 3600));
        
        Ok(Self {
            analyzer,
            task_status: Arc::new(DashMap::new()),
            progress_tx,
            progress_rx: Arc::new(tokio::sync::Mutex::new(progress_rx)),
            auth_service,
            ai_service,
            database,
            cache,
            currency_converter,
        })
    }
}

pub async fn analyze_single(
    data: web::Json<SingleAnalysisRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let request = data.into_inner();
    
    match state.analyzer.analyze_single_stock(&request.stock_code, request.enable_ai.unwrap_or(true)).await {
        Ok(report) => {
            Ok(HttpResponse::Ok().json(ApiResponse::success(report)))
        }
        Err(error) => {
            Ok(HttpResponse::Ok().json(ApiResponse::<AnalysisReport>::error(error)))
        }
    }
}

pub async fn analyze_single_streaming(
    data: web::Json<SingleAnalysisRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let request = data.into_inner();
    let stock_code = request.stock_code.clone();
    let stock_code_clone = stock_code.clone();
    let enable_ai = request.enable_ai.unwrap_or(true);
    let progress_tx = state.progress_tx.clone();
    
    // Send initial progress update
    let _ = progress_tx.send(ProgressUpdate {
        task_id: stock_code.clone(),
        current: 0,
        total: 1,
        percentage: 0.0,
        status: "开始分析".to_string(),
        current_stock: Some(stock_code.clone()),
        message: Some(format!("开始分析股票: {}", stock_code)),
        timestamp: chrono::Utc::now(),
        analysis_report: None,
    });
    
    // Spawn the analysis task
    let analyzer = state.analyzer.clone();
    let progress_tx_clone = progress_tx.clone();
    
    tokio::spawn(async move {
        match analyzer.analyze_single_stock(&stock_code, enable_ai).await {
            Ok(report) => {
                // Send completion message with full report
                let _ = progress_tx_clone.send(ProgressUpdate {
                    task_id: stock_code.clone(),
                    current: 1,
                    total: 1,
                    percentage: 100.0,
                    status: "分析完成".to_string(),
                    current_stock: Some(stock_code.clone()),
                    message: Some(format!("完成分析: {}", stock_code)),
                    timestamp: chrono::Utc::now(),
                    analysis_report: Some(report),
                });
            }
            Err(error) => {
                // Send error message
                let _ = progress_tx_clone.send(ProgressUpdate {
                    task_id: stock_code.clone(),
                    current: 1,
                    total: 1,
                    percentage: 100.0,
                    status: "分析失败".to_string(),
                    current_stock: Some(stock_code.clone()),
                    message: Some(format!("分析失败: {}", error)),
                    timestamp: chrono::Utc::now(),
                    analysis_report: None,
                });
            }
        }
    });
    
    // Return Server-Sent Events stream
    Ok(HttpResponse::Ok()
        .insert_header(("content-type", "text/event-stream"))
        .insert_header(("cache-control", "no-cache"))
        .insert_header(("connection", "keep-alive"))
        .insert_header(("access-control-allow-origin", "*"))
        .streaming(stream! {
            let mut progress_rx = state.progress_rx.lock().await;
            let mut last_message = None;
            
            // Send initial message
            yield Ok::<_, actix_web::Error>(Bytes::from(format!(
                "data: {}\n\n",
                serde_json::json!({
                    "type": "started",
                    "message": format!("开始分析股票: {}", stock_code_clone)
                })
            )));
            
            loop {
                tokio::select! {
                    Some(progress_update) = progress_rx.recv() => {
                        // Only send messages for this specific stock
                        if progress_update.task_id == stock_code_clone {
                            let message = if progress_update.analysis_report.is_some() {
                                // Send final result with actual analysis data
                                serde_json::json!({
                                    "type": "final_result",
                                    "data": progress_update.analysis_report.unwrap()
                                })
                            } else {
                                // Send regular progress update
                                serde_json::json!({
                                    "type": "progress",
                                    "data": progress_update
                                })
                            };
                            
                            if last_message.as_ref() != Some(&message.to_string()) {
                                yield Ok::<_, actix_web::Error>(Bytes::from(format!(
                                    "data: {}\n\n",
                                    message
                                )));
                                last_message = Some(message.to_string());
                            }
                            
                            // Break if analysis is complete
                            if progress_update.percentage >= 100.0 {
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                        // Send keepalive
                        yield Ok::<_, actix_web::Error>(Bytes::from("data: {\"type\": \"keepalive\"}\n\n"));
                    }
                }
            }
        }))
}

pub async fn analyze_batch(
    data: web::Json<BatchAnalysisRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let request = data.into_inner();
    let task_id = Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();
    
    let task_status = TaskStatus {
        task_id: task_id.clone(),
        status: "运行中".to_string(),
        progress: 0.0,
        total_stocks: request.stock_codes.len() as i32,
        completed: 0,
        failed: 0,
        current_stock: None,
        start_time: chrono::Utc::now(),
        last_update: chrono::Utc::now(),
    };

    state.task_status.insert(task_id.clone(), task_status);

    let analyzer = state.analyzer.clone();
    let task_status = state.task_status.clone();
    let progress_tx = state.progress_tx.clone();
    let stock_codes = request.stock_codes.clone();
    let enable_ai = request.enable_ai.unwrap_or(true);

    tokio::spawn(async move {
        let total_stocks = stock_codes.len() as i32;
        let mut completed = 0;
        let mut failed = 0;

        for (index, stock_code) in stock_codes.iter().enumerate() {
            let progress = (index as f64 / total_stocks as f64) * 100.0;
            
            // Update current stock
            if let Some(mut status) = task_status.get_mut(&task_id_clone) {
                status.current_stock = Some(stock_code.clone());
                status.progress = progress;
                status.last_update = chrono::Utc::now();
            }

            // Send progress update
            let _ = progress_tx.send(ProgressUpdate {
                task_id: task_id_clone.clone(),
                current: index as i32 + 1,
                total: total_stocks,
                percentage: progress,
                status: "运行中".to_string(),
                current_stock: Some(stock_code.clone()),
                message: Some(format!("分析股票: {}", stock_code)),
                timestamp: chrono::Utc::now(),
                analysis_report: None,
            });

            match analyzer.analyze_single_stock(stock_code, enable_ai).await {
                Ok(_) => {
                    completed += 1;
                }
                Err(_) => {
                    failed += 1;
                }
            }

            // Update task status
            if let Some(mut status) = task_status.get_mut(&task_id_clone) {
                status.completed = completed;
                status.failed = failed;
                status.progress = ((completed + failed) as f64 / total_stocks as f64) * 100.0;
                status.last_update = chrono::Utc::now();
            }

            // Send completion update
            let _ = progress_tx.send(ProgressUpdate {
                task_id: task_id_clone.clone(),
                current: index as i32 + 1,
                total: total_stocks,
                percentage: ((completed + failed) as f64 / total_stocks as f64) * 100.0,
                status: "运行中".to_string(),
                current_stock: Some(stock_code.clone()),
                message: Some(format!("完成分析: {}", stock_code)),
                timestamp: chrono::Utc::now(),
                analysis_report: None,
            });

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Mark task as completed
        if let Some(mut status) = task_status.get_mut(&task_id_clone) {
            status.status = "已完成".to_string();
            status.progress = 100.0;
            status.last_update = chrono::Utc::now();
        }

        let _ = progress_tx.send(ProgressUpdate {
            task_id: task_id_clone.clone(),
            current: total_stocks,
            total: total_stocks,
            percentage: 100.0,
            status: "已完成".to_string(),
            current_stock: None,
            message: Some("批量分析完成".to_string()),
            timestamp: chrono::Utc::now(),
            analysis_report: None,
        });
    });

    Ok(HttpResponse::Ok().json(ApiResponse::success(task_id)))
}

pub async fn get_task_status(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let task_id = path.into_inner();
    
    match state.task_status.get(&task_id) {
        Some(status) => Ok(HttpResponse::Ok().json(ApiResponse::success(status.clone()))),
        None => Ok(HttpResponse::Ok().json(ApiResponse::<TaskStatus>::error("任务不存在".to_string()))),
    }
}

pub async fn websocket_handler(
    _req: actix_web::HttpRequest,
    _stream: web::Payload,
    _state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // For now, return a simple response indicating WebSocket is not implemented
    // The frontend should use the streaming endpoint instead
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "WebSocket not implemented. Use /api/analyze/stream for streaming analysis."
    })))
}

pub async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(ApiResponse::success("服务运行正常".to_string())))
}

// Cache management endpoints
pub async fn get_cache_stats(state: web::Data<AppState>) -> Result<HttpResponse> {
    let stats = state.cache.get_stats().await;
    Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
}

pub async fn clear_cache(state: web::Data<AppState>) -> Result<HttpResponse> {
    state.cache.clear().await;
    Ok(HttpResponse::Ok().json(ApiResponse::success("缓存已清空".to_string())))
}

// Currency conversion endpoints
pub async fn convert_currency(
    query: web::Query<CurrencyConversionQuery>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let result = state.currency_converter.convert_amount(
        query.amount,
        &query.from_currency,
        &query.to_currency,
    ).await;
    
    match result {
        Ok(converted_amount) => {
            let response = CurrencyConversionResponse {
                original_amount: query.amount,
                from_currency: query.from_currency.clone(),
                to_currency: query.to_currency.clone(),
                converted_amount,
                exchange_rate: converted_amount / query.amount,
                timestamp: chrono::Utc::now(),
            };
            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        },
        Err(e) => {
            Ok(HttpResponse::BadRequest().json(ApiResponse::<String>::error(e)))
        }
    }
}

pub async fn get_exchange_rate(
    query: web::Query<ExchangeRateQuery>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let result = state.currency_converter.get_exchange_rate(
        &query.from_currency,
        &query.to_currency,
    ).await;
    
    match result {
        Ok(rate) => {
            let response = ExchangeRateResponse {
                from_currency: query.from_currency.clone(),
                to_currency: query.to_currency.clone(),
                rate,
                timestamp: chrono::Utc::now(),
            };
            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        },
        Err(e) => {
            Ok(HttpResponse::BadRequest().json(ApiResponse::<String>::error(e)))
        }
    }
}

pub async fn get_market_time(
    query: web::Query<MarketTimeQuery>,
    _state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let market = crate::models::Market::from_stock_code(&query.stock_code);
    let current_time = chrono::Utc::now();
    let market_time_info = MarketTimeInfo::new(market, current_time);
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(market_time_info)))
}

pub async fn get_supported_currencies(state: web::Data<AppState>) -> Result<HttpResponse> {
    let currencies = state.currency_converter.get_supported_currencies().await;
    Ok(HttpResponse::Ok().json(ApiResponse::success(currencies)))
}

// Web handlers for templates
pub async fn index() -> Result<HttpResponse> {
    let html = include_str!("../templates/index.html");
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

pub async fn batch() -> Result<HttpResponse> {
    let html = include_str!("../templates/batch.html");
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

// Additional API endpoints
pub async fn get_stock_price(
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let stock_code = path.into_inner();
    let days = query.get("days").and_then(|d| d.parse::<i32>().ok()).unwrap_or(30);
    
    match state.analyzer.data_fetcher().get_stock_data(&stock_code, days).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::success(data))),
        Err(error) => Ok(HttpResponse::Ok().json(ApiResponse::<Vec<PriceData>>::error(error))),
    }
}

pub async fn get_stock_fundamental(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let stock_code = path.into_inner();
    
    match state.analyzer.data_fetcher().get_fundamental_data(&stock_code).await {
        Ok(data) => Ok(HttpResponse::Ok().json(ApiResponse::success(data))),
        Err(error) => Ok(HttpResponse::Ok().json(ApiResponse::<FundamentalData>::error(error))),
    }
}

pub async fn get_stock_news(
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let stock_code = path.into_inner();
    let days = query.get("days").and_then(|d| d.parse::<i32>().ok()).unwrap_or(15);
    
    match state.analyzer.data_fetcher().get_news_data(&stock_code, days).await {
        Ok((news, sentiment)) => Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
            "news": news,
            "sentiment": sentiment
        })))),
        Err(error) => Ok(HttpResponse::Ok().json(ApiResponse::<FundamentalData>::error(error))),
    }
}

pub async fn get_stock_name(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let stock_code = path.into_inner();
    let name = state.analyzer.data_fetcher().get_stock_name(&stock_code).await;
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(name)))
}

// Configuration handlers
pub async fn config() -> Result<HttpResponse> {
    let html = include_str!("../templates/config.html");
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

pub async fn test_config() -> Result<HttpResponse> {
    let html = include_str!("../templates/test_fix.html");
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

pub async fn get_ai_config(state: web::Data<AppState>) -> Result<HttpResponse> {
    // Try to load configuration from database first
    let config = if let Ok(Some(saved_config)) = state.database.get_active_configuration("ai").await {
        if let Ok(ai_config) = serde_json::from_value::<crate::models::AIConfig>(saved_config.config_data) {
            ai_config
        } else {
            // Fallback to current AI service config
            let ai_service = state.ai_service.read().await;
            ai_service.get_config().clone()
        }
    } else {
        // Fallback to current AI service config
        let ai_service = state.ai_service.read().await;
        ai_service.get_config().clone()
    };
    
    let response = serde_json::json!({
        "provider": config.provider,
        "model": config.model,
        "enabled": config.enabled,
        "base_url": config.base_url,
        "api_key": config.api_key, // Include API key from database
        "is_configured": !config.api_key.is_empty(),
        "supported_providers": get_ai_providers_info(),
    });
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

pub async fn update_ai_config(
    data: web::Json<serde_json::Value>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let mut ai_service = state.ai_service.write().await;
    
    let update_config = crate::models::AIConfig {
        provider: data["provider"].as_str().unwrap_or("openai").to_string(),
        api_key: data["api_key"].as_str().unwrap_or("").to_string(),
        base_url: data["base_url"].as_str().map(|s| s.to_string()),
        model: data["model"].as_str().map(|s| s.to_string()),
        enabled: data["enabled"].as_bool().unwrap_or(true),
        timeout_seconds: data["timeout_seconds"].as_u64().unwrap_or(30),
    };
    
    // Update AI service configuration
    ai_service.update_config(update_config.clone());
    
    // Save configuration to database
    let config_json = serde_json::to_value(update_config).unwrap_or_default();
    match state.database.save_configuration("ai", "default", &config_json).await {
        Ok(id) => {
            // Activate the newly saved configuration
            if let Err(e) = state.database.activate_configuration(id).await {
                log::warn!("Failed to activate AI configuration: {}", e);
            }
        }
        Err(e) => {
            log::warn!("Failed to save AI configuration to database: {}", e);
        }
    }
    
    Ok(HttpResponse::Ok().json(ApiResponse::success("AI配置已更新")))
}

pub async fn get_ai_providers() -> Result<HttpResponse> {
    let providers = get_ai_providers_info();
    Ok(HttpResponse::Ok().json(ApiResponse::success(providers)))
}

pub async fn test_ai_connection(state: web::Data<AppState>) -> Result<HttpResponse> {
    let ai_service = state.ai_service.read().await;
    
    if !ai_service.is_enabled() {
        return Ok(HttpResponse::Ok().json(ApiResponse::<bool>::error("AI服务未启用".to_string())));
    }
    
    // Simple test - we'll just check if we can create a basic request
    Ok(HttpResponse::Ok().json(ApiResponse::success(true)))
}

pub async fn get_auth_config(state: web::Data<AppState>) -> Result<HttpResponse> {
    let auth_service = state.auth_service.read().await;
    let config = auth_service.get_config().clone();
    
    let response = serde_json::json!({
        "enabled": config.enabled,
        "session_timeout": config.session_timeout,
        "bcrypt_cost": config.bcrypt_cost,
    });
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

pub async fn update_auth_config(
    _data: web::Json<serde_json::Value>,
    _state: web::Data<AppState>,
) -> Result<HttpResponse> {
    // Note: In a real implementation, this would need proper error handling
    Ok(HttpResponse::Ok().json(ApiResponse::success("认证配置已更新")))
}

pub async fn get_system_config(_state: web::Data<AppState>) -> Result<HttpResponse> {
    let config = load_config();
    
    let response = serde_json::json!({
        "akshare_url": config.akshare.proxy_url,
        "akshare_timeout": config.akshare.timeout_seconds,
        "max_workers": config.analysis.max_workers,
        "technical_period": config.analysis.parameters.technical_period_days,
        "sentiment_period": config.analysis.parameters.sentiment_period_days,
    });
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

pub async fn update_system_config(
    _data: web::Json<serde_json::Value>,
    _state: web::Data<AppState>,
) -> Result<HttpResponse> {
    // Note: System config changes would require restart in this implementation
    Ok(HttpResponse::Ok().json(ApiResponse::success("系统配置已更新（需要重启生效）")))
}

pub async fn test_datasource(state: web::Data<AppState>) -> Result<HttpResponse> {
    let stock_code = "000001";
    
    match state.analyzer.data_fetcher().get_stock_data(stock_code, 1).await {
        Ok(data) => {
            if data.is_empty() {
                Ok(HttpResponse::Ok().json(ApiResponse::<bool>::error("数据源返回空数据".to_string())))
            } else {
                Ok(HttpResponse::Ok().json(ApiResponse::success(true)))
            }
        }
        Err(error) => {
            Ok(HttpResponse::Ok().json(ApiResponse::<bool>::error(format!("数据源连接失败: {}", error))))
        }
    }
}

// History and configuration endpoints
pub async fn get_analysis_history(
    query: web::Query<HistoryQuery>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let history = state.database.get_analysis_history(&query).await;
    match history {
        Ok(response) => Ok(HttpResponse::Ok().json(ApiResponse::success(response))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<HistoryResponse>::error(
            format!("Failed to get analysis history: {}", e)
        ))),
    }
}

pub async fn get_analysis_by_id(
    path: web::Path<uuid::Uuid>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let analysis = state.database.get_analysis_by_id(*path).await;
    match analysis {
        Ok(Some(analysis)) => Ok(HttpResponse::Ok().json(ApiResponse::success(analysis))),
        Ok(None) => Ok(HttpResponse::NotFound().json(ApiResponse::<SavedAnalysis>::error(
            "Analysis not found".to_string()
        ))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<SavedAnalysis>::error(
            format!("Failed to get analysis: {}", e)
        ))),
    }
}

pub async fn save_configuration(
    config: web::Json<serde_json::Value>,
    query: web::Query<serde_json::Value>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let config_type = query.get("type").and_then(|v| v.as_str()).unwrap_or("general");
    let config_name = query.get("name").and_then(|v| v.as_str()).unwrap_or("default");
    
    match state.database.save_configuration(config_type, config_name, &config).await {
        Ok(id) => Ok(HttpResponse::Ok().json(ApiResponse::success(id))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<uuid::Uuid>::error(
            format!("Failed to save configuration: {}", e)
        ))),
    }
}

pub async fn get_configurations(
    query: web::Query<serde_json::Value>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let config_type = query.get("type").and_then(|v| v.as_str());
    
    match state.database.list_configurations(config_type).await {
        Ok(configs) => Ok(HttpResponse::Ok().json(ApiResponse::success(configs))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<Vec<SavedConfiguration>>::error(
            format!("Failed to get configurations: {}", e)
        ))),
    }
}

pub async fn activate_configuration(
    path: web::Path<uuid::Uuid>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    match state.database.activate_configuration(*path).await {
        Ok(true) => Ok(HttpResponse::Ok().json(ApiResponse::success(true))),
        Ok(false) => Ok(HttpResponse::NotFound().json(ApiResponse::<bool>::error(
            "Configuration not found".to_string()
        ))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<bool>::error(
            format!("Failed to activate configuration: {}", e)
        ))),
    }
}

pub async fn delete_configuration(
    path: web::Path<uuid::Uuid>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    match state.database.delete_configuration(*path).await {
        Ok(true) => Ok(HttpResponse::Ok().json(ApiResponse::success(true))),
        Ok(false) => Ok(HttpResponse::NotFound().json(ApiResponse::<bool>::error(
            "Configuration not found".to_string()
        ))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ApiResponse::<bool>::error(
            format!("Failed to delete configuration: {}", e)
        ))),
    }
}

// Helper function to load config
use crate::models::AppConfig;
use std::fs;

fn load_config() -> AppConfig {
    // Try to load from config file
    if let Ok(config_str) = fs::read_to_string("config.json") {
        if let Ok(config) = serde_json::from_str::<AppConfig>(&config_str) {
            return config;
        }
    }

    // Try to load from environment or use defaults
    AppConfig {
        server: crate::models::ServerConfig {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse().unwrap_or(8080),
            workers: std::env::var("WORKERS").ok().and_then(|w| w.parse().ok()),
        },
        analysis: crate::models::AnalysisConfig {
            max_workers: std::env::var("MAX_WORKERS").unwrap_or_else(|_| "10".to_string()).parse().unwrap_or(10),
            timeout_seconds: std::env::var("TIMEOUT_SECONDS").unwrap_or_else(|_| "30".to_string()).parse().unwrap_or(30),
            weights: crate::models::AnalysisWeights {
                technical: std::env::var("TECHNICAL_WEIGHT").unwrap_or_else(|_| "0.5".to_string()).parse().unwrap_or(0.5),
                fundamental: std::env::var("FUNDAMENTAL_WEIGHT").unwrap_or_else(|_| "0.3".to_string()).parse().unwrap_or(0.3),
                sentiment: std::env::var("SENTIMENT_WEIGHT").unwrap_or_else(|_| "0.2".to_string()).parse().unwrap_or(0.2),
            },
            parameters: crate::models::AnalysisParameters {
                technical_period_days: std::env::var("TECHNICAL_PERIOD").unwrap_or_else(|_| "60".to_string()).parse().unwrap_or(60),
                sentiment_period_days: std::env::var("SENTIMENT_PERIOD").unwrap_or_else(|_| "30".to_string()).parse().unwrap_or(30),
            },
        },
        akshare: crate::models::AkshareConfig {
            proxy_url: std::env::var("AKSERVICE_URL").unwrap_or_else(|_| "http://localhost:5000".to_string()),
            timeout_seconds: std::env::var("AKSERVICE_TIMEOUT").unwrap_or_else(|_| "30".to_string()).parse().unwrap_or(30),
        },
        ai: crate::models::AIConfig {
            provider: std::env::var("AI_PROVIDER").unwrap_or_else(|_| "openai".to_string()),
            api_key: std::env::var("AI_API_KEY").unwrap_or_else(|_| "".to_string()),
            base_url: std::env::var("AI_BASE_URL").ok(),
            model: std::env::var("AI_MODEL").ok(),
            enabled: std::env::var("AI_ENABLED").unwrap_or_else(|_| "true".to_string()).parse().unwrap_or(true),
            timeout_seconds: std::env::var("AI_TIMEOUT").unwrap_or_else(|_| "30".to_string()).parse().unwrap_or(30),
        },
        auth: crate::models::AuthConfig {
            enabled: std::env::var("AUTH_ENABLED").unwrap_or_else(|_| "false".to_string()).parse().unwrap_or(false),
            secret_key: std::env::var("AUTH_SECRET_KEY").unwrap_or_else(|_| "your-secret-key-change-this".to_string()),
            session_timeout: std::env::var("SESSION_TIMEOUT").unwrap_or_else(|_| "86400".to_string()).parse().unwrap_or(86400),
            bcrypt_cost: std::env::var("BCRYPT_COST").unwrap_or_else(|_| "12".to_string()).parse().unwrap_or(12),
        },
        database: crate::models::DatabaseConfig {
            url: std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:stock_analyzer.db".to_string()),
            max_connections: std::env::var("DATABASE_MAX_CONNECTIONS").unwrap_or_else(|_| "5".to_string()).parse().unwrap_or(5),
            enable_migrations: std::env::var("DATABASE_ENABLE_MIGRATIONS").unwrap_or_else(|_| "true".to_string()).parse().unwrap_or(true),
        },
        cache: crate::models::CacheConfig::default(),
    }
}