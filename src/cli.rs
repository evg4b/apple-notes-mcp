use crate::mcp::Scope;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about)]
pub(crate) struct Args {
    /// The scope of access to request from the user. Multiple scopes can be specified.
    #[clap(long, value_enum, default_value = "read")]
    pub(crate) scopes: Vec<Scope>,
    /// The path to the log file. If not specified, defaults to ~/Library/Logs/apple-notes-mcp.log
    #[clap(long)]
    pub(crate) log_file: Option<PathBuf>,
    /// The log level to use. If not specified, defaults to INFO.
    #[clap(long, value_enum)]
    pub(crate) log_level: Option<tracing::Level>,
}
