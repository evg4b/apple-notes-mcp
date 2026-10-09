mod api;
mod bridge;
mod delegate;
mod helpers;
mod html;
mod types;

#[cfg(test)]
mod integration_tests;

pub use api::NotesApp;
pub use html::to_plain_text;
pub use types::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo, NotePage, PartialNoteInfo};
