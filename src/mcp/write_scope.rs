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
        description = "Create a new note. content must be an HTML string, e.g. \
                       \"<b>Hello</b> world\"; wrap plain text in <div> tags if no \
                       formatting is needed. Pass folder to place the note in a specific \
                       folder, otherwise the Notes default folder is used."
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
        description = "Replace the title and/or HTML body of an existing note, matched by \
                       exact title. Omit new_title or new_content to leave that field \
                       unchanged. Use append_to_note to add to a body without replacing it."
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
        description = "Append HTML to the end of an existing note's body, matched by exact \
                       title. The existing content is preserved. Returns success=false \
                       when no note with that title is found."
    )]
    pub fn append_to_note(
        &self,
        p: Parameters<AppendNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        let appended = self.app.append_to_note(&p.0.title, &p.0.content);
        Ok(note_result("append_to_note", &p.0.title, appended))
    }

    #[tool(
        description = "Move a note into another folder, both matched by exact name. The \
                       note keeps its id, dates and attachments. Use list_folders to \
                       discover valid destination folders."
    )]
    pub fn move_note(&self, p: Parameters<MoveNoteRequest>) -> Result<Json<WriteResponse>, String> {
        let moved = self.app.move_note(&p.0.title, &p.0.folder);
        Ok(note_result("move_note", &p.0.title, moved))
    }

    #[tool(
        description = "Create a top-level folder in an account. Pass account to choose \
                       which one, otherwise the first account is used. Nested folders are \
                       not supported."
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
