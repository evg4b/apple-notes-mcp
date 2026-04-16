use rmcp::schemars;

/// Account (e.g. "iCloud", "On My Mac").
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct AccountInfo {
    /// Unique scripting ID of the account.
    pub id: String,
    pub name: String,
}

/// A Notes folder (may be nested inside another folder or directly under an account).
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct FolderInfo {
    /// Unique scripting ID of the folder.
    pub id: String,
    pub name: String,
    /// Name of the containing account (top-level container).
    pub account: String,
    /// Name of the immediate parent (account name or another folder name).
    pub parent: String,
}

/// Full metadata and content of a single note.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct NoteInfo {
    /// Unique scripting ID of the note.
    pub id: String,
    pub title: String,
    /// HTML body of the note.
    pub body: String,
    pub creation_date: String,
    pub modification_date: String,
    /// Containing folder name.
    pub folder: String,
    /// Containing account name (top-level).
    pub account: String,
    pub shared: bool,
    pub password_protected: bool,
}

/// A file attachment embedded in a note.
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct AttachmentInfo {
    /// Unique scripting ID of the attachment.
    pub id: String,
    pub name: String,
    pub creation_date: String,
    pub modification_date: String,
    /// File URL of the attachment (may be empty for inline attachments).
    pub url: String,
    /// Title of the note that contains this attachment.
    pub note_title: String,
}
