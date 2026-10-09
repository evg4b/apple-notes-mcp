use rmcp::schemars;
use rmcp::schemars::JsonSchema;
use rmcp::serde::{Deserialize, Serialize};

// Every doc comment here is copied into the `outputSchema` of each tool that
// returns the type, so it is paid for on every session. Explanations belong in
// docs/tools.md.

/// An account, e.g. "iCloud" or "On My Mac".
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
}

/// A folder, nested either in another folder or directly in an account.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FolderInfo {
    pub id: String,
    pub name: String,
    pub account: String,
    /// Immediate container: an account name for top-level folders, else a folder name.
    pub parent: String,
}

/// A note with its full body.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NoteInfo {
    pub id: String,
    pub title: String,
    /// Plain text, or HTML when the request asked for `format: "html"`.
    pub body: String,
    pub creation_date: String,
    pub modification_date: String,
    pub folder: String,
    pub account: String,
    /// Omitted when false.
    #[serde(default, skip_serializing_if = "is_false")]
    pub shared: bool,
    /// Omitted when false. A protected note always reports an empty body.
    #[serde(default, skip_serializing_if = "is_false")]
    pub password_protected: bool,
}

/// What a write already knows about the note it touched, returned instead of
/// re-reading it. Every field but `id` varies by operation.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PartialNoteInfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modification_date: Option<String>,
}

/// A file attached to a note.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AttachmentInfo {
    pub id: String,
    pub name: String,
    pub creation_date: String,
    pub modification_date: String,
    /// Empty for inline attachments.
    pub url: String,
    pub note_title: String,
}

/// A bounded batch of notes, plus whether the bound cut anything off.
#[derive(Default)]
pub struct NotePage {
    pub notes: Vec<NoteInfo>,
    pub truncated: bool,
}

impl NotePage {
    /// `notes` was collected up to `limit + 1`, so a full page can be told
    /// apart from an overflowing one.
    pub(super) fn from_overshoot(mut notes: Vec<NoteInfo>, limit: usize) -> Self {
        let truncated = notes.len() > limit;
        notes.truncate(limit);
        Self { notes, truncated }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}
