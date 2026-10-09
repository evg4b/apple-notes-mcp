use crate::notes::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo, NotePage, PartialNoteInfo};
use rmcp::schemars;
use rmcp::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Notes returned when a bulk read is called without an explicit `limit`.
///
/// Every bulk read is capped: an uncapped one returns every body in the library
/// and can exhaust a client's context in a single call. Responses say when the
/// cap cut something off.
pub(crate) const DEFAULT_NOTE_LIMIT: usize = 50;

// `BodyFormat` and `BodyOptions` are inlined into five tool schemas, so their
// doc comments are paid for five times over. The prose lives on the `format`
// field, which is the one a client actually reads.
#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BodyFormat {
    #[default]
    Text,
    Html,
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
pub(crate) struct BodyOptions {
    /// Body rendering: "text" (default, far smaller) or "html".
    pub format: Option<BodyFormat>,
    /// Maximum notes to return. Defaults to 50.
    pub limit: Option<usize>,
}

impl BodyOptions {
    pub fn format(&self) -> BodyFormat {
        self.format.unwrap_or_default()
    }

    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(DEFAULT_NOTE_LIMIT)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct EmptyRequest {}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct TitleRequest {
    /// Exact title of the note.
    pub title: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct GetNoteRequest {
    /// Exact title of the note.
    pub title: String,
    /// Body rendering: "text" (default, far smaller) or "html".
    pub format: Option<BodyFormat>,
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
pub(crate) struct BulkNotesRequest {
    #[serde(flatten)]
    pub body: BodyOptions,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct FolderRequest {
    /// Exact folder name.
    pub folder: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct FolderNotesRequest {
    /// Exact folder name.
    pub folder: String,
    #[serde(flatten)]
    pub body: BodyOptions,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct AccountNotesRequest {
    /// Account name, e.g. "iCloud" or "On My Mac".
    pub account: String,
    #[serde(flatten)]
    pub body: BodyOptions,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct SearchRequest {
    /// Case-insensitive substring to look for.
    pub query: String,
    /// Search bodies as well as titles. Defaults to true.
    pub in_body: Option<bool>,
    #[serde(flatten)]
    pub body: BodyOptions,
}

impl SearchRequest {
    pub fn in_body(&self) -> bool {
        self.in_body.unwrap_or(true)
    }
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct CreateNoteRequest {
    /// Title for the new note.
    pub title: String,
    /// HTML body.
    pub content: String,
    /// Destination folder. Defaults to the Notes default folder.
    pub folder: Option<String>,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct UpdateNoteRequest {
    /// Current exact title.
    pub title: String,
    /// New title. Omit to keep.
    pub new_title: Option<String>,
    /// New HTML body, replacing the old one. Omit to keep.
    pub new_content: Option<String>,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct AppendNoteRequest {
    /// Exact title of the note.
    pub title: String,
    /// HTML to add at the end.
    pub content: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct MoveNoteRequest {
    /// Exact title of the note.
    pub title: String,
    /// Destination folder name.
    pub folder: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct CreateFolderRequest {
    /// Name for the new folder.
    pub name: String,
    /// Account to create it in. Defaults to Notes' default account.
    pub account: Option<String>,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct FolderNameRequest {
    /// Exact folder name.
    pub name: String,
}

#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct NoteTitlesResponse {
    pub titles: Vec<String>,
}

#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct NotesResponse {
    pub notes: Vec<NoteInfo>,
    /// Present when `limit` cut the result short. Raise `limit`, or narrow the
    /// query, to see the rest.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

impl NotesResponse {
    pub fn new(page: NotePage, format: BodyFormat) -> Self {
        let mut notes = page.notes;
        if format == BodyFormat::Text {
            for note in &mut notes {
                note.body = crate::notes::to_plain_text(&note.body);
            }
        }
        Self {
            notes,
            truncated: page.truncated,
        }
    }
}

#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct NoteResponse {
    /// Null when no note has that title.
    pub note: Option<NoteInfo>,
}

impl NoteResponse {
    pub fn new(note: Option<NoteInfo>, format: BodyFormat) -> Self {
        let note = note.map(|mut note| {
            if format == BodyFormat::Text {
                note.body = crate::notes::to_plain_text(&note.body);
            }
            note
        });
        Self { note }
    }
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
    /// True when the operation applied.
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<PartialNoteInfo>,
    /// Why it failed. Absent on success.
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
    /// Why it failed. Absent on success.
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
    fn search_without_options_uses_defaults() {
        let request: SearchRequest = parse(json!({ "query": "budget" }));
        assert!(request.in_body());
        assert_eq!(request.body.limit(), DEFAULT_NOTE_LIMIT);
        assert_eq!(request.body.format(), BodyFormat::Text);
    }

    #[test]
    fn search_honours_explicit_options() {
        let request: SearchRequest =
            parse(json!({ "query": "budget", "in_body": false, "limit": 5, "format": "html" }));
        assert!(!request.in_body());
        assert_eq!(request.body.limit(), 5);
        assert_eq!(request.body.format(), BodyFormat::Html);
    }

    #[test]
    fn zero_limit_is_accepted() {
        let request: SearchRequest = parse(json!({ "query": "x", "limit": 0 }));
        assert_eq!(request.body.limit(), 0);
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
        assert!(
            value.get("error").is_none(),
            "a successful write carried an error: {value}"
        );
    }

    #[test]
    fn not_found_write_names_the_missing_note() {
        let value = to_value(WriteResponse::not_found("Shopping list")).unwrap();
        assert_eq!(value["success"], json!(false));
        assert!(
            value.get("note").is_none(),
            "a failed write carried a note: {value}"
        );
        let error = value["error"].as_str().unwrap();
        assert!(
            error.contains("Shopping list"),
            "error does not name the missing note: {error}"
        );
    }

    #[test]
    fn failed_write_carries_the_reason() {
        let value = to_value(WriteResponse::failed("Notes refused".into())).unwrap();
        assert_eq!(value["success"], json!(false));
        assert_eq!(value["error"], json!("Notes refused"));
    }

    #[test]
    fn default_write_response_is_a_failure() {
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
    fn empty_collections_serialize_as_arrays() {
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

    /// A note body as Notes stores it: one `<div>` per line, inline styles,
    /// entity-escaped punctuation.
    const REALISTIC_BODY: &str = concat!(
        r#"<div><h1>Q3 planning</h1></div><div><br></div>"#,
        r#"<div><span style="font-family: Helvetica; font-size: 14px">"#,
        r#"Owner &amp; reviewer: Sam</span></div><div><br></div>"#,
        r#"<div><ul class="Apple-dash-list"><li>Draft the brief</li>"#,
        r#"<li>Book the review</li><li>Send it round</li></ul></div>"#,
    );

    /// One note of `REALISTIC_BODY`, rendered the way a tool call would.
    fn rendered(format: BodyFormat) -> NotesResponse {
        let page = NotePage {
            notes: vec![note_with_body(REALISTIC_BODY)],
            truncated: false,
        };
        NotesResponse::new(page, format)
    }

    fn rendered_json(format: BodyFormat) -> String {
        to_value(rendered(format)).unwrap().to_string()
    }

    #[test]
    fn text_format_shrinks_the_payload() {
        let as_html = rendered_json(BodyFormat::Html);
        let as_text = rendered_json(BodyFormat::Text);

        let body_text = crate::notes::to_plain_text(REALISTIC_BODY).len();
        assert!(
            body_text * 2 < REALISTIC_BODY.len(),
            "body went from {} to {body_text} chars; expected at least half off",
            REALISTIC_BODY.len()
        );
        assert!(
            as_text.len() * 10 < as_html.len() * 7,
            "payload went from {} to {} bytes; expected at least 30% off",
            as_html.len(),
            as_text.len()
        );
        assert!(as_text.contains("Owner & reviewer: Sam"));
        assert!(!as_text.contains("<div>"));
    }

    #[test]
    fn html_format_leaves_the_body_untouched() {
        assert_eq!(rendered(BodyFormat::Html).notes[0].body, REALISTIC_BODY);
    }

    #[test]
    fn truncated_page_sets_the_flag() {
        let page = NotePage {
            notes: vec![note_with_body("a"), note_with_body("b")],
            truncated: true,
        };
        let value = to_value(NotesResponse::new(page, BodyFormat::Text)).unwrap();
        assert_eq!(value["truncated"], json!(true));
    }

    #[test]
    fn untruncated_page_omits_the_flag() {
        let value = to_value(NotesResponse::default()).unwrap();
        assert!(
            value.get("truncated").is_none(),
            "truncated=false was serialized: {value}"
        );
    }

    #[test]
    fn false_note_flags_are_omitted() {
        let value = to_value(note_with_body("x")).unwrap();
        assert!(value.get("shared").is_none(), "shared=false sent: {value}");
        assert!(
            value.get("password_protected").is_none(),
            "password_protected=false sent: {value}"
        );
    }

    fn note_with_body(body: &str) -> NoteInfo {
        NoteInfo {
            id: "x-coredata://1".into(),
            title: "Q3 planning".into(),
            body: body.into(),
            creation_date: "2024-01-15 09:30:00 +0000".into(),
            modification_date: "2024-01-15 09:30:00 +0000".into(),
            folder: "Work".into(),
            account: "iCloud".into(),
            shared: false,
            password_protected: false,
        }
    }
}
