mod apple_notes_mcp;
mod notes;

use std::sync::Mutex;

use apple_notes_mcp::AppleNotesMCP;
use anyhow::{Context, Result};
use rmcp::{transport::stdio, ServiceExt};
use tracing::{info, warn};
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
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!(log = %path.display(), "apple-notes-mcp starting");

    // Probe Notes.app at startup.
    // This forces macOS to show the Automation permission dialog on the first
    // run before the server enters the stdio loop.  Without this the dialog
    // never fires because the ScriptingBridge connection is lazy.
    match notes::list_accounts() {
        Ok(accounts) if accounts.is_empty() => {
            warn!(
                "Notes returned 0 accounts — Automation permission is probably missing. \
                 Go to System Settings → Privacy & Security → Automation and allow \
                 this binary to control Notes.app, then restart."
            );
        }
        Ok(accounts) => {
            info!(accounts = accounts.len(), "Notes.app connected");
        }
        Err(e) => {
            warn!(error = %e, "Notes.app probe failed — check Automation permission");
        }
    }

    let service = AppleNotesMCP.serve(stdio()).await?;
    info!("MCP server ready, waiting for requests");
    service.waiting().await?;
    info!("MCP server shut down");
    Ok(())
}
