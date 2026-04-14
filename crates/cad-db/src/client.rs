use anyhow::{Context, Result};
use surrealdb::Surreal;
use surrealdb::engine::remote::http::{Client, Http};

use crate::config::DbConfig;

/// SurrealDB クライアント
pub struct CadDbClient {
    pub db: Surreal<Client>,
}

impl CadDbClient {
    /// 接続して初期化（HTTP プロトコル）
    pub async fn connect(config: &DbConfig) -> Result<Self> {
        // SDK は scheme を自前で付加するので、ホスト:ポートだけ渡す
        let url = config
            .url
            .replace("ws://", "")
            .replace("wss://", "")
            .replace("http://", "")
            .replace("https://", "");

        let db = Surreal::new::<Http>(&url)
            .await
            .context("SurrealDB 接続失敗")?;

        db.signin(surrealdb::opt::auth::Namespace {
            namespace: config.namespace.clone(),
            username: config.username.clone(),
            password: config.password.clone(),
        })
        .await
        .context("SurrealDB 認証失敗")?;

        db.use_ns(&config.namespace)
            .use_db(&config.database)
            .await
            .context("NS/DB 選択失敗")?;

        tracing::info!(
            "SurrealDB connected: {}/{}/{}",
            url,
            config.namespace,
            config.database
        );

        Ok(Self { db })
    }

    /// スキーマを初期化
    pub async fn init_schema(&self) -> Result<()> {
        let schema = include_str!("../schema/init.surql");
        self.db.query(schema).await.context("スキーマ初期化失敗")?;
        tracing::info!("Schema initialized");
        Ok(())
    }
}
