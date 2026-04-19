use super::AppleNotesMCP;
use super::models::{TitleRequest, WriteResponse};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool};
use tracing::{debug, info, warn};

impl AppleNotesMCP {
    #[tool(
        description = "Permanently delete a note by exact title. Cannot be undone. \
                          Returns success=false when no note with that title is found."
    )]
    pub fn delete_note(&self, p: Parameters<TitleRequest>) -> Result<Json<WriteResponse>, String> {
        debug!(tool = "delete_note", "called");
        let success = self
            .app
            .delete_note(&p.0.title)
            .inspect_err(|e| warn!(error = %e, "delete_note failed"))
            .unwrap_or(false);
        info!(tool = "delete_note", success, "ok");
        Ok(Json(WriteResponse {
            success,
            note: None,
        }))
    }
}
