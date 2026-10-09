use rmcp::schemars;
use rmcp::schemars::JsonSchema;
use rmcp::serde::Serialize;
use std::sync::Arc;

// Doc comments here are copied into every tool's `outputSchema`; keep them
// short and explain in docs/tools.md. Names repeated across rows are
// `Arc<str>`, shared rather than copied.

/// An account, e.g. "iCloud" or "On My Mac".
#[derive(Debug, Serialize, JsonSchema)]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
}

/// A folder, in an account or in another folder.
#[derive(Debug, Serialize, JsonSchema)]
pub struct FolderInfo {
    pub id: String,
    pub name: Arc<str>,
    pub account: Arc<str>,
    /// Account name for top-level folders, else the parent folder's name.
    pub parent: Arc<str>,
}

/// A note with its body.
#[derive(Debug, Serialize, JsonSchema)]
pub struct NoteInfo {
    pub id: String,
    pub title: String,
    /// Plain text, or HTML with `format: "html"`.
    pub body: String,
    pub creation_date: String,
    pub modification_date: String,
    pub folder: Arc<str>,
    pub account: Arc<str>,
    /// Omitted when false.
    #[serde(skip_serializing_if = "is_false")]
    pub shared: bool,
    /// Omitted when false. Protected notes report an empty body.
    #[serde(skip_serializing_if = "is_false")]
    pub password_protected: bool,
}

/// What a write knows about the note, so it need not be re-read. Only `id` is
/// always set.
#[derive(Debug, Serialize, JsonSchema)]
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
#[derive(Debug, Serialize, JsonSchema)]
pub struct AttachmentInfo {
    pub id: String,
    pub name: String,
    pub creation_date: String,
    pub modification_date: String,
    /// Empty for inline attachments.
    pub url: String,
    pub note_title: Arc<str>,
}

/// A capped batch of notes and whether the cap cut any off.
#[derive(Default)]
pub struct NotePage {
    pub notes: Vec<NoteInfo>,
    pub truncated: bool,
}

impl NotePage {
    /// `fill` gets a ceiling of `limit + 1`, so a full page differs from an
    /// overflowing one.
    pub(super) fn collect(limit: usize, fill: impl FnOnce(usize, &mut Vec<NoteInfo>)) -> Self {
        let mut notes = Vec::new();
        fill(limit.saturating_add(1), &mut notes);
        let truncated = notes.len() > limit;
        notes.truncate(limit);
        Self { notes, truncated }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(title: &str) -> NoteInfo {
        NoteInfo {
            id: String::new(),
            title: title.into(),
            body: String::new(),
            creation_date: String::new(),
            modification_date: String::new(),
            folder: "".into(),
            account: "".into(),
            shared: false,
            password_protected: false,
        }
    }

    fn page_of(limit: usize, available: usize) -> NotePage {
        NotePage::collect(limit, |ceiling, out| {
            out.extend((0..available.min(ceiling)).map(|i| note(&i.to_string())));
        })
    }

    #[test]
    fn collect_offers_one_slot_past_the_limit() {
        NotePage::collect(3, |ceiling, _| assert_eq!(ceiling, 4));
        NotePage::collect(usize::MAX, |ceiling, _| assert_eq!(ceiling, usize::MAX));
    }

    #[test]
    fn collect_under_the_limit_is_not_truncated() {
        let page = page_of(5, 3);
        assert_eq!(page.notes.len(), 3);
        assert!(!page.truncated);
    }

    #[test]
    fn collect_exactly_at_the_limit_is_not_truncated() {
        let page = page_of(3, 3);
        assert_eq!(page.notes.len(), 3);
        assert!(!page.truncated);
    }

    #[test]
    fn collect_over_the_limit_is_cut_and_flagged() {
        let page = page_of(3, 10);
        let titles: Vec<_> = page.notes.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, ["0", "1", "2"]);
        assert!(page.truncated);
    }
}
