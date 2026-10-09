use crate::mcp::Scope;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about)]
pub(crate) struct Args {
    /// Tool groups to enable, comma-separated.
    #[clap(long, value_enum, value_delimiter = ',', default_value = "read")]
    pub(crate) scopes: Vec<Scope>,
    /// Log file. Defaults to ~/Library/Logs/apple-notes-mcp/apple-notes-mcp.log
    #[clap(long)]
    pub(crate) log_file: Option<PathBuf>,
    /// Log verbosity. Defaults to ERROR.
    #[clap(long, value_enum)]
    pub(crate) log_level: Option<tracing::Level>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_default() {
        let args = Args::parse_from(vec!["app"]);
        assert_eq!(args.scopes, vec![Scope::Read]);
    }

    #[test]
    fn scopes_read_write() {
        let args = Args::parse_from(vec!["app", "--scopes", "read,write"]);
        assert_eq!(args.scopes, vec![Scope::Read, Scope::Write]);
    }

    #[test]
    fn scopes_read_write_delete() {
        let args = Args::parse_from(vec!["app", "--scopes", "read,write,delete"]);
        assert_eq!(args.scopes, vec![Scope::Read, Scope::Write, Scope::Delete]);
    }

    #[test]
    fn log_file_default() {
        let args = Args::parse_from(vec!["app"]);
        assert_eq!(args.log_file, None);
    }

    #[test]
    fn log_file_custom() {
        let args = Args::parse_from(vec!["app", "--log-file", "/tmp/log.txt"]);
        assert_eq!(args.log_file, Some(PathBuf::from("/tmp/log.txt")));
    }

    #[test]
    fn log_level_default() {
        let args = Args::parse_from(vec!["app"]);
        assert_eq!(args.log_level, None);
    }

    #[test]
    fn log_level_info() {
        let args = Args::parse_from(vec!["app", "--log-level", "info"]);
        assert_eq!(args.log_level, Some(tracing::Level::INFO));
    }

    #[test]
    fn log_level_debug() {
        let args = Args::parse_from(vec!["app", "--log-level", "debug"]);
        assert_eq!(args.log_level, Some(tracing::Level::DEBUG));
    }
}
