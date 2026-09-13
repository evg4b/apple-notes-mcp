use super::AppleNotesMCP;
use super::models::{FolderNameRequest, FolderWriteResponse, TitleRequest, WriteResponse};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool};
use tracing::{info, warn};

impl AppleNotesMCP {
    #[tool(
        description = "Permanently delete a note by exact title. Cannot be undone. \
                       Returns success=false when no note with that title is found."
    )]
    pub fn delete_note(&self, p: Parameters<TitleRequest>) -> Result<Json<WriteResponse>, String> {
        let response = match self.app.delete_note(&p.0.title) {
            Ok(true) => WriteResponse {
                success: true,
                ..Default::default()
            },
            Ok(false) => WriteResponse::not_found(&p.0.title),
            Err(error) => {
                warn!(tool = "delete_note", %error, "delete failed");
                WriteResponse::failed(error.to_string())
            }
        };
        info!(tool = "delete_note", success = response.success, "ok");
        Ok(Json(response))
    }

    #[tool(
        description = "Permanently delete a top-level folder and every note inside it, \
                       matched by exact name. Cannot be undone. Returns success=false \
                       when no folder with that name is found."
    )]
    pub fn delete_folder(
        &self,
        p: Parameters<FolderNameRequest>,
    ) -> Result<Json<FolderWriteResponse>, String> {
        let response = match self.app.delete_folder(&p.0.name) {
            Ok(true) => FolderWriteResponse {
                success: true,
                folder: None,
                error: None,
            },
            Ok(false) => FolderWriteResponse {
                success: false,
                folder: None,
                error: Some(format!("No folder named {:?} was found", p.0.name)),
            },
            Err(error) => {
                warn!(tool = "delete_folder", %error, "delete failed");
                FolderWriteResponse {
                    success: false,
                    folder: None,
                    error: Some(error.to_string()),
                }
            }
        };
        info!(tool = "delete_folder", success = response.success, "ok");
        Ok(Json(response))
    }
}
