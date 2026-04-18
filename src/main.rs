mod cli;
mod mcp;
mod models;
mod notes;

use crate::cli::Args;
use anyhow::{Context, Result};
use clap::Parser;
use mcp::AppleNotesMCP;
use notes::NotesApp;
use rmcp::{transport::stdio, ServiceExt};
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::info;
use tracing_subscriber::{fmt};

fn default_log_path() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library/Logs/apple-notes-mcp.log"))
        .unwrap_or_else(|| PathBuf::from("apple-notes-mcp.log"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let log_file_path = args.log_file.unwrap_or_else(default_log_path);

    if let Some(parent) = log_file_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create log directory {}", parent.display()))?;
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .with_context(|| format!("open log file {}", log_file_path.display()))?;

    fmt().with_writer(Mutex::new(file)).with_ansi(false).init();

    info!(log = %log_file_path.display(), "apple-notes-mcp starting");

    let notes_app = NotesApp::connect()?;
    let service = AppleNotesMCP::new(notes_app, args.scopes)
        .serve(stdio())
        .await?;
    info!("MCP server ready, waiting for requests");
    service.waiting().await?;
    info!("MCP server shut down");
    Ok(())
}
