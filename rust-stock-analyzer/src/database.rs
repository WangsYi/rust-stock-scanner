use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use chrono::Utc;
use uuid::Uuid;

use crate::models::{SavedAnalysis, SavedConfiguration, HistoryQuery, HistoryResponse, AnalysisReport};

pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        // Handle different URL formats
        let sqlite_url = if database_url.starts_with("sqlite:") {
            database_url.to_string()
        } else if database_url.starts_with("postgres:") || database_url.starts_with("postgresql:") {
            // For now, default to SQLite even if PostgreSQL URL is provided
            // This maintains compatibility while we focus on SQLite
            "sqlite:stock_analyzer.db".to_string()
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

        Ok(Database { pool })
    }

    pub async fn save_analysis(&self, report: &AnalysisReport, ai_provider: Option<String>, ai_model: Option<String>) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        
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
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_analysis_history(&self, query: &HistoryQuery) -> Result<HistoryResponse, sqlx::Error> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);
        
        let mut sql = String::from(
            "SELECT id, stock_code, stock_name, analysis_date, price_info, technical, 
             fundamental, sentiment, scores, recommendation, ai_analysis, data_quality,
             ai_provider, ai_model, created_at 
             FROM saved_analyses WHERE 1=1"
        );
        
        let mut binds = vec![];
        
        if let Some(ref stock_code) = query.stock_code {
            sql.push_str(" AND stock_code = ?");
            binds.push(stock_code.clone());
        }
        
        if let Some(ref start_date) = query.start_date {
            sql.push_str(" AND analysis_date >= ?");
            binds.push(start_date.to_rfc3339());
        }
        
        if let Some(ref end_date) = query.end_date {
            sql.push_str(" AND analysis_date <= ?");
            binds.push(end_date.to_rfc3339());
        }
        
        sql.push_str(" ORDER BY analysis_date DESC LIMIT ? OFFSET ?");
        binds.push(limit.to_string());
        binds.push(offset.to_string());
        
        // Build query dynamically
        let mut query_builder = sqlx::query_as::<_, SavedAnalysis>(&sql);
        for bind in binds {
            query_builder = query_builder.bind(bind);
        }
        
        let analyses = query_builder.fetch_all(&self.pool).await?;
        
        // For count, we'll use a simpler approach - count all results
        let total = analyses.len() as i64;
        
        Ok(HistoryResponse {
            analyses,
            total,
            query: query.clone(),
        })
    }

    pub async fn get_analysis_by_id(&self, id: Uuid) -> Result<Option<SavedAnalysis>, sqlx::Error> {
        let id_str = id.to_string();
        Ok(sqlx::query_as::<_, SavedAnalysis>(
            "SELECT id, stock_code, stock_name, analysis_date, price_info, technical, 
             fundamental, sentiment, scores, recommendation, ai_analysis, data_quality,
             ai_provider, ai_model, created_at 
             FROM saved_analyses WHERE id = ?1"
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn save_configuration(&self, config_type: &str, config_name: &str, config_data: &serde_json::Value) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let now = Utc::now();
        
        sqlx::query(
            r#"
            INSERT INTO saved_configurations (
                id, config_type, config_name, config_data, is_active, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, true, ?5, ?6)
            "#
        )
        .bind(&id_str)
        .bind(config_type)
        .bind(config_name)
        .bind(config_data)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn get_active_configuration(&self, config_type: &str) -> Result<Option<SavedConfiguration>, sqlx::Error> {
        Ok(sqlx::query_as::<_, SavedConfiguration>(
            "SELECT id, config_type, config_name, config_data, is_active, created_at, updated_at 
             FROM saved_configurations 
             WHERE config_type = ?1 AND is_active = true 
             ORDER BY updated_at DESC 
             LIMIT 1"
        )
        .bind(config_type)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn list_configurations(&self, config_type: Option<&str>) -> Result<Vec<SavedConfiguration>, sqlx::Error> {
        if let Some(config_type) = config_type {
            Ok(sqlx::query_as::<_, SavedConfiguration>(
                "SELECT id, config_type, config_name, config_data, is_active, created_at, updated_at 
                 FROM saved_configurations 
                 WHERE config_type = ?1 
                 ORDER BY updated_at DESC"
            )
            .bind(config_type)
            .fetch_all(&self.pool)
            .await?)
        } else {
            Ok(sqlx::query_as::<_, SavedConfiguration>(
                "SELECT id, config_type, config_name, config_data, is_active, created_at, updated_at 
                 FROM saved_configurations 
                 ORDER BY updated_at DESC"
            )
            .fetch_all(&self.pool)
            .await?)
        }
    }

    pub async fn delete_configuration(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let id_str = id.to_string();
        let result = sqlx::query("DELETE FROM saved_configurations WHERE id = ?1")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn activate_configuration(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let id_str = id.to_string();
        let mut tx = self.pool.begin().await?;
        
        // Deactivate all configurations of the same type
        let config_type: Option<String> = sqlx::query_scalar("SELECT config_type FROM saved_configurations WHERE id = ?1")
            .bind(&id_str)
            .fetch_one(&mut *tx)
            .await?;
        
        if let Some(config_type) = config_type {
            sqlx::query("UPDATE saved_configurations SET is_active = false WHERE config_type = ?1")
                .bind(&config_type)
                .execute(&mut *tx)
                .await?;
        }
        
        // Activate the specified configuration
        let result = sqlx::query("UPDATE saved_configurations SET is_active = true, updated_at = ?1 WHERE id = ?2")
            .bind(Utc::now())
            .bind(&id_str)
            .execute(&mut *tx)
            .await?;
        
        tx.commit().await?;
        
        Ok(result.rows_affected() > 0)
    }

    pub async fn create_tables(&self) -> Result<(), sqlx::Error> {
        // Create saved_analyses table for SQLite
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS saved_analyses (
                id TEXT PRIMARY KEY,
                stock_code TEXT NOT NULL,
                stock_name TEXT NOT NULL,
                analysis_date TEXT NOT NULL,
                price_info TEXT NOT NULL,
                technical TEXT NOT NULL,
                fundamental TEXT NOT NULL,
                sentiment TEXT NOT NULL,
                scores TEXT NOT NULL,
                recommendation TEXT NOT NULL,
                ai_analysis TEXT NOT NULL,
                data_quality TEXT NOT NULL,
                ai_provider TEXT,
                ai_model TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        // Create saved_configurations table for SQLite
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS saved_configurations (
                id TEXT PRIMARY KEY,
                config_type TEXT NOT NULL,
                config_name TEXT NOT NULL,
                config_data TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for SQLite
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_saved_analyses_stock_code ON saved_analyses(stock_code)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_saved_analyses_analysis_date ON saved_analyses(analysis_date)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_saved_configurations_type ON saved_configurations(config_type)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_saved_configurations_active ON saved_configurations(is_active)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}