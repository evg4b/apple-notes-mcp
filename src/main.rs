mod apple_notes_mcp;
mod notes;

use apple_notes_mcp::AppleNotesMCP;
use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr — stdout is reserved for the MCP stdio transport.
    // Control verbosity with RUST_LOG, e.g.:
    //   RUST_LOG=debug          — everything
    //   RUST_LOG=apple_notes_mcp=trace,rmcp=debug
    fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("apple-notes-mcp starting");
    let service = AppleNotesMCP.serve(stdio()).await?;
    info!("MCP server ready, waiting for requests");
    service.waiting().await?;
    info!("MCP server shut down");
    Ok(())
}
