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

/// Reports the reason on failure so the client can tell "no such note" apart
/// from "Notes refused the command".
fn note_result(
    tool: &'static str,
    title: &str,
    result: Result<Option<PartialNoteInfo>>,
) -> Json<WriteResponse> {
    let response = match result {
        Ok(Some(note)) => WriteResponse::done(Some(note)),
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
        Parameters(req): Parameters<CreateNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        let created = self
            .app
            .create_note(&req.title, &req.content, req.folder.as_deref())
            .map(Some);
        Ok(note_result("create_note", &req.title, created))
    }

    #[tool(
        description = "Replace a note's title and/or body. Omitted fields stay as they \
                       are. new_content is HTML and replaces the whole body; read it back \
                       with get_note format=html first, or use append_to_note instead."
    )]
    pub fn update_note(
        &self,
        Parameters(req): Parameters<UpdateNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        let updated = self.app.update_note(
            &req.title,
            req.new_title.as_deref(),
            req.new_content.as_deref(),
        );
        Ok(note_result("update_note", &req.title, updated))
    }

    #[tool(
        description = "Add HTML to the end of a note, keeping what is already there. \
                       Prefer this over update_note for adding a line."
    )]
    pub fn append_to_note(
        &self,
        Parameters(req): Parameters<AppendNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        let appended = self.app.append_to_note(&req.title, &req.content);
        Ok(note_result("append_to_note", &req.title, appended))
    }

    #[tool(
        description = "Move a note to another folder, both by exact name. Within one \
                       account it keeps its id, dates and attachments."
    )]
    pub fn move_note(
        &self,
        Parameters(req): Parameters<MoveNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        let moved = self.app.move_note(&req.title, &req.folder);
        Ok(note_result("move_note", &req.title, moved))
    }

    #[tool(
        description = "Create a top-level folder, in the default account unless account \
                       says otherwise. Nested folders are not supported."
    )]
    pub fn create_folder(
        &self,
        Parameters(req): Parameters<CreateFolderRequest>,
    ) -> Result<Json<FolderWriteResponse>, String> {
        let response = match self.app.create_folder(&req.name, req.account.as_deref()) {
            Ok(folder) => FolderWriteResponse::done(Some(folder)),
            Err(error) => {
                warn!(tool = "create_folder", %error, "write failed");
                FolderWriteResponse::failed(error.to_string())
            }
        };
        info!(tool = "create_folder", success = response.success, "ok");
        Ok(Json(response))
    }
}
