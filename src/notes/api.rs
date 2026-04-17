use anyhow::{Context, Result, anyhow};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{ClassType, msg_send};
use objc2_foundation::NSString;
use tracing::{debug, info, instrument, warn};

use super::bridge::SBApplication;
use super::bridge::{
    account_info, app_accounts, app_notes, attachment_info, collect_folders,
    collect_notes_in_folder, collect_notes_in_folders, note_attachments, note_info, obj_folders,
    obj_notes,
};
use super::helpers::{kvc_set, kvc_string, kvc_string_vec, sb_at, sb_count};
use super::types::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo};

/// A live ScriptingBridge proxy to Notes.app.
pub struct NotesApp {
    sb_app: Retained<SBApplication>,
}

// ScriptingBridge sends synchronous Apple Events which macOS serializes at the
// OS level, so sharing a single proxy across threads is safe in practice.
unsafe impl Send for NotesApp {}
unsafe impl Sync for NotesApp {}

static APPLE_NOTES_BUNDLE_ID: &str = "com.apple.Notes";

impl NotesApp {
    pub fn connect() -> Result<Self> {
        let bundle_id = NSString::from_str(APPLE_NOTES_BUNDLE_ID);
        let retained_app: Option<Retained<SBApplication>> = unsafe {
            msg_send![SBApplication::class(), applicationWithBundleIdentifier: &*bundle_id]
        };

        let sb_app =
            retained_app.ok_or(anyhow!("Cannot connect to Apple Notes via ScriptingBridge"))?;

        let app = Self { sb_app };

        match app.list_accounts() {
            Ok(accounts) if accounts.is_empty() => {
                warn!(
                    "Notes returned 0 accounts — Automation permission is probably missing. \
                     Go to System Settings → Privacy & Security → Automation and allow \
                     this binary to control Notes.app, then restart."
                );
            }
            Ok(accounts) => {
                info!(accounts = accounts.len(), "Notes.app connected");
            }
            Err(e) => {
                warn!(error = %e, "Notes.app probe failed — check Automation permission");
            }
        }

        Ok(app)
    }

    #[instrument(skip(self))]
    pub fn list_accounts(&self) -> Result<Vec<AccountInfo>> {
        let result = unsafe {
            let arr = app_accounts(&self.sb_app);
            let count = sb_count(&arr);
            debug!(count, "found accounts");
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                out.push(account_info(&sb_at(&arr, i)));
            }
            out
        };
        Ok(result)
    }

    #[instrument(skip(self))]
    pub fn list_folders(&self) -> Result<Vec<FolderInfo>> {
        let result = unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
            let account_count = sb_count(&accounts_arr);
            let mut out = Vec::with_capacity(account_count);
            for i in 0..account_count {
                let account = sb_at(&accounts_arr, i);
                let account_name = kvc_string(&account, "name");
                let folders_arr = obj_folders(&account);
                let before = out.len();
                collect_folders(&folders_arr, &account_name, &account_name, &mut out);
                debug!(account = %account_name, folders = out.len() - before, "collected folders");
            }
            out
        };
        debug!(total = result.len(), "list_folders complete");
        Ok(result)
    }

    #[instrument(skip(self))]
    pub fn get_subfolders(&self, folder_name: &str) -> Result<Vec<FolderInfo>> {
        unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
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
                        debug!(count = out.len(), "found subfolders");
                        return Ok(out);
                    }
                }
            }
            debug!("folder not found");
            Ok(vec![])
        }
    }

    #[instrument(skip(self))]
    pub fn list_notes(&self) -> Result<Vec<String>> {
        let result = unsafe {
            let arr = app_notes(&self.sb_app);
            let names = kvc_string_vec(&arr, "name");
            debug!(count = names.len(), "fetching note titles");
            names
        };
        Ok(result)
    }

    #[instrument(skip(self))]
    pub fn get_all_notes(&self) -> Result<Vec<NoteInfo>> {
        let result = unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
            let account_count = sb_count(&accounts_arr);
            let mut out = Vec::with_capacity(account_count);
            for i in 0..account_count {
                let account = sb_at(&accounts_arr, i);
                let account_name = kvc_string(&account, "name");
                let folders_arr = obj_folders(&account);
                let before = out.len();
                collect_notes_in_folders(&folders_arr, &account_name, &mut out);
                debug!(account = %account_name, notes = out.len() - before, "collected notes");
            }
            out
        };
        debug!(total = result.len(), "get_all_notes complete");
        Ok(result)
    }

    #[instrument(skip(self))]
    pub fn get_note_by_title(&self, title: &str) -> Result<Option<NoteInfo>> {
        unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
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
                    // Batch-fetch all titles in this folder: 1 AE instead of 2 per note.
                    let names = kvc_string_vec(&notes_arr, "name");
                    if let Some(k) = names.iter().position(|n| n == title) {
                        let note = sb_at(&notes_arr, k);
                        debug!(folder = %folder_name, account = %account_name, "note found");
                        return Ok(Some(note_info(&note, &folder_name, &account_name)));
                    }
                }
            }
            debug!("note not found");
            Ok(None)
        }
    }

    #[instrument(skip(self))]
    pub fn get_notes_in_folder(&self, folder_name: &str) -> Result<Vec<NoteInfo>> {
        unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
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
                        debug!(count = out.len(), account = %account_name, "found notes in folder");
                        return Ok(out);
                    }
                }
            }
            debug!("folder not found");
            Ok(vec![])
        }
    }

    #[instrument(skip(self))]
    pub fn get_notes_in_account(&self, account_name: &str) -> Result<Vec<NoteInfo>> {
        unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
            let account_count = sb_count(&accounts_arr);
            for i in 0..account_count {
                let account = sb_at(&accounts_arr, i);
                if kvc_string(&account, "name") == account_name {
                    let mut out = Vec::new();
                    let folders_arr = obj_folders(&account);
                    collect_notes_in_folders(&folders_arr, account_name, &mut out);
                    debug!(count = out.len(), "found notes in account");
                    return Ok(out);
                }
            }
            debug!("account not found");
            Ok(vec![])
        }
    }

    #[instrument(skip(self))]
    pub fn get_note_attachments_by_title(&self, title: &str) -> Result<Vec<AttachmentInfo>> {
        unsafe {
            let arr = app_notes(&self.sb_app);
            let names = kvc_string_vec(&arr, "name");
            if let Some(i) = names.iter().position(|n| n == title) {
                let note = sb_at(&arr, i);
                let att_arr = note_attachments(&note);
                let att_count = sb_count(&att_arr);
                debug!(count = att_count, "found attachments");
                let mut out = Vec::with_capacity(att_count);
                for j in 0..att_count {
                    out.push(attachment_info(&sb_at(&att_arr, j), title));
                }
                return Ok(out);
            }
            debug!("note not found");
            Ok(vec![])
        }
    }

    #[instrument(skip(self))]
    pub fn get_all_attachments(&self) -> Result<Vec<AttachmentInfo>> {
        unsafe {
            let notes_arr = app_notes(&self.sb_app);
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
            debug!(total = out.len(), "get_all_attachments complete");
            Ok(out)
        }
    }

    #[instrument(skip(self, content))]
    pub fn create_note(&self, title: &str, content: &str) -> Result<()> {
        unsafe {
            let class_name = NSString::from_str("note");
            let note_cls: Option<Retained<AnyObject>> =
                msg_send![&*self.sb_app, classForScriptingClass: &*class_name];
            let note_cls = note_cls
                .context("Notes scripting class 'note' not found — is Notes.app installed?")?;

            let raw_alloc: *mut AnyObject = msg_send![&*note_cls, alloc];
            if raw_alloc.is_null() {
                anyhow::bail!("Failed to allocate note object");
            }
            let raw_init: *mut AnyObject = msg_send![raw_alloc, init];
            let note = Retained::from_raw(raw_init)
                .ok_or_else(|| anyhow!("Failed to initialize note object"))?;

            let arr = app_notes(&self.sb_app);
            let _: () = msg_send![&*arr, insertObject: &*note, atIndex: 0usize];

            kvc_set(&note, "name", title);
            kvc_set(&note, "body", content);
            debug!("note created");
        }
        Ok(())
    }

    #[instrument(skip(self, content))]
    pub fn update_note(
        &self,
        title: &str,
        new_title: Option<&str>,
        content: Option<&str>,
    ) -> Result<bool> {
        unsafe {
            let arr = app_notes(&self.sb_app);
            let names = kvc_string_vec(&arr, "name");
            if let Some(i) = names.iter().position(|n| n == title) {
                let note = sb_at(&arr, i);
                if let Some(t) = new_title {
                    kvc_set(&note, "name", t);
                }
                if let Some(c) = content {
                    kvc_set(&note, "body", c);
                }
                debug!("note updated");
                return Ok(true);
            }
            debug!("note not found");
            Ok(false)
        }
    }

    #[instrument(skip(self))]
    pub fn delete_note(&self, title: &str) -> Result<bool> {
        unsafe {
            let arr = app_notes(&self.sb_app);
            let names = kvc_string_vec(&arr, "name");
            if let Some(i) = names.iter().position(|n| n == title) {
                let note = sb_at(&arr, i);
                let sel = objc2::sel!(delete:);
                let _: () = msg_send![&self.sb_app, performSelector: sel, withObject: &*note];
                debug!("note deleted");
                return Ok(true);
            }
            debug!("note not found");
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> NotesApp {
        NotesApp::connect().expect("failed to connect to Notes.app")
    }

    #[test]
    #[ignore = "requires Notes.app with Automation permission"]
    fn test_list_notes() {
        let notes = app().list_notes().unwrap();
        assert!(!notes.is_empty());
    }

    #[test]
    #[ignore = "requires Notes.app with Automation permission"]
    fn test_list_accounts() {
        let accounts = app().list_accounts().unwrap();
        assert!(!accounts.is_empty());
        for a in &accounts {
            assert!(!a.name.is_empty(), "account name should not be empty");
            assert!(!a.id.is_empty(), "account id should not be empty");
        }
    }

    #[test]
    #[ignore = "requires Notes.app with Automation permission"]
    fn test_list_folders() {
        let folders = app().list_folders().unwrap();
        assert!(!folders.is_empty());
        for f in &folders {
            assert!(!f.name.is_empty(), "folder name should not be empty");
            assert!(
                !f.account.is_empty(),
                "folder account should not be empty: {f:?}"
            );
        }
    }

    #[test]
    #[ignore = "requires Notes.app with Automation permission"]
    fn test_get_all_notes() {
        let notes = app().get_all_notes().unwrap();
        assert!(!notes.is_empty());
        let first = notes.first().expect("asserted non-empty above");
        assert!(!first.title.is_empty());
        assert!(!first.id.is_empty());
        assert!(!first.creation_date.is_empty());
        assert!(!first.folder.is_empty(), "folder empty: {first:?}");
        assert!(!first.account.is_empty(), "account empty: {first:?}");
    }

    #[test]
    #[ignore = "requires Notes.app with Automation permission"]
    fn test_get_note_by_title() {
        let app = app();
        let titles = app.list_notes().unwrap();
        if let Some(title) = titles.first() {
            let note = app.get_note_by_title(title).unwrap();
            assert!(note.is_some(), "note should be found by its own title");
            let note = note.expect("asserted is_some above");
            assert_eq!(&note.title, title);
            assert!(!note.id.is_empty());
            assert!(!note.folder.is_empty(), "folder empty for note: {note:?}");
        }
    }
}
