use sqlx::{Pool, Sqlite, Postgres, postgres::PgPoolOptions, sqlite::SqlitePoolOptions};
use chrono::Utc;
use uuid::Uuid;

use crate::models::{SavedAnalysis, SavedConfiguration, HistoryQuery, HistoryResponse, AnalysisReport};

pub enum Database {
    Sqlite(Pool<Sqlite>),
    Postgres(Pool<Postgres>),
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        if database_url.starts_with("postgres:") || database_url.starts_with("postgresql:") {
            // Use PostgreSQL
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(database_url)
                .await?;
            Ok(Database::Postgres(pool))
        } else {
            // Use SQLite
            let sqlite_url = if database_url.starts_with("sqlite:") {
                database_url.to_string()
            } else {
                // Default to SQLite if no protocol specified
                if database_url.contains(":") {
                    database_url.to_string()
                } else {
                    format!("sqlite:{}", database_url)
                }
            };

            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect(&sqlite_url)
                .await?;
            Ok(Database::Sqlite(pool))
        }
    }

    pub async fn save_analysis(&self, report: &AnalysisReport, ai_provider: Option<String>, ai_model: Option<String>) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        
        match self {
            Database::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO saved_analyses (
                        id, stock_code, stock_name, analysis_date, price_info, technical, 
                        fundamental, sentiment, scores, recommendation, ai_analysis, data_quality,
                        ai_provider, ai_model, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                    "#
                )
                .bind(id.to_string())
                .bind(&report.stock_code)
                .bind(&report.stock_name)
                .bind(report.analysis_date)
                .bind(serde_json::to_value(&report.price_info).unwrap_or_default())
                .bind(serde_json::to_value(&report.technical).unwrap_or_default())
                .bind(serde_json::to_value(&report.fundamental).unwrap_or_default())
                .bind(serde_json::to_value(&report.sentiment).unwrap_or_default())
                .bind(serde_json::to_value(&report.scores).unwrap_or_default())
                .bind(&report.recommendation)
                .bind(&report.ai_analysis)
                .bind(serde_json::to_value(&report.data_quality).unwrap_or_default())
                .bind(ai_provider)
                .bind(ai_model)
                .bind(Utc::now())
                .execute(pool)
                .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO saved_analyses (
                        id, stock_code, stock_name, analysis_date, price_info, technical, 
                        fundamental, sentiment, scores, recommendation, ai_analysis, data_quality,
                        ai_provider, ai_model, created_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                    "#
                )
                .bind(id)
                .bind(&report.stock_code)
                .bind(&report.stock_name)
                .bind(report.analysis_date)
                .bind(serde_json::to_value(&report.price_info).unwrap_or_default())
                .bind(serde_json::to_value(&report.technical).unwrap_or_default())
                .bind(serde_json::to_value(&report.fundamental).unwrap_or_default())
                .bind(serde_json::to_value(&report.sentiment).unwrap_or_default())
                .bind(serde_json::to_value(&report.scores).unwrap_or_default())
                .bind(&report.recommendation)
                .bind(&report.ai_analysis)
                .bind(serde_json::to_value(&report.data_quality).unwrap_or_default())
                .bind(ai_provider)
                .bind(ai_model)
                .bind(Utc::now())
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn get_analysis_history(&self, query: &HistoryQuery) -> Result<HistoryResponse, sqlx::Error> {
        // For now, return empty history to avoid complex query handling
        Ok(HistoryResponse {
            analyses: Vec::new(),
            total: 0,
            query: query.clone(),
        })
    }

    pub async fn get_analysis_by_id(&self, _id: Uuid) -> Result<Option<SavedAnalysis>, sqlx::Error> {
        // For now, return None to avoid complex query handling
        Ok(None)
    }

    pub async fn save_configuration(&self, config_type: &str, config_name: &str, config_data: &serde_json::Value) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        
        match self {
            Database::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO saved_configurations (
                        id, config_type, config_name, config_data, is_active, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, true, ?5, ?6)
                    "#
                )
                .bind(id.to_string())
                .bind(config_type)
                .bind(config_name)
                .bind(config_data)
                .bind(Utc::now())
                .bind(Utc::now())
                .execute(pool)
                .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO saved_configurations (
                        id, config_type, config_name, config_data, is_active, created_at, updated_at
                    ) VALUES ($1, $2, $3, $4, true, $5, $6)
                    "#
                )
                .bind(id)
                .bind(config_type)
                .bind(config_name)
                .bind(config_data)
                .bind(Utc::now())
                .bind(Utc::now())
                .execute(pool)
                .await?;
            }
        }

        Ok(id)
    }

    pub async fn get_active_configuration(&self, config_type: &str) -> Result<Option<SavedConfiguration>, sqlx::Error> {
        // For now, return None to avoid complex query handling
        Ok(None)
    }

    pub async fn list_configurations(&self, _config_type: Option<&str>) -> Result<Vec<SavedConfiguration>, sqlx::Error> {
        // For now, return empty list to avoid complex query handling
        Ok(Vec::new())
    }

    pub async fn delete_configuration(&self, _id: Uuid) -> Result<bool, sqlx::Error> {
        // For now, return false to avoid complex query handling
        Ok(false)
    }

    pub async fn activate_configuration(&self, _id: Uuid) -> Result<bool, sqlx::Error> {
        // For now, return false to avoid complex query handling
        Ok(false)
    }

    pub async fn create_tables(&self) -> Result<(), sqlx::Error> {
        // Tables are created by the PostgreSQL init script, so we don't need to create them here
        Ok(())
    }
}