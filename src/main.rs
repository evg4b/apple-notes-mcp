mod cli;
mod log;
mod mcp;
mod notes;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Args;
use mcp::{AppleNotesMCP, Scope, ScopeSet};
use notes::NotesApp;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    log::init(args.log_file, args.log_level)?;

    // MCP clients rarely show a server's stderr, so the log file is where a
    // failure gets seen.
    let result = serve(args.scopes).await;
    if let Err(error) = &result {
        error!(error = format!("{error:#}"), "MCP server failed");
    }
    result
}

async fn serve(scopes: Vec<Scope>) -> Result<()> {
    let notes_app = NotesApp::connect()?;
    let service = AppleNotesMCP::new(notes_app, ScopeSet::from_iter(scopes))
        .serve(stdio())
        .await
        .context("start the MCP server on stdio")?;

    info!("MCP server ready, waiting for requests");
    service
        .waiting()
        .await
        .context("MCP server stopped unexpectedly")?;
    info!("MCP server shut down");

    Ok(())
}
