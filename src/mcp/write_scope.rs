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

/// Keeps "no such note" distinct from "Notes refused the command".
fn note_result(
    tool: &'static str,
    title: &str,
    result: Result<Option<PartialNoteInfo>>,
) -> Json<WriteResponse> {
    let response = match result {
        Ok(Some(note)) => WriteResponse::done(Some(note)),
        Ok(None) => WriteResponse::not_found(title),
        Err(error) => {
            warn!(tool, error = format!("{error:#}"), "write failed");
            WriteResponse::failed(format!("{error:#}"))
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
    pub async fn create_note(
        &self,
        Parameters(req): Parameters<CreateNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        self.blocking(move |app| {
            let created = app
                .create_note(&req.title, &req.content, req.folder.as_deref())
                .map(Some);
            note_result("create_note", &req.title, created)
        })
        .await
    }

    #[tool(
        description = "Replace a note's title and/or body. Omitted fields stay as they \
                       are. new_content is HTML and replaces the whole body; read it back \
                       with get_note format=html first, or use append_to_note instead."
    )]
    pub async fn update_note(
        &self,
        Parameters(req): Parameters<UpdateNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        self.blocking(move |app| {
            let updated = app.update_note(
                &req.title,
                req.new_title.as_deref(),
                req.new_content.as_deref(),
            );
            note_result("update_note", &req.title, updated)
        })
        .await
    }

    #[tool(
        description = "Add HTML to the end of a note, keeping what is already there. \
                       Prefer this over update_note for adding a line."
    )]
    pub async fn append_to_note(
        &self,
        Parameters(req): Parameters<AppendNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        self.blocking(move |app| {
            let appended = app.append_to_note(&req.title, &req.content);
            note_result("append_to_note", &req.title, appended)
        })
        .await
    }

    #[tool(
        description = "Move a note to another folder, both by exact name. Within one \
                       account it keeps its id, dates and attachments."
    )]
    pub async fn move_note(
        &self,
        Parameters(req): Parameters<MoveNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        self.blocking(move |app| {
            let moved = app.move_note(&req.title, &req.folder);
            note_result("move_note", &req.title, moved)
        })
        .await
    }

    #[tool(
        description = "Create a top-level folder, in the default account unless account \
                       says otherwise. Nested folders are not supported."
    )]
    pub async fn create_folder(
        &self,
        Parameters(req): Parameters<CreateFolderRequest>,
    ) -> Result<Json<FolderWriteResponse>, String> {
        self.blocking(move |app| {
            let response = match app.create_folder(&req.name, req.account.as_deref()) {
                Ok(folder) => FolderWriteResponse::done(Some(folder)),
                Err(error) => {
                    warn!(
                        tool = "create_folder",
                        error = format!("{error:#}"),
                        "write failed"
                    );
                    FolderWriteResponse::failed(format!("{error:#}"))
                }
            };
            info!(tool = "create_folder", success = response.success, "ok");
            Json(response)
        })
        .await
    }
}
