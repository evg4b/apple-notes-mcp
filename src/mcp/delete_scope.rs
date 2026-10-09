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
    pub fn delete_note(
        &self,
        Parameters(TitleRequest { title }): Parameters<TitleRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        let response = match self.app.delete_note(&title) {
            Ok(true) => WriteResponse::done(None),
            Ok(false) => WriteResponse::not_found(&title),
            Err(error) => {
                warn!(tool = "delete_note", %error, "delete failed");
                WriteResponse::failed(error.to_string())
            }
        };
        info!(tool = "delete_note", success = response.success, "ok");
        Ok(Json(response))
    }

    #[tool(
        description = "Delete a folder and every note in it, by exact name. Cannot be \
                       undone."
    )]
    pub fn delete_folder(
        &self,
        Parameters(FolderNameRequest { name }): Parameters<FolderNameRequest>,
    ) -> Result<Json<FolderWriteResponse>, String> {
        let response = match self.app.delete_folder(&name) {
            Ok(true) => FolderWriteResponse::done(None),
            Ok(false) => FolderWriteResponse::not_found(&name),
            Err(error) => {
                warn!(tool = "delete_folder", %error, "delete failed");
                FolderWriteResponse::failed(error.to_string())
            }
        };
        info!(tool = "delete_folder", success = response.success, "ok");
        Ok(Json(response))
    }
}
