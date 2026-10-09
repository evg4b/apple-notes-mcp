use super::AppleNotesMCP;
use super::models::{FolderNameRequest, FolderWriteResponse, TitleRequest, WriteResponse};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool};
use tracing::{info, warn};

impl AppleNotesMCP {
    #[tool(
        description = "Delete a note by exact title. iCloud moves it to Recently Deleted; \
                       in other accounts it is gone for good. Confirm first."
    )]
    pub async fn delete_note(
        &self,
        Parameters(TitleRequest { title }): Parameters<TitleRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        self.blocking(move |app| {
            let response = match app.delete_note(&title) {
                Ok(true) => WriteResponse::done(None),
                Ok(false) => WriteResponse::not_found(&title),
                Err(error) => {
                    warn!(
                        tool = "delete_note",
                        error = format!("{error:#}"),
                        "delete failed"
                    );
                    WriteResponse::failed(format!("{error:#}"))
                }
            };
            info!(tool = "delete_note", success = response.success, "ok");
            Json(response)
        })
        .await
    }

    #[tool(
        description = "Delete a folder and every note in it, by exact name. Cannot be \
                       undone."
    )]
    pub async fn delete_folder(
        &self,
        Parameters(FolderNameRequest { name }): Parameters<FolderNameRequest>,
    ) -> Result<Json<FolderWriteResponse>, String> {
        self.blocking(move |app| {
            let response = match app.delete_folder(&name) {
                Ok(true) => FolderWriteResponse::done(None),
                Ok(false) => FolderWriteResponse::not_found(&name),
                Err(error) => {
                    warn!(
                        tool = "delete_folder",
                        error = format!("{error:#}"),
                        "delete failed"
                    );
                    FolderWriteResponse::failed(format!("{error:#}"))
                }
            };
            info!(tool = "delete_folder", success = response.success, "ok");
            Json(response)
        })
        .await
    }
}
