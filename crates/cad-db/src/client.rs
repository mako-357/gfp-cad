use anyhow::{Context, Result};
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::{Client, Ws};

use crate::config::DbConfig;

/// SurrealDB クライアント
pub struct CadDbClient {
    pub db: Surreal<Client>,
}

impl CadDbClient {
    /// 接続して初期化
    pub async fn connect(config: &DbConfig) -> Result<Self> {
        let db = Surreal::new::<Ws>(&config.url)
            .await
            .context("SurrealDB 接続失敗")?;

        db.signin(surrealdb::opt::auth::Namespace {
            namespace: &config.namespace,
            username: &config.username,
            password: &config.password,
        })
        .await
        .context("SurrealDB 認証失敗")?;

        db.use_ns(&config.namespace)
            .use_db(&config.database)
            .await
            .context("NS/DB 選択失敗")?;

        tracing::info!(
            "SurrealDB connected: {}/{}/{}",
            config.url,
            config.namespace,
            config.database
        );

        Ok(Self { db })
    }

    /// スキーマを初期化
    pub async fn init_schema(&self) -> Result<()> {
        let schema = include_str!("../schema/init.surql");
        self.db
            .query(schema)
            .await
            .context("スキーマ初期化失敗")?;
        tracing::info!("Schema initialized");
        Ok(())
    }
}
