use crate::notes::NoteInfo;
use rmcp::schemars;
use rmcp::serde::Deserialize;
use schemars::JsonSchema;

/// Uncapped bulk reads can exhaust a client's context in one call.
pub(crate) const DEFAULT_NOTE_LIMIT: usize = 50;

// Inlined into five tool schemas, so the `format` fields document it instead.
#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BodyFormat {
    #[default]
    Text,
    Html,
}

impl BodyFormat {
    pub(super) fn render(self, note: &mut NoteInfo) {
        if self == Self::Text {
            note.body = crate::notes::to_plain_text(&note.body);
        }
    }
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
pub(crate) struct BodyOptions {
    /// Body format: "text" (default, smaller) or "html".
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
    /// Exact note title.
    pub title: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct GetNoteRequest {
    /// Exact note title.
    pub title: String,
    /// Body format: "text" (default, smaller) or "html".
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
    /// Case-insensitive substring.
    pub query: String,
    /// Also search bodies. Defaults to true.
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
    /// Note title.
    pub title: String,
    /// HTML body.
    pub content: String,
    /// Destination folder. Defaults to Notes' default folder.
    pub folder: Option<String>,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct UpdateNoteRequest {
    /// Current exact title.
    pub title: String,
    /// New title; omit to keep.
    pub new_title: Option<String>,
    /// New HTML body, replacing the old one; omit to keep.
    pub new_content: Option<String>,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct AppendNoteRequest {
    /// Exact note title.
    pub title: String,
    /// HTML to append.
    pub content: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct MoveNoteRequest {
    /// Exact note title.
    pub title: String,
    /// Destination folder name.
    pub folder: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct CreateFolderRequest {
    /// Folder name.
    pub name: String,
    /// Account to create it in. Defaults to Notes' default account.
    pub account: Option<String>,
}

#[derive(Clone, Deserialize, JsonSchema)]
pub(crate) struct FolderNameRequest {
    /// Exact folder name.
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::serde_json::{Value, from_value, json};

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
}
