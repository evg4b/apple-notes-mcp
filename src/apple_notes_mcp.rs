use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router, Json};

use crate::notes::{
    create_note, delete_note, get_all_attachments, get_all_notes, get_note_attachments_by_title,
    get_note_by_title, get_notes_in_account, get_notes_in_folder, get_subfolders, list_accounts,
    list_folders, list_notes, update_note, AccountInfo, AttachmentInfo, FolderInfo, NoteInfo,
};

#[derive(Clone)]
pub struct AppleNotesMCP;

// ─── Request types ────────────────────────────────────────────────────────────

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct EmptyRequest {}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct TitleRequest {
    /// Title of the note.
    title: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct FolderRequest {
    /// Name of the folder.
    folder: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct AccountRequest {
    /// Name of the account (e.g. "iCloud" or "On My Mac").
    account: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct CreateNoteRequest {
    /// Title of the new note.
    title: String,
    /// HTML body of the new note.
    content: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct UpdateNoteRequest {
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
struct NoteTitlesResponse {
    titles: Vec<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct NotesResponse {
    notes: Vec<NoteInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct NoteResponse {
    /// `null` when no note with the requested title was found.
    note: Option<NoteInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct FoldersResponse {
    folders: Vec<FolderInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct AccountsResponse {
    accounts: Vec<AccountInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct AttachmentsResponse {
    attachments: Vec<AttachmentInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct WriteResponse {
    /// `true` if the note was found and the operation applied.
    success: bool,
}

// ─── Tool implementations ─────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl AppleNotesMCP {
    // ── Read: notes ───────────────────────────────────────────────────────────

    #[tool(description = "Return the titles of all notes (fast — does not fetch body content).")]
    pub fn list_notes(&self, _p: Parameters<EmptyRequest>) -> Json<NoteTitlesResponse> {
        Json(NoteTitlesResponse {
            titles: list_notes().unwrap_or_default(),
        })
    }

    #[tool(description = "Return full metadata and content (HTML + plaintext) for every note, \
                          including folder, account, shared status and password-protection flag.")]
    pub fn get_all_notes(&self, _p: Parameters<EmptyRequest>) -> Json<NotesResponse> {
        Json(NotesResponse { notes: get_all_notes().unwrap_or_default() })
    }

    #[tool(description = "Return full metadata and content for a single note looked up by title. \
                          note is null when no note with that title exists.")]
    pub fn get_note(&self, p: Parameters<TitleRequest>) -> Json<NoteResponse> {
        Json(NoteResponse { note: get_note_by_title(&p.0.title).unwrap_or(None) })
    }

    #[tool(description = "Return all notes inside a specific folder (matched by name).")]
    pub fn get_notes_in_folder(&self, p: Parameters<FolderRequest>) -> Json<NotesResponse> {
        Json(NotesResponse { notes: get_notes_in_folder(&p.0.folder).unwrap_or_default() })
    }

    #[tool(description = "Return all notes belonging to a specific account \
                          (e.g. \"iCloud\" or \"On My Mac\").")]
    pub fn get_notes_in_account(&self, p: Parameters<AccountRequest>) -> Json<NotesResponse> {
        Json(NotesResponse { notes: get_notes_in_account(&p.0.account).unwrap_or_default() })
    }

    // ── Read: folders & accounts ──────────────────────────────────────────────

    #[tool(description = "Return all folders across all accounts, including subfolders, \
                          with their account and parent-folder context.")]
    pub fn list_folders(&self, _p: Parameters<EmptyRequest>) -> Json<FoldersResponse> {
        Json(FoldersResponse { folders: list_folders().unwrap_or_default() })
    }

    #[tool(description = "Return all subfolders of a specific folder (matched by name), \
                          including nested subfolders.")]
    pub fn get_subfolders(&self, p: Parameters<FolderRequest>) -> Json<FoldersResponse> {
        Json(FoldersResponse { folders: get_subfolders(&p.0.folder).unwrap_or_default() })
    }

    #[tool(description = "Return all accounts configured in Apple Notes \
                          (iCloud, On My Mac, Exchange, etc.).")]
    pub fn list_accounts(&self, _p: Parameters<EmptyRequest>) -> Json<AccountsResponse> {
        Json(AccountsResponse { accounts: list_accounts().unwrap_or_default() })
    }

    // ── Read: attachments ─────────────────────────────────────────────────────

    #[tool(description = "Return all attachments embedded in a specific note (matched by title).")]
    pub fn get_note_attachments(&self, p: Parameters<TitleRequest>) -> Json<AttachmentsResponse> {
        Json(AttachmentsResponse {
            attachments: get_note_attachments_by_title(&p.0.title).unwrap_or_default(),
        })
    }

    #[tool(description = "Return every attachment from every note across all accounts.")]
    pub fn get_all_attachments(&self, _p: Parameters<EmptyRequest>) -> Json<AttachmentsResponse> {
        Json(AttachmentsResponse { attachments: get_all_attachments().unwrap_or_default() })
    }

    // ── Write ─────────────────────────────────────────────────────────────────

    #[tool(description = "Create a new note with the given title and HTML body \
                          in the default Notes folder.")]
    pub fn create_note(&self, p: Parameters<CreateNoteRequest>) -> Json<WriteResponse> {
        Json(WriteResponse {
            success: create_note(&p.0.title, &p.0.content).is_ok(),
        })
    }

    #[tool(description = "Update the title and/or body of an existing note. \
                          Omit new_title or new_content to leave that field unchanged.")]
    pub fn update_note(&self, p: Parameters<UpdateNoteRequest>) -> Json<WriteResponse> {
        Json(WriteResponse {
            success: update_note(
                &p.0.title,
                p.0.new_title.as_deref(),
                p.0.new_content.as_deref(),
            )
            .unwrap_or(false),
        })
    }

    #[tool(description = "Permanently delete a note by title. \
                          Returns success=false when no note with that title exists.")]
    pub fn delete_note(&self, p: Parameters<TitleRequest>) -> Json<WriteResponse> {
        Json(WriteResponse {
            success: delete_note(&p.0.title).unwrap_or(false),
        })
    }
}
