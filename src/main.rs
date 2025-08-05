use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use env_logger::Env;
use log::info;

mod ai_service;
mod analyzer;
mod auth;
mod cache;
mod currency;
mod data_fetcher;
mod database;
mod handlers;
mod models;

use crate::handlers::AppState;
use crate::models::AppConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Load configuration
    let config = load_config();

    info!(
        "Starting Rust Stock Analyzer on {}:{}",
        config.server.host, config.server.port
    );
    info!("Akshare proxy URL: {}", config.akshare.proxy_url);
    info!("Max workers: {}", config.analysis.max_workers);
    info!("Database URL: {}", config.database.url);

    let app_state = match AppState::new(config.clone()).await {
        Ok(state) => web::Data::new(state),
        Err(e) => {
            log::error!("Failed to initialize application state: {e}");
            return Err(std::io::Error::other(e));
        }
    };

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|_origin, _req_head| true)
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec!["Authorization", "Accept", "Content-Type"])
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(actix_web::middleware::Logger::default())
            .service(
                web::scope("/api")
                    .route("/analyze", web::post().to(handlers::analyze_single))
                    .route(
                        "/analyze/stream",
                        web::post().to(handlers::analyze_single_streaming),
                    )
                    .route("/batch/analyze", web::post().to(handlers::analyze_batch))
                    .route(
                        "/batch/status/{task_id}",
                        web::get().to(handlers::get_task_status),
                    )
                    .route(
                        "/stock/{stock_code}/price",
                        web::get().to(handlers::get_stock_price),
                    )
                    .route(
                        "/stock/{stock_code}/fundamental",
                        web::get().to(handlers::get_stock_fundamental),
                    )
                    .route(
                        "/stock/{stock_code}/news",
                        web::get().to(handlers::get_stock_news),
                    )
                    .route(
                        "/stock/{stock_code}/name",
                        web::get().to(handlers::get_stock_name),
                    )
                    .route("/health", web::get().to(handlers::health_check))
                    .route("/cache/stats", web::get().to(handlers::get_cache_stats))
                    .route("/cache/clear", web::post().to(handlers::clear_cache))
                    .route(
                        "/currency/convert",
                        web::get().to(handlers::convert_currency),
                    )
                    .route(
                        "/currency/exchange-rate",
                        web::get().to(handlers::get_exchange_rate),
                    )
                    .route(
                        "/currency/supported",
                        web::get().to(handlers::get_supported_currencies),
                    )
                    .route("/market/time", web::get().to(handlers::get_market_time))
                    .service(
                        web::scope("/config")
                            .route("/ai", web::get().to(handlers::get_ai_config))
                            .route("/ai", web::post().to(handlers::update_ai_config))
                            .route("/ai/providers", web::get().to(handlers::get_ai_providers))
                            .route("/ai/test", web::post().to(handlers::test_ai_connection))
                            .route("/auth", web::get().to(handlers::get_auth_config))
                            .route("/auth", web::post().to(handlers::update_auth_config))
                            .route("/system", web::get().to(handlers::get_system_config))
                            .route("/system", web::post().to(handlers::update_system_config))
                            .route(
                                "/datasource/test",
                                web::post().to(handlers::test_datasource),
                            ),
                    )
                    .route("/history", web::get().to(handlers::get_analysis_history))
                    .route("/history/{id}", web::get().to(handlers::get_analysis_by_id))
                    .route("/datasource/test", web::post().to(handlers::test_datasource))
                    .service(
                        web::scope("/configurations")
                            .route("", web::post().to(handlers::save_configuration))
                            .route("", web::get().to(handlers::get_configurations))
                            .route(
                                "/{id}/activate",
                                web::post().to(handlers::activate_configuration),
                            )
                            .route("/{id}", web::delete().to(handlers::delete_configuration)),
                    ),
            )
            .route("/ws", web::get().to(handlers::websocket_handler))
            .route("/", web::get().to(handlers::index))
            .route("/batch", web::get().to(handlers::batch))
            .route("/config", web::get().to(handlers::config))
            .route("/test-config", web::get().to(handlers::test_config))
    })
    .bind((config.server.host.as_str(), config.server.port))?
    .workers(config.server.workers.unwrap_or(4))
    .run()
    .await
}

fn load_config() -> AppConfig {
    use std::fs;

    // Try to load from config file
    if let Ok(config_str) = fs::read_to_string("config.json") {
        if let Ok(config) = serde_json::from_str::<AppConfig>(&config_str) {
            return config;
        }
    }

    // Try to load from environment or use defaults
    AppConfig {
        server: models::ServerConfig {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            workers: std::env::var("WORKERS").ok().and_then(|w| w.parse().ok()),
        },
        analysis: models::AnalysisConfig {
            max_workers: std::env::var("MAX_WORKERS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            timeout_seconds: std::env::var("TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            weights: models::AnalysisWeights {
                technical: std::env::var("TECHNICAL_WEIGHT")
                    .unwrap_or_else(|_| "0.5".to_string())
                    .parse()
                    .unwrap_or(0.5),
                fundamental: std::env::var("FUNDAMENTAL_WEIGHT")
                    .unwrap_or_else(|_| "0.3".to_string())
                    .parse()
                    .unwrap_or(0.3),
                sentiment: std::env::var("SENTIMENT_WEIGHT")
                    .unwrap_or_else(|_| "0.2".to_string())
                    .parse()
                    .unwrap_or(0.2),
            },
            parameters: models::AnalysisParameters {
                technical_period_days: std::env::var("TECHNICAL_PERIOD")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .unwrap_or(60),
                sentiment_period_days: std::env::var("SENTIMENT_PERIOD")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            },
        },
        akshare: models::AkshareConfig {
            proxy_url: std::env::var("AKSERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:5000".to_string()),
            timeout_seconds: std::env::var("AKSERVICE_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        },
        ai: models::AIConfig {
            provider: std::env::var("AI_PROVIDER").unwrap_or_else(|_| "openai".to_string()),
            api_key: std::env::var("AI_API_KEY").unwrap_or_else(|_| "".to_string()),
            base_url: std::env::var("AI_BASE_URL").ok(),
            model: std::env::var("AI_MODEL").ok(),
            enabled: std::env::var("AI_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            timeout_seconds: std::env::var("AI_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        },
        auth: models::AuthConfig {
            enabled: std::env::var("AUTH_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            secret_key: std::env::var("AUTH_SECRET_KEY")
                .unwrap_or_else(|_| "your-secret-key-change-this".to_string()),
            session_timeout: std::env::var("SESSION_TIMEOUT")
                .unwrap_or_else(|_| "86400".to_string())
                .parse()
                .unwrap_or(86400),
            bcrypt_cost: std::env::var("BCRYPT_COST")
                .unwrap_or_else(|_| "12".to_string())
                .parse()
                .unwrap_or(12),
        },
        database: models::DatabaseConfig {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:stock_analyzer.db".to_string()),
            max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            enable_migrations: std::env::var("DATABASE_ENABLE_MIGRATIONS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        },
        cache: models::CacheConfig {
            enabled: std::env::var("CACHE_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            price_data_ttl: std::env::var("CACHE_PRICE_TTL")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            fundamental_data_ttl: std::env::var("CACHE_FUNDAMENTAL_TTL")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .unwrap_or(3600),
            news_data_ttl: std::env::var("CACHE_NEWS_TTL")
                .unwrap_or_else(|_| "1800".to_string())
                .parse()
                .unwrap_or(1800),
            stock_name_ttl: std::env::var("CACHE_NAME_TTL")
                .unwrap_or_else(|_| "86400".to_string())
                .parse()
                .unwrap_or(86400),
            max_entries: std::env::var("CACHE_MAX_ENTRIES")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
            cleanup_interval: std::env::var("CACHE_CLEANUP_INTERVAL")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            enable_stats: std::env::var("CACHE_ENABLE_STATS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        },
    }
}
