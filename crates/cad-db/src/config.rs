use anyhow::{Context, Result};

/// SurrealDB 接続設定
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub url: String,
    pub namespace: String,
    pub database: String,
    pub username: String,
    pub password: String,
}

impl DbConfig {
    /// 環境変数から読み込み。SURREALDB_PASS は必須。
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            url: std::env::var("SURREALDB_URL").unwrap_or_else(|_| "http://localhost:12000".into()),
            namespace: std::env::var("SURREALDB_NS").unwrap_or_else(|_| "gfp".into()),
            database: std::env::var("SURREALDB_DB").unwrap_or_else(|_| "cad".into()),
            username: std::env::var("SURREALDB_USER").unwrap_or_else(|_| "admin".into()),
            password: std::env::var("SURREALDB_PASS")
                .context("SURREALDB_PASS 環境変数が設定されていません")?,
        })
    }

    /// テスト用ローカル設定
    #[cfg(test)]
    pub fn local() -> Self {
        Self {
            url: "http://localhost:12000".into(),
            namespace: "gfp".into(),
            database: "cad".into(),
            username: "admin".into(),
            password: "admin-local-dev".into(),
        }
    }
}
