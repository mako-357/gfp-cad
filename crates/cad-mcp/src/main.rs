mod server;

use anyhow::Result;
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    tracing::info!("gfp-cad MCP Server starting...");

    // DB 接続（オプション — 接続できなくてもローカルモードで動作）
    let config = match cad_db::DbConfig::from_env() {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!("DB config: {e} (running in local-only mode)");
            None
        }
    };
    let db = match config {
        None => None,
        Some(ref cfg) => match cad_db::CadDbClient::connect(cfg).await {
            Ok(client) => {
                if let Err(e) = client.init_schema().await {
                    // スキーマ初期化の失敗は info レベル
                    // （権限不足は想定内: 他のプロセスが既にスキーマを作成済みの場合など）
                    tracing::info!("Schema init skipped: {e}");
                }
                tracing::info!("SurrealDB connected");
                Some(client)
            }
            Err(e) => {
                tracing::warn!("SurrealDB not available: {e} (running in local-only mode)");
                None
            }
        },
    };

    let server = server::GfpCadMcpServer::new(db);
    let service = server.serve(rmcp::transport::stdio()).await?;

    tracing::info!("gfp-cad MCP Server running");
    service.waiting().await?;
    Ok(())
}
