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
    /// 環境変数から読み込み
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("SURREALDB_URL").unwrap_or_else(|_| "ws://localhost:12000".into()),
            namespace: std::env::var("SURREALDB_NS").unwrap_or_else(|_| "gfp".into()),
            database: std::env::var("SURREALDB_DB").unwrap_or_else(|_| "cad".into()),
            username: std::env::var("SURREALDB_USER").unwrap_or_else(|_| "admin".into()),
            password: std::env::var("SURREALDB_PASS").unwrap_or_else(|_| "admin-local-dev".into()),
        }
    }

    /// ローカル開発用
    pub fn local() -> Self {
        Self {
            url: "ws://localhost:12000".into(),
            namespace: "gfp".into(),
            database: "cad".into(),
            username: "admin".into(),
            password: "admin-local-dev".into(),
        }
    }
}
