mod mcp;
mod notes;

use anyhow::{Context, Result};
use mcp::AppleNotesMCP;
use notes::NotesApp;
use rmcp::{ServiceExt, transport::stdio};
use std::sync::Mutex;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

/// Default log path: ~/Library/Logs/apple-notes-mcp.log
/// Override with the APPLE_NOTES_MCP_LOG environment variable.
fn log_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("APPLE_NOTES_MCP_LOG") {
        return std::path::PathBuf::from(p);
    }
    let mut p = dirs_next::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    p.push("Library/Logs/apple-notes-mcp.log");
    p
}

#[tokio::main]
async fn main() -> Result<()> {
    let path = log_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create log directory {}", parent.display()))?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open log file {}", path.display()))?;

    // stdout is reserved for the MCP stdio transport — all logs go to the file.
    // Control verbosity with RUST_LOG, e.g.:
    //   RUST_LOG=debug                              — everything
    //   RUST_LOG=apple_notes_mcp=trace,rmcp=debug
    fmt()
        .with_writer(Mutex::new(file))
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!(log = %path.display(), "apple-notes-mcp starting");

    let notes_app = NotesApp::connect()?;
    let service = AppleNotesMCP::new(notes_app).serve(stdio()).await?;
    info!("MCP server ready, waiting for requests");
    service.waiting().await?;
    info!("MCP server shut down");
    Ok(())
}
