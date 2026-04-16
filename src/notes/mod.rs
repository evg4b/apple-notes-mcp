//! Apple Notes CRUD via macOS ScriptingBridge.framework.
//!
//! All public functions are synchronous blocking calls; they send Apple Events
//! to Notes.app in-process through the Scripting Bridge — no subprocess is spawned.

mod api;
mod bridge;
mod types;

pub use api::{
    create_note, delete_note, get_all_attachments, get_all_notes, get_note_attachments_by_title,
    get_note_by_title, get_notes_in_account, get_notes_in_folder, get_subfolders, list_accounts,
    list_folders, list_notes, update_note,
};
pub use types::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo};
