use chrono::Utc;
use sqlx::{postgres::PgPoolOptions, sqlite::SqlitePoolOptions, Pool, Postgres, Row, Sqlite};
use uuid::Uuid;

use crate::models::{
    AnalysisReport, HistoryQuery, HistoryResponse, SavedAnalysis, SavedConfiguration,
};

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

    pub async fn save_analysis(
        &self,
        report: &AnalysisReport,
        ai_provider: Option<String>,
        ai_model: Option<String>,
    ) -> Result<(), sqlx::Error> {
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
                    "#,
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
                    "#,
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

    pub async fn get_analysis_history(
        &self,
        query: &HistoryQuery,
    ) -> Result<HistoryResponse, sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                let limit = query.limit.unwrap_or(20).min(100);
                let offset = query.offset.unwrap_or(0);

                // Get total count
                let count_query = if let Some(ref stock_code) = query.stock_code {
                    if stock_code.is_empty() {
                        "SELECT COUNT(*) as total FROM saved_analyses"
                    } else {
                        "SELECT COUNT(*) as total FROM saved_analyses WHERE stock_code = ?1"
                    }
                } else {
                    "SELECT COUNT(*) as total FROM saved_analyses"
                };

                let total_count = if let Some(ref stock_code) = query.stock_code {
                    if stock_code.is_empty() {
                        sqlx::query(count_query)
                            .fetch_one(pool)
                            .await?
                            .get::<i64, _>("total")
                    } else {
                        sqlx::query(count_query)
                            .bind(stock_code)
                            .fetch_one(pool)
                            .await?
                            .get::<i64, _>("total")
                    }
                } else {
                    sqlx::query(count_query)
                        .fetch_one(pool)
                        .await?
                        .get::<i64, _>("total")
                };

                // Get paginated data
                let (data_query, binds) = if let Some(ref stock_code) = query.stock_code {
                    if stock_code.is_empty() {
                        (
                            "SELECT * FROM saved_analyses ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
                            vec![limit.to_string(), offset.to_string()]
                        )
                    } else {
                        (
                            "SELECT * FROM saved_analyses WHERE stock_code = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
                            vec![stock_code.clone(), limit.to_string(), offset.to_string()]
                        )
                    }
                } else {
                    (
                        "SELECT * FROM saved_analyses ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
                        vec![limit.to_string(), offset.to_string()],
                    )
                };

                let query_builder = if binds.len() == 2 {
                    sqlx::query(data_query).bind(&binds[0]).bind(&binds[1])
                } else if binds.len() == 3 {
                    sqlx::query(data_query)
                        .bind(&binds[0])
                        .bind(&binds[1])
                        .bind(&binds[2])
                } else {
                    sqlx::query(data_query)
                };

                let rows = query_builder.fetch_all(pool).await?;
                let mut analyses = Vec::new();

                for row in rows {
                    let analysis = SavedAnalysis {
                        id: row.get("id"),
                        stock_code: row.get("stock_code"),
                        stock_name: row.get("stock_name"),
                        analysis_date: row.get("analysis_date"),
                        price_info: serde_json::from_value(row.get("price_info"))
                            .unwrap_or_default(),
                        technical: serde_json::from_value(row.get("technical")).unwrap_or_default(),
                        fundamental: serde_json::from_value(row.get("fundamental"))
                            .unwrap_or_default(),
                        sentiment: serde_json::from_value(row.get("sentiment")).unwrap_or_default(),
                        scores: serde_json::from_value(row.get("scores")).unwrap_or_default(),
                        recommendation: row.get("recommendation"),
                        ai_analysis: row.get("ai_analysis"),
                        data_quality: serde_json::from_value(row.get("data_quality"))
                            .unwrap_or_default(),
                        ai_provider: row.get("ai_provider"),
                        ai_model: row.get("ai_model"),
                        created_at: row.get("created_at"),
                    };
                    analyses.push(analysis);
                }

                Ok(HistoryResponse {
                    analyses,
                    total: total_count,
                    query: query.clone(),
                })
            }
            Database::Postgres(pool) => {
                let limit = query.limit.unwrap_or(20).min(100);
                let offset = query.offset.unwrap_or(0);

                // Get total count
                let count_query = if let Some(ref stock_code) = query.stock_code {
                    if stock_code.is_empty() {
                        "SELECT COUNT(*) as total FROM saved_analyses"
                    } else {
                        "SELECT COUNT(*) as total FROM saved_analyses WHERE stock_code = $1"
                    }
                } else {
                    "SELECT COUNT(*) as total FROM saved_analyses"
                };

                let total_count = if let Some(ref stock_code) = query.stock_code {
                    if stock_code.is_empty() {
                        sqlx::query(count_query)
                            .fetch_one(pool)
                            .await?
                            .get::<i64, _>("total")
                    } else {
                        sqlx::query(count_query)
                            .bind(stock_code)
                            .fetch_one(pool)
                            .await?
                            .get::<i64, _>("total")
                    }
                } else {
                    sqlx::query(count_query)
                        .fetch_one(pool)
                        .await?
                        .get::<i64, _>("total")
                };

                // Get paginated data
                let (data_query, binds) = if let Some(ref stock_code) = query.stock_code {
                    if stock_code.is_empty() {
                        (
                            "SELECT * FROM saved_analyses ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                            vec![limit.to_string(), offset.to_string()]
                        )
                    } else {
                        (
                            "SELECT * FROM saved_analyses WHERE stock_code = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                            vec![stock_code.clone(), limit.to_string(), offset.to_string()]
                        )
                    }
                } else {
                    (
                        "SELECT * FROM saved_analyses ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                        vec![limit.to_string(), offset.to_string()],
                    )
                };

                let query_builder = if binds.len() == 2 {
                    sqlx::query(data_query).bind(&binds[0]).bind(&binds[1])
                } else if binds.len() == 3 {
                    sqlx::query(data_query)
                        .bind(&binds[0])
                        .bind(&binds[1])
                        .bind(&binds[2])
                } else {
                    sqlx::query(data_query)
                };

                let rows = query_builder.fetch_all(pool).await?;
                let mut analyses = Vec::new();

                for row in rows {
                    let analysis = SavedAnalysis {
                        id: row.get("id"),
                        stock_code: row.get("stock_code"),
                        stock_name: row.get("stock_name"),
                        analysis_date: row.get("analysis_date"),
                        price_info: serde_json::from_value(row.get("price_info"))
                            .unwrap_or_default(),
                        technical: serde_json::from_value(row.get("technical")).unwrap_or_default(),
                        fundamental: serde_json::from_value(row.get("fundamental"))
                            .unwrap_or_default(),
                        sentiment: serde_json::from_value(row.get("sentiment")).unwrap_or_default(),
                        scores: serde_json::from_value(row.get("scores")).unwrap_or_default(),
                        recommendation: row.get("recommendation"),
                        ai_analysis: row.get("ai_analysis"),
                        data_quality: serde_json::from_value(row.get("data_quality"))
                            .unwrap_or_default(),
                        ai_provider: row.get("ai_provider"),
                        ai_model: row.get("ai_model"),
                        created_at: row.get("created_at"),
                    };
                    analyses.push(analysis);
                }

                Ok(HistoryResponse {
                    analyses,
                    total: total_count,
                    query: query.clone(),
                })
            }
        }
    }

    pub async fn get_analysis_by_id(&self, id: Uuid) -> Result<Option<SavedAnalysis>, sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                let query = "SELECT * FROM saved_analyses WHERE id = ?1";
                match sqlx::query(query)
                    .bind(id.to_string())
                    .fetch_optional(pool)
                    .await?
                {
                    Some(row) => {
                        let analysis = SavedAnalysis {
                            id: row.get("id"),
                            stock_code: row.get("stock_code"),
                            stock_name: row.get("stock_name"),
                            analysis_date: row.get("analysis_date"),
                            price_info: serde_json::from_value(row.get("price_info"))
                                .unwrap_or_default(),
                            technical: serde_json::from_value(row.get("technical"))
                                .unwrap_or_default(),
                            fundamental: serde_json::from_value(row.get("fundamental"))
                                .unwrap_or_default(),
                            sentiment: serde_json::from_value(row.get("sentiment"))
                                .unwrap_or_default(),
                            scores: serde_json::from_value(row.get("scores")).unwrap_or_default(),
                            recommendation: row.get("recommendation"),
                            ai_analysis: row.get("ai_analysis"),
                            data_quality: serde_json::from_value(row.get("data_quality"))
                                .unwrap_or_default(),
                            ai_provider: row.get("ai_provider"),
                            ai_model: row.get("ai_model"),
                            created_at: row.get("created_at"),
                        };
                        Ok(Some(analysis))
                    }
                    None => Ok(None),
                }
            }
            Database::Postgres(pool) => {
                let query = "SELECT * FROM saved_analyses WHERE id = $1";
                match sqlx::query(query).bind(id).fetch_optional(pool).await? {
                    Some(row) => {
                        let analysis = SavedAnalysis {
                            id: row.get("id"),
                            stock_code: row.get("stock_code"),
                            stock_name: row.get("stock_name"),
                            analysis_date: row.get("analysis_date"),
                            price_info: serde_json::from_value(row.get("price_info"))
                                .unwrap_or_default(),
                            technical: serde_json::from_value(row.get("technical"))
                                .unwrap_or_default(),
                            fundamental: serde_json::from_value(row.get("fundamental"))
                                .unwrap_or_default(),
                            sentiment: serde_json::from_value(row.get("sentiment"))
                                .unwrap_or_default(),
                            scores: serde_json::from_value(row.get("scores")).unwrap_or_default(),
                            recommendation: row.get("recommendation"),
                            ai_analysis: row.get("ai_analysis"),
                            data_quality: serde_json::from_value(row.get("data_quality"))
                                .unwrap_or_default(),
                            ai_provider: row.get("ai_provider"),
                            ai_model: row.get("ai_model"),
                            created_at: row.get("created_at"),
                        };
                        Ok(Some(analysis))
                    }
                    None => Ok(None),
                }
            }
        }
    }

    pub async fn save_configuration(
        &self,
        config_type: &str,
        config_name: &str,
        config_data: &serde_json::Value,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();

        match self {
            Database::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO saved_configurations (
                        id, config_type, config_name, config_data, is_active, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, true, ?5, ?6)
                    "#,
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
                    "#,
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

    pub async fn get_active_configuration(
        &self,
        _config_type: &str,
    ) -> Result<Option<SavedConfiguration>, sqlx::Error> {
        // For now, return None to avoid complex query handling
        Ok(None)
    }

    pub async fn list_configurations(
        &self,
        _config_type: Option<&str>,
    ) -> Result<Vec<SavedConfiguration>, sqlx::Error> {
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
        match self {
            Database::Sqlite(pool) => {
                // Create tables for SQLite
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
                        ai_analysis TEXT,
                        data_quality TEXT NOT NULL,
                        ai_provider TEXT,
                        ai_model TEXT,
                        created_at TEXT NOT NULL
                    )
                    "#,
                )
                .execute(pool)
                .await?;

                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS saved_configurations (
                        id TEXT PRIMARY KEY,
                        config_type TEXT NOT NULL,
                        config_name TEXT NOT NULL,
                        config_data TEXT NOT NULL,
                        is_active INTEGER DEFAULT 0,
                        created_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL
                    )
                    "#,
                )
                .execute(pool)
                .await?;
            }
            Database::Postgres(pool) => {
                // For PostgreSQL, tables should be created by init script
                // But let's verify they exist and create them if needed
                let table_exists = sqlx::query(
                    "SELECT EXISTS (
                        SELECT FROM information_schema.tables 
                        WHERE table_schema = 'public' 
                        AND table_name = 'saved_analyses'
                    )",
                )
                .fetch_one(pool)
                .await?;

                let exists: bool = table_exists.get("exists");
                if !exists {
                    log::warn!("saved_analyses table does not exist, attempting to create it");
                    // Try to create the table
                    sqlx::query(
                        r#"
                        CREATE TABLE saved_analyses (
                            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                            stock_code VARCHAR(20) NOT NULL,
                            stock_name VARCHAR(100) NOT NULL,
                            analysis_date TIMESTAMP WITH TIME ZONE NOT NULL,
                            price_info JSONB NOT NULL,
                            technical JSONB NOT NULL,
                            fundamental JSONB NOT NULL,
                            sentiment JSONB NOT NULL,
                            scores JSONB NOT NULL,
                            recommendation VARCHAR(50) NOT NULL,
                            ai_analysis TEXT,
                            data_quality JSONB NOT NULL,
                            ai_provider VARCHAR(50),
                            ai_model VARCHAR(50),
                            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
                        )
                        "#,
                    )
                    .execute(pool)
                    .await?;
                }
            }
        }
        Ok(())
    }
}
