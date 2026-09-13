use crate::notes::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo, PartialNoteInfo};
use rmcp::schemars;
use rmcp::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Result cap applied when `search_notes` is called without an explicit `limit`.
pub(crate) const DEFAULT_SEARCH_LIMIT: usize = 50;

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct EmptyRequest {}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct TitleRequest {
    /// Title of the note.
    pub title: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct FolderRequest {
    /// Name of the folder.
    pub folder: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct AccountRequest {
    /// Name of the account (e.g. "iCloud" or "On My Mac").
    pub account: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct SearchRequest {
    /// Case-insensitive substring to look for.
    pub query: String,
    /// Also search note bodies, not just titles. Defaults to `true`.
    pub in_body: Option<bool>,
    /// Maximum number of notes to return. Defaults to 50.
    pub limit: Option<usize>,
}

impl SearchRequest {
    pub fn in_body(&self) -> bool {
        self.in_body.unwrap_or(true)
    }

    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(DEFAULT_SEARCH_LIMIT)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct CreateNoteRequest {
    /// Title of the new note.
    pub title: String,
    /// HTML body of the new note.
    pub content: String,
    /// Folder to create the note in. Defaults to the Notes default folder.
    pub folder: Option<String>,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct UpdateNoteRequest {
    /// Current title of the note to update.
    pub title: String,
    /// New title (omit to keep unchanged).
    pub new_title: Option<String>,
    /// New HTML body (omit to keep unchanged).
    pub new_content: Option<String>,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct AppendNoteRequest {
    /// Title of the note to append to.
    pub title: String,
    /// HTML appended to the end of the existing body.
    pub content: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct MoveNoteRequest {
    /// Title of the note to move.
    pub title: String,
    /// Name of the destination folder.
    pub folder: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct CreateFolderRequest {
    /// Name of the new folder.
    pub name: String,
    /// Account to create the folder in. Defaults to the first account.
    pub account: Option<String>,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct FolderNameRequest {
    /// Name of the folder.
    pub name: String,
}

#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct NoteTitlesResponse {
    pub titles: Vec<String>,
}

#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct NotesResponse {
    pub notes: Vec<NoteInfo>,
}

#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct NoteResponse {
    /// `null` when no note with the requested title was found.
    pub note: Option<NoteInfo>,
}

#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct FoldersResponse {
    pub folders: Vec<FolderInfo>,
}

#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct AccountsResponse {
    pub accounts: Vec<AccountInfo>,
}

#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct AttachmentsResponse {
    pub attachments: Vec<AttachmentInfo>,
}

/// Outcome of a note write or delete.
#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct WriteResponse {
    /// `true` if the note was found and the operation applied.
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<PartialNoteInfo>,
    /// Why the operation failed. Absent on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WriteResponse {
    pub fn found(note: PartialNoteInfo) -> Self {
        Self {
            success: true,
            note: Some(note),
            error: None,
        }
    }

    pub fn not_found(what: &str) -> Self {
        Self {
            success: false,
            note: None,
            error: Some(format!("No note titled {what:?} was found")),
        }
    }

    pub fn failed(error: String) -> Self {
        Self {
            success: false,
            note: None,
            error: Some(error),
        }
    }
}

/// Outcome of a folder create or delete.
#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct FolderWriteResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<FolderInfo>,
    /// Why the operation failed. Absent on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::serde_json::{Value, from_value, json, to_value};

    fn parse<T: for<'de> Deserialize<'de>>(value: Value) -> T {
        from_value(value).expect("request should deserialize")
    }

    #[test]
    fn search_defaults_to_body_search_and_the_standard_limit() {
        let request: SearchRequest = parse(json!({ "query": "budget" }));
        assert!(request.in_body());
        assert_eq!(request.limit(), DEFAULT_SEARCH_LIMIT);
    }

    #[test]
    fn search_honours_explicit_options() {
        let request: SearchRequest =
            parse(json!({ "query": "budget", "in_body": false, "limit": 5 }));
        assert!(!request.in_body());
        assert_eq!(request.limit(), 5);
    }

    #[test]
    fn search_accepts_a_zero_limit() {
        let request: SearchRequest = parse(json!({ "query": "x", "limit": 0 }));
        assert_eq!(request.limit(), 0);
    }

    #[test]
    fn create_note_folder_is_optional() {
        let request: CreateNoteRequest = parse(json!({ "title": "t", "content": "c" }));
        assert_eq!(request.folder, None);

        let request: CreateNoteRequest =
            parse(json!({ "title": "t", "content": "c", "folder": "Work" }));
        assert_eq!(request.folder.as_deref(), Some("Work"));
    }

    #[test]
    fn update_note_fields_are_independently_optional() {
        let request: UpdateNoteRequest = parse(json!({ "title": "t", "new_title": "u" }));
        assert_eq!(request.new_title.as_deref(), Some("u"));
        assert_eq!(request.new_content, None);
    }

    #[test]
    fn successful_write_omits_the_error_field() {
        let note = PartialNoteInfo {
            id: "x-coredata://1".into(),
            title: Some("t".into()),
            body: None,
            creation_date: None,
            modification_date: None,
        };
        let value = to_value(WriteResponse::found(note)).unwrap();
        assert_eq!(value["success"], json!(true));
        assert_eq!(value["note"]["id"], json!("x-coredata://1"));
        assert!(value.get("error").is_none(), "error leaked into success");
    }

    #[test]
    fn not_found_write_names_the_missing_note() {
        let value = to_value(WriteResponse::not_found("Shopping list")).unwrap();
        assert_eq!(value["success"], json!(false));
        assert!(value.get("note").is_none(), "note leaked into failure");
        let error = value["error"].as_str().unwrap();
        assert!(error.contains("Shopping list"), "unhelpful error: {error}");
    }

    #[test]
    fn failed_write_carries_the_reason() {
        let value = to_value(WriteResponse::failed("Notes refused".into())).unwrap();
        assert_eq!(value["success"], json!(false));
        assert_eq!(value["error"], json!("Notes refused"));
    }

    #[test]
    fn default_write_response_is_an_unexplained_failure() {
        let value = to_value(WriteResponse::default()).unwrap();
        assert_eq!(value["success"], json!(false));
        assert!(value.get("note").is_none());
        assert!(value.get("error").is_none());
    }

    #[test]
    fn folder_write_response_omits_empty_fields() {
        let value = to_value(FolderWriteResponse {
            success: true,
            folder: None,
            error: None,
        })
        .unwrap();
        assert_eq!(value, json!({ "success": true }));
    }

    #[test]
    fn empty_collection_responses_serialize_as_empty_arrays() {
        assert_eq!(
            to_value(NotesResponse::default()).unwrap(),
            json!({ "notes": [] })
        );
        assert_eq!(
            to_value(AttachmentsResponse::default()).unwrap(),
            json!({ "attachments": [] })
        );
        assert_eq!(
            to_value(NoteResponse::default()).unwrap(),
            json!({ "note": null })
        );
    }
}
