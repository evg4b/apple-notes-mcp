//! Public API — all functions are synchronous blocking calls that send Apple
//! Events to Notes.app via ScriptingBridge.

use anyhow::{anyhow, Context, Result};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::msg_send;
use objc2_foundation::NSString;

use super::bridge::{
    app_accounts, app_as_any, app_notes, attachment_info, collect_folders,
    collect_notes_in_folder, collect_notes_in_folders, kvc_set, kvc_string, note_attachments,
    note_info, obj_folders, obj_notes, sb_at, sb_count, account_info,
};
use super::types::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo};
use super::bridge::SBApplication;

/// Obtain a live ScriptingBridge proxy to Notes.app.
fn notes_app() -> Result<Retained<SBApplication>> {
    use objc2::ClassType;
    let bundle_id = NSString::from_str("com.apple.Notes");
    let app: Option<Retained<SBApplication>> = unsafe {
        msg_send![SBApplication::class(), applicationWithBundleIdentifier: &*bundle_id]
    };
    app.ok_or_else(|| anyhow!("Cannot connect to Apple Notes via ScriptingBridge"))
}

// ─── Read: accounts ───────────────────────────────────────────────────────────

/// Return all accounts configured in Apple Notes.
pub fn list_accounts() -> Result<Vec<AccountInfo>> {
    let app = notes_app()?;
    unsafe {
        let arr = app_accounts(app_as_any(&app));
        let count = sb_count(&arr);
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            out.push(account_info(&sb_at(&arr, i)));
        }
        Ok(out)
    }
}

// ─── Read: folders ────────────────────────────────────────────────────────────

/// Return all folders across all accounts (including subfolders).
pub fn list_folders() -> Result<Vec<FolderInfo>> {
    let app = notes_app()?;
    unsafe {
        let accounts_arr = app_accounts(app_as_any(&app));
        let account_count = sb_count(&accounts_arr);
        let mut out = Vec::new();
        for i in 0..account_count {
            let account = sb_at(&accounts_arr, i);
            let account_name = kvc_string(&account, "name");
            let folders_arr = obj_folders(&account);
            collect_folders(&folders_arr, &account_name, &account_name, &mut out);
        }
        Ok(out)
    }
}

/// Return all subfolders of a given folder (matched by name), including nested ones.
pub fn get_subfolders(folder_name: &str) -> Result<Vec<FolderInfo>> {
    let app = notes_app()?;
    unsafe {
        let accounts_arr = app_accounts(app_as_any(&app));
        let account_count = sb_count(&accounts_arr);
        for i in 0..account_count {
            let account = sb_at(&accounts_arr, i);
            let account_name = kvc_string(&account, "name");
            let folders_arr = obj_folders(&account);
            let folder_count = sb_count(&folders_arr);
            for j in 0..folder_count {
                let folder = sb_at(&folders_arr, j);
                if kvc_string(&folder, "name") == folder_name {
                    let mut out = Vec::new();
                    let sub_arr = obj_folders(&folder);
                    collect_folders(&sub_arr, &account_name, folder_name, &mut out);
                    return Ok(out);
                }
            }
        }
        Ok(vec![])
    }
}

// ─── Read: notes ──────────────────────────────────────────────────────────────

/// Return just the titles of all notes (fast — avoids fetching body/plaintext).
pub fn list_notes() -> Result<Vec<String>> {
    let app = notes_app()?;
    unsafe {
        let arr = app_notes(app_as_any(&app));
        let count = sb_count(&arr);
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            out.push(kvc_string(&sb_at(&arr, i), "name"));
        }
        Ok(out)
    }
}

/// Return full metadata + content for every note.
/// Iterates accounts → folders → notes to resolve folder/account context.
pub fn get_all_notes() -> Result<Vec<NoteInfo>> {
    let app = notes_app()?;
    unsafe {
        let accounts_arr = app_accounts(app_as_any(&app));
        let account_count = sb_count(&accounts_arr);
        let mut out = Vec::new();
        for i in 0..account_count {
            let account = sb_at(&accounts_arr, i);
            let account_name = kvc_string(&account, "name");
            let folders_arr = obj_folders(&account);
            collect_notes_in_folders(&folders_arr, &account_name, &mut out);
        }
        Ok(out)
    }
}

/// Return the full details of a note by title, or `None` if not found.
pub fn get_note_by_title(title: &str) -> Result<Option<NoteInfo>> {
    let app = notes_app()?;
    unsafe {
        let accounts_arr = app_accounts(app_as_any(&app));
        let account_count = sb_count(&accounts_arr);
        for i in 0..account_count {
            let account = sb_at(&accounts_arr, i);
            let account_name = kvc_string(&account, "name");
            let folders_arr = obj_folders(&account);
            let folder_count = sb_count(&folders_arr);
            for j in 0..folder_count {
                let folder = sb_at(&folders_arr, j);
                let folder_name = kvc_string(&folder, "name");
                let notes_arr = obj_notes(&folder);
                let note_count = sb_count(&notes_arr);
                for k in 0..note_count {
                    let note = sb_at(&notes_arr, k);
                    if kvc_string(&note, "name") == title {
                        return Ok(Some(note_info(&note, &folder_name, &account_name)));
                    }
                }
            }
        }
        Ok(None)
    }
}

/// Return all notes inside a specific folder (matched by folder name).
pub fn get_notes_in_folder(folder_name: &str) -> Result<Vec<NoteInfo>> {
    let app = notes_app()?;
    unsafe {
        let accounts_arr = app_accounts(app_as_any(&app));
        let account_count = sb_count(&accounts_arr);
        for i in 0..account_count {
            let account = sb_at(&accounts_arr, i);
            let account_name = kvc_string(&account, "name");
            let folders_arr = obj_folders(&account);
            let folder_count = sb_count(&folders_arr);
            for j in 0..folder_count {
                let folder = sb_at(&folders_arr, j);
                if kvc_string(&folder, "name") == folder_name {
                    let mut out = Vec::new();
                    collect_notes_in_folder(&folder, folder_name, &account_name, &mut out);
                    return Ok(out);
                }
            }
        }
        Ok(vec![])
    }
}

/// Return all notes inside a specific account (matched by account name).
pub fn get_notes_in_account(account_name: &str) -> Result<Vec<NoteInfo>> {
    let app = notes_app()?;
    unsafe {
        let accounts_arr = app_accounts(app_as_any(&app));
        let account_count = sb_count(&accounts_arr);
        for i in 0..account_count {
            let account = sb_at(&accounts_arr, i);
            if kvc_string(&account, "name") == account_name {
                let mut out = Vec::new();
                let folders_arr = obj_folders(&account);
                collect_notes_in_folders(&folders_arr, account_name, &mut out);
                return Ok(out);
            }
        }
        Ok(vec![])
    }
}

// ─── Read: attachments ────────────────────────────────────────────────────────

/// Return all attachments for a note (matched by title).
pub fn get_note_attachments_by_title(title: &str) -> Result<Vec<AttachmentInfo>> {
    let app = notes_app()?;
    unsafe {
        let arr = app_notes(app_as_any(&app));
        let count = sb_count(&arr);
        for i in 0..count {
            let note = sb_at(&arr, i);
            let note_title = kvc_string(&note, "name");
            if note_title == title {
                let att_arr = note_attachments(&note);
                let att_count = sb_count(&att_arr);
                let mut out = Vec::with_capacity(att_count);
                for j in 0..att_count {
                    out.push(attachment_info(&sb_at(&att_arr, j), &note_title));
                }
                return Ok(out);
            }
        }
        Ok(vec![])
    }
}

/// Return every attachment from every note.
pub fn get_all_attachments() -> Result<Vec<AttachmentInfo>> {
    let app = notes_app()?;
    unsafe {
        let notes_arr = app_notes(app_as_any(&app));
        let note_count = sb_count(&notes_arr);
        let mut out = Vec::new();
        for i in 0..note_count {
            let note = sb_at(&notes_arr, i);
            let note_title = kvc_string(&note, "name");
            let att_arr = note_attachments(&note);
            let att_count = sb_count(&att_arr);
            for j in 0..att_count {
                out.push(attachment_info(&sb_at(&att_arr, j), &note_title));
            }
        }
        Ok(out)
    }
}

// ─── Write ────────────────────────────────────────────────────────────────────

/// Create a new note with the given title and HTML body in the default folder.
pub fn create_note(title: &str, content: &str) -> Result<()> {
    let app = notes_app()?;
    unsafe {
        // Ask the Notes scripting dictionary for the "note" proxy class.
        let class_name = NSString::from_str("note");
        let note_cls: Option<Retained<AnyObject>> =
            msg_send![&*app, classForScriptingClass: &*class_name];
        let note_cls =
            note_cls.context("Notes scripting class 'note' not found — is Notes.app installed?")?;

        // alloc + init via raw pointers (objc2 reserves Allocated<T> for typed paths).
        let raw_alloc: *mut AnyObject = msg_send![&*note_cls, alloc];
        let raw_init: *mut AnyObject = msg_send![raw_alloc, init];
        let note = Retained::from_raw(raw_init)
            .ok_or_else(|| anyhow!("Failed to instantiate note object"))?;

        // Insert into the element array → ScriptingBridge sends the Apple Event.
        let arr = app_notes(app_as_any(&app));
        let _: () = msg_send![&*arr, insertObject: &*note, atIndex: 0usize];

        // Use KVC: setName:/setBody: are ScriptingBridge-generated and absent from
        // the static method table — same issue as the collection accessors above.
        kvc_set(&note, "name", title);
        kvc_set(&note, "body", content);
    }
    Ok(())
}

/// Update title and/or body of an existing note (looked up by current title).
/// Returns `false` if no note with `title` exists.
pub fn update_note(title: &str, new_title: Option<&str>, content: Option<&str>) -> Result<bool> {
    let app = notes_app()?;
    unsafe {
        let arr = app_notes(app_as_any(&app));
        let count = sb_count(&arr);
        for i in 0..count {
            let note = sb_at(&arr, i);
            if kvc_string(&note, "name") == title {
                if let Some(t) = new_title {
                    kvc_set(&note, "name", t);
                }
                if let Some(c) = content {
                    kvc_set(&note, "body", c);
                }
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Permanently delete a note by title.
/// Returns `false` if no note with `title` exists.
pub fn delete_note(title: &str) -> Result<bool> {
    let app = notes_app()?;
    unsafe {
        let arr = app_notes(app_as_any(&app));
        let count = sb_count(&arr);
        for i in 0..count {
            let note = sb_at(&arr, i);
            if kvc_string(&note, "name") == title {
                // `delete:` is ScriptingBridge-generated; use performSelector:withObject:.
                let sel = objc2::sel!(delete:);
                let _: () =
                    msg_send![app_as_any(&app), performSelector: sel, withObject: &*note];
                return Ok(true);
            }
        }
        Ok(false)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_notes() {
        let notes = list_notes().unwrap();
        assert!(notes.len() > 0);
    }

    #[test]
    fn test_list_accounts() {
        let accounts = list_accounts().unwrap();
        assert!(accounts.len() > 0);
        for a in &accounts {
            assert!(!a.name.is_empty(), "account name should not be empty");
            assert!(!a.id.is_empty(), "account id should not be empty");
        }
    }

    #[test]
    fn test_list_folders() {
        let folders = list_folders().unwrap();
        assert!(folders.len() > 0);
        for f in &folders {
            assert!(!f.name.is_empty(), "folder name should not be empty");
            assert!(!f.account.is_empty(), "folder account should not be empty: {f:?}");
        }
    }

    #[test]
    fn test_get_all_notes() {
        let notes = get_all_notes().unwrap();
        assert!(notes.len() > 0);
        let first = &notes[0];
        assert!(!first.title.is_empty());
        assert!(!first.id.is_empty());
        assert!(!first.creation_date.is_empty());
        assert!(!first.folder.is_empty(), "folder empty: {first:?}");
        assert!(!first.account.is_empty(), "account empty: {first:?}");
    }

    #[test]
    fn test_get_note_by_title() {
        let titles = list_notes().unwrap();
        if let Some(title) = titles.first() {
            let note = get_note_by_title(title).unwrap();
            assert!(note.is_some(), "note should be found by its own title");
            let note = note.unwrap();
            assert_eq!(&note.title, title);
            assert!(!note.id.is_empty());
            assert!(!note.folder.is_empty(), "folder empty for note: {note:?}");
        }
    }
}
