mod apple_notes_mcp;
mod notes;

use apple_notes_mcp::AppleNotesMCP;
use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};

#[tokio::main]
async fn main() -> Result<()> {
    let service = AppleNotesMCP.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
