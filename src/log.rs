use anyhow::{Context, Result};
use std::env;
use std::fs::{OpenOptions, create_dir_all};
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::{Level, debug};
use tracing_subscriber::fmt;

static LOG_FILE_NAME: &str = "apple-notes-mcp.log";
static DEFAULT_LOG_PATH: &str = "Library/Logs/apple-notes-mcp/apple-notes-mcp.log";

fn default_log_path() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(DEFAULT_LOG_PATH))
        .unwrap_or_else(|| PathBuf::from(LOG_FILE_NAME))
}

pub(crate) fn init(log_file: Option<PathBuf>, max_level: Option<Level>) -> Result<()> {
    let log_file_path = log_file.unwrap_or_else(default_log_path);
    if let Some(parent) = log_file_path.parent() {
        create_dir_all(parent).with_context(|| format!("create log directory {:?}", parent))?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .with_context(|| format!("open log file {:?}", log_file_path))?;

    fmt()
        .with_writer(Mutex::new(file))
        .with_ansi(false)
        .with_level(true)
        .with_max_level(max_level.unwrap_or(Level::ERROR))
        .init();

    debug!(log = %log_file_path.display(), "log file initialized");

    Ok(())
}
