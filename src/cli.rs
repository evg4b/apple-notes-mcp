use std::path::PathBuf;
use crate::mcp::Scope;
use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about)]
pub(crate) struct Args {
    /// The scope of access to request from the user. Multiple scopes can be specified.
    #[clap(long, value_enum, default_value = "read")]
    pub(crate) scopes: Vec<Scope>,
    pub(crate) log_file: Option<PathBuf>,   
}
