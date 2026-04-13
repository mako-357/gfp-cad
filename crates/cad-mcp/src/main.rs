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

    let server = server::GfpCadMcpServer::new();
    let service = server.serve(rmcp::transport::stdio()).await?;

    tracing::info!("gfp-cad MCP Server running");
    service.waiting().await?;
    Ok(())
}
