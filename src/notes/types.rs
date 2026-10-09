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
    /// `fill` is given a ceiling one past `limit`, so a full page can be told
    /// apart from an overflowing one.
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
            folder: String::new(),
            account: String::new(),
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
