use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, schemars, tool, tool_router};
use tracing::{debug, info, warn};

use crate::notes::{
    AccountInfo, AttachmentInfo, FolderInfo, NoteInfo, create_note, delete_note,
    get_all_attachments, get_all_notes, get_note_attachments_by_title, get_note_by_title,
    get_notes_in_account, get_notes_in_folder, get_subfolders, list_accounts, list_folders,
    list_notes, update_note,
};

#[derive(Clone)]
pub struct AppleNotesMCP;

// ─── Request types ────────────────────────────────────────────────────────────

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct EmptyRequest {}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct TitleRequest {
    /// Title of the note.
    title: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct FolderRequest {
    /// Name of the folder.
    folder: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct AccountRequest {
    /// Name of the account (e.g. "iCloud" or "On My Mac").
    account: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct CreateNoteRequest {
    /// Title of the new note.
    title: String,
    /// HTML body of the new note.
    content: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct UpdateNoteRequest {
    /// Current title of the note to update.
    title: String,
    /// New title (omit to keep unchanged).
    new_title: Option<String>,
    /// New HTML body (omit to keep unchanged).
    new_content: Option<String>,
}

// ─── Response types ───────────────────────────────────────────────────────────
// MCP requires the root output schema to be an `object`, so every array or
// optional value must be wrapped in a named struct.

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct NoteTitlesResponse {
    titles: Vec<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct NotesResponse {
    notes: Vec<NoteInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct NoteResponse {
    /// `null` when no note with the requested title was found.
    note: Option<NoteInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct FoldersResponse {
    folders: Vec<FolderInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct AccountsResponse {
    accounts: Vec<AccountInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct AttachmentsResponse {
    attachments: Vec<AttachmentInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub(crate) struct WriteResponse {
    /// `true` if the note was found and the operation applied.
    success: bool,
}

// ─── Tool implementations ─────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl AppleNotesMCP {
    // ── Read: notes ───────────────────────────────────────────────────────────

    #[tool(description = "Return the titles of all notes (fast — does not fetch body content).")]
    pub fn list_notes(&self, _p: Parameters<EmptyRequest>) -> Json<NoteTitlesResponse> {
        debug!(tool = "list_notes", "called");
        let titles = list_notes()
            .inspect_err(|e| warn!(error = %e, "list_notes failed"))
            .unwrap_or_default();
        info!(tool = "list_notes", count = titles.len(), "ok");
        Json(NoteTitlesResponse { titles })
    }

    #[tool(
        description = "Return full metadata and content (HTML + plaintext) for every note, \
                          including folder, account, shared status and password-protection flag."
    )]
    pub fn get_all_notes(&self, _p: Parameters<EmptyRequest>) -> Json<NotesResponse> {
        debug!(tool = "get_all_notes", "called");
        let notes = get_all_notes()
            .inspect_err(|e| warn!(error = %e, "get_all_notes failed"))
            .unwrap_or_default();
        info!(tool = "get_all_notes", count = notes.len(), "ok");
        Json(NotesResponse { notes })
    }

    #[tool(
        description = "Return full metadata and content for a single note looked up by title. \
                          note is null when no note with that title exists."
    )]
    pub fn get_note(&self, p: Parameters<TitleRequest>) -> Json<NoteResponse> {
        debug!(tool = "get_note", "called");
        let note = get_note_by_title(&p.0.title)
            .inspect_err(|e| warn!(error = %e, "get_note failed"))
            .ok()
            .flatten();
        info!(tool = "get_note", found = note.is_some(), "ok");
        Json(NoteResponse { note })
    }

    #[tool(description = "Return all notes inside a specific folder (matched by name).")]
    pub fn get_notes_in_folder(&self, p: Parameters<FolderRequest>) -> Json<NotesResponse> {
        debug!(tool = "get_notes_in_folder", "called");
        let notes = get_notes_in_folder(&p.0.folder)
            .inspect_err(|e| warn!(error = %e, "get_notes_in_folder failed"))
            .unwrap_or_default();
        info!(tool = "get_notes_in_folder", count = notes.len(), "ok");
        Json(NotesResponse { notes })
    }

    #[tool(description = "Return all notes belonging to a specific account \
                          (e.g. \"iCloud\" or \"On My Mac\").")]
    pub fn get_notes_in_account(&self, p: Parameters<AccountRequest>) -> Json<NotesResponse> {
        debug!(tool = "get_notes_in_account", "called");
        let notes = get_notes_in_account(&p.0.account)
            .inspect_err(|e| warn!(error = %e, "get_notes_in_account failed"))
            .unwrap_or_default();
        info!(tool = "get_notes_in_account", count = notes.len(), "ok");
        Json(NotesResponse { notes })
    }

    // ── Read: folders & accounts ──────────────────────────────────────────────

    #[tool(
        description = "Return all folders across all accounts, including subfolders, \
                          with their account and parent-folder context."
    )]
    pub fn list_folders(&self, _p: Parameters<EmptyRequest>) -> Json<FoldersResponse> {
        debug!(tool = "list_folders", "called");
        let folders = list_folders()
            .inspect_err(|e| warn!(error = %e, "list_folders failed"))
            .unwrap_or_default();
        info!(tool = "list_folders", count = folders.len(), "ok");
        Json(FoldersResponse { folders })
    }

    #[tool(
        description = "Return all subfolders of a specific folder (matched by name), \
                          including nested subfolders."
    )]
    pub fn get_subfolders(&self, p: Parameters<FolderRequest>) -> Json<FoldersResponse> {
        debug!(tool = "get_subfolders", "called");
        let folders = get_subfolders(&p.0.folder)
            .inspect_err(|e| warn!(error = %e, "get_subfolders failed"))
            .unwrap_or_default();
        info!(tool = "get_subfolders", count = folders.len(), "ok");
        Json(FoldersResponse { folders })
    }

    #[tool(description = "Return all accounts configured in Apple Notes \
                          (iCloud, On My Mac, Exchange, etc.).")]
    pub fn list_accounts(&self, _p: Parameters<EmptyRequest>) -> Json<AccountsResponse> {
        debug!(tool = "list_accounts", "called");
        let accounts = list_accounts()
            .inspect_err(|e| warn!(error = %e, "list_accounts failed"))
            .unwrap_or_default();
        info!(tool = "list_accounts", count = accounts.len(), "ok");
        Json(AccountsResponse { accounts })
    }

    // ── Read: attachments ─────────────────────────────────────────────────────

    #[tool(description = "Return all attachments embedded in a specific note (matched by title).")]
    pub fn get_note_attachments(&self, p: Parameters<TitleRequest>) -> Json<AttachmentsResponse> {
        debug!(tool = "get_note_attachments", "called");
        let attachments = get_note_attachments_by_title(&p.0.title)
            .inspect_err(|e| warn!(error = %e, "get_note_attachments failed"))
            .unwrap_or_default();
        info!(
            tool = "get_note_attachments",
            count = attachments.len(),
            "ok"
        );
        Json(AttachmentsResponse { attachments })
    }

    #[tool(description = "Return every attachment from every note across all accounts.")]
    pub fn get_all_attachments(&self, _p: Parameters<EmptyRequest>) -> Json<AttachmentsResponse> {
        debug!(tool = "get_all_attachments", "called");
        let attachments = get_all_attachments()
            .inspect_err(|e| warn!(error = %e, "get_all_attachments failed"))
            .unwrap_or_default();
        info!(
            tool = "get_all_attachments",
            count = attachments.len(),
            "ok"
        );
        Json(AttachmentsResponse { attachments })
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    #[tool(description = "Create a new note with the given title and HTML body \
                          in the default Notes folder.")]
    pub fn create_note(&self, p: Parameters<CreateNoteRequest>) -> Json<WriteResponse> {
        debug!(tool = "create_note", "called");
        let success = create_note(&p.0.title, &p.0.content)
            .inspect_err(|e| warn!(error = %e, "create_note failed"))
            .is_ok();
        info!(tool = "create_note", success, "ok");
        Json(WriteResponse { success })
    }

    #[tool(description = "Update the title and/or body of an existing note. \
                          Omit new_title or new_content to leave that field unchanged.")]
    pub fn update_note(&self, p: Parameters<UpdateNoteRequest>) -> Json<WriteResponse> {
        debug!(
            tool = "update_note",
            new_title = p.0.new_title.as_deref().unwrap_or("<unchanged>"),
            "called"
        );
        let success = update_note(
            &p.0.title,
            p.0.new_title.as_deref(),
            p.0.new_content.as_deref(),
        )
        .inspect_err(|e| warn!(error = %e, "update_note failed"))
        .unwrap_or(false);
        info!(tool = "update_note", success, "ok");
        Json(WriteResponse { success })
    }

    #[tool(description = "Permanently delete a note by title. \
                          Returns success=false when no note with that title exists.")]
    pub fn delete_note(&self, p: Parameters<TitleRequest>) -> Json<WriteResponse> {
        debug!(tool = "delete_note", "called");
        let success = delete_note(&p.0.title)
            .inspect_err(|e| warn!(error = %e, "delete_note failed"))
            .unwrap_or(false);
        info!(tool = "delete_note", success, "ok");
        Json(WriteResponse { success })
    }
}
