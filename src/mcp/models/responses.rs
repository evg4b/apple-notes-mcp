use super::requests::BodyFormat;
use crate::notes::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo, NotePage, PartialNoteInfo};
use rmcp::schemars;
use rmcp::serde::Serialize;
use schemars::JsonSchema;

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
    pub fn new(mut page: NotePage, format: BodyFormat) -> Self {
        page.notes.iter_mut().for_each(|note| format.render(note));
        Self {
            notes: page.notes,
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
    pub fn new(mut note: Option<NoteInfo>, format: BodyFormat) -> Self {
        if let Some(note) = &mut note {
            format.render(note);
        }
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
    pub fn done(note: Option<PartialNoteInfo>) -> Self {
        Self {
            success: true,
            note,
            error: None,
        }
    }

    pub fn not_found(title: &str) -> Self {
        Self::failed(format!("No note titled {title:?} was found"))
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

impl FolderWriteResponse {
    pub fn done(folder: Option<FolderInfo>) -> Self {
        Self {
            success: true,
            folder,
            error: None,
        }
    }

    pub fn not_found(name: &str) -> Self {
        Self::failed(format!("No folder named {name:?} was found"))
    }

    pub fn failed(error: String) -> Self {
        Self {
            success: false,
            folder: None,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::serde_json::{json, to_value};

    #[test]
    fn successful_write_omits_the_error_field() {
        let note = PartialNoteInfo {
            id: "x-coredata://1".into(),
            title: Some("t".into()),
            body: None,
            creation_date: None,
            modification_date: None,
        };
        let value = to_value(WriteResponse::done(Some(note))).unwrap();
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
        let value = to_value(FolderWriteResponse::done(None)).unwrap();
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
