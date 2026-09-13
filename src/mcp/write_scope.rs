use super::AppleNotesMCP;
use super::models::{
    AppendNoteRequest, CreateFolderRequest, CreateNoteRequest, FolderWriteResponse,
    MoveNoteRequest, UpdateNoteRequest, WriteResponse,
};
use crate::notes::PartialNoteInfo;
use anyhow::Result;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool};
use tracing::{info, warn};

/// Map a note write to its response, reporting the reason on failure so the
/// client can tell "no such note" apart from "Notes refused the command".
fn note_result(
    tool: &'static str,
    title: &str,
    result: Result<Option<PartialNoteInfo>>,
) -> Json<WriteResponse> {
    let response = match result {
        Ok(Some(note)) => WriteResponse::found(note),
        Ok(None) => WriteResponse::not_found(title),
        Err(error) => {
            warn!(tool, %error, "write failed");
            WriteResponse::failed(error.to_string())
        }
    };
    info!(tool, success = response.success, "ok");
    Json(response)
}

impl AppleNotesMCP {
    #[tool(
        description = "Create a note. content is HTML — wrap plain text in <div> tags. \
                       Without folder it lands in the default folder."
    )]
    pub fn create_note(
        &self,
        p: Parameters<CreateNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        let created = self
            .app
            .create_note(&p.0.title, &p.0.content, p.0.folder.as_deref())
            .map(Some);
        Ok(note_result("create_note", &p.0.title, created))
    }

    #[tool(
        description = "Replace a note's title and/or body. Omitted fields stay as they \
                       are. new_content is HTML and replaces the whole body; read it back \
                       with get_note format=html first, or use append_to_note instead."
    )]
    pub fn update_note(
        &self,
        p: Parameters<UpdateNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        let updated = self.app.update_note(
            &p.0.title,
            p.0.new_title.as_deref(),
            p.0.new_content.as_deref(),
        );
        Ok(note_result("update_note", &p.0.title, updated))
    }

    #[tool(
        description = "Add HTML to the end of a note, keeping what is already there. \
                       Prefer this over update_note for adding a line."
    )]
    pub fn append_to_note(
        &self,
        p: Parameters<AppendNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        let appended = self.app.append_to_note(&p.0.title, &p.0.content);
        Ok(note_result("append_to_note", &p.0.title, appended))
    }

    #[tool(
        description = "Move a note to another folder, both by exact name. Keeps its id, \
                       dates and attachments."
    )]
    pub fn move_note(&self, p: Parameters<MoveNoteRequest>) -> Result<Json<WriteResponse>, String> {
        let moved = self.app.move_note(&p.0.title, &p.0.folder);
        Ok(note_result("move_note", &p.0.title, moved))
    }

    #[tool(
        description = "Create a top-level folder, in the first account unless account \
                       says otherwise. Nested folders are not supported."
    )]
    pub fn create_folder(
        &self,
        p: Parameters<CreateFolderRequest>,
    ) -> Result<Json<FolderWriteResponse>, String> {
        let response = match self.app.create_folder(&p.0.name, p.0.account.as_deref()) {
            Ok(folder) => FolderWriteResponse {
                success: true,
                folder: Some(folder),
                error: None,
            },
            Err(error) => {
                warn!(tool = "create_folder", %error, "write failed");
                FolderWriteResponse {
                    success: false,
                    folder: None,
                    error: Some(error.to_string()),
                }
            }
        };
        info!(tool = "create_folder", success = response.success, "ok");
        Ok(Json(response))
    }
}
