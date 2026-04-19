use super::models::{CreateNoteRequest, UpdateNoteRequest, WriteResponse};
use crate::mcp::AppleNotesMCP;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool};
use tracing::{debug, info, warn};

impl AppleNotesMCP {
    #[tool(description = "Create a new note in the default folder. \
                          content must be an HTML string, e.g. \"<b>Hello</b> world\". \
                          Use plain text wrapped in <div> tags if no formatting is needed.")]
    pub fn create_note(
        &self,
        p: Parameters<CreateNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        debug!(tool = "create_note", "called");
        let note = self
            .app
            .create_note(&p.0.title, &p.0.content)
            .inspect_err(|e| warn!(error = %e, "create_note failed"))
            .ok();
        let success = note.is_some();
        info!(tool = "create_note", success, "ok");
        Ok(Json(WriteResponse { success, note }))
    }

    #[tool(
        description = "Update the title and/or HTML body of an existing note by exact title. \
                          Omit new_title or new_content to leave that field unchanged. \
                          Returns success=false when no note with that title is found."
    )]
    pub fn update_note(
        &self,
        p: Parameters<UpdateNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        debug!(
            tool = "update_note",
            new_title = p.0.new_title.as_deref().unwrap_or("<unchanged>"),
            "called"
        );
        let note = self
            .app
            .update_note(
                &p.0.title,
                p.0.new_title.as_deref(),
                p.0.new_content.as_deref(),
            )
            .inspect_err(|e| warn!(error = %e, "update_note failed"))
            .ok()
            .flatten();
        let success = note.is_some();
        info!(tool = "update_note", success, "ok");
        Ok(Json(WriteResponse { success, note }))
    }
}
