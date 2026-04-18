use crate::notes::{AccountInfo, FolderInfo, NoteInfo};
use rmcp::schemars;

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct EmptyRequest {}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct TitleRequest {
    /// Title of the note.
    pub title: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct FolderRequest {
    /// Name of the folder.
    pub folder: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct AccountRequest {
    /// Name of the account (e.g. "iCloud" or "On My Mac").
    pub account: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct CreateNoteRequest {
    /// Title of the new note.
    pub title: String,
    /// HTML body of the new note.
    pub content: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct UpdateNoteRequest {
    /// Current title of the note to update.
    pub title: String,
    /// New title (omit to keep unchanged).
    pub new_title: Option<String>,
    /// New HTML body (omit to keep unchanged).
    pub new_content: Option<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct NoteTitlesResponse {
    pub titles: Vec<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct NotesResponse {
    pub notes: Vec<NoteInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct NoteResponse {
    /// `null` when no note with the requested title was found.
    pub note: Option<NoteInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct FoldersResponse {
    pub folders: Vec<FolderInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct AccountsResponse {
    pub accounts: Vec<AccountInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct WriteResponse {
    /// `true` if the note was found and the operation applied.
    pub success: bool,
}
