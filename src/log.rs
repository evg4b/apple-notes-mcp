use anyhow::{Context, Result};
use std::env;
use std::fs::{OpenOptions, create_dir_all};
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::{Level, debug};
use tracing_subscriber::fmt;

static LOG_FILE_NAME: &str = "apple-notes-mcp.log";
static LOG_DIR: &str = "Library/Logs/apple-notes-mcp";

/// `~/Library/Logs/apple-notes-mcp/apple-notes-mcp.log`, falling back to the
/// working directory when `$HOME` is unset.
fn default_log_path() -> PathBuf {
    match env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join(LOG_DIR).join(LOG_FILE_NAME),
        None => PathBuf::from(LOG_FILE_NAME),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_log_path_lives_under_home() {
        let path = default_log_path();
        assert!(path.ends_with(LOG_FILE_NAME), "unexpected path: {path:?}");
        if env::var_os("HOME").is_some() {
            assert!(path.is_absolute(), "expected an absolute path: {path:?}");
            assert!(
                path.to_string_lossy().contains(LOG_DIR),
                "not under the log directory: {path:?}"
            );
        }
    }
}
