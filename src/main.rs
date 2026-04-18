mod cli;
mod log;
mod mcp;
mod models;
mod notes;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use mcp::AppleNotesMCP;
use notes::NotesApp;
use rmcp::{ServiceExt, transport::stdio};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    log::init(args.log_file, args.log_level)?;

    let notes_app = NotesApp::connect()?;
    let service = AppleNotesMCP::new(notes_app, args.scopes)
        .serve(stdio())
        .await?;

    info!("MCP server ready, waiting for requests");
    service.waiting().await?;
    info!("MCP server shut down");

    Ok(())
}
