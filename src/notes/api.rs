use anyhow::{Context, Result, anyhow};
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSMutableDictionary, NSObject, NSString};
use tracing::{debug, error, info, instrument, trace, warn};

use super::bridge::{
    SBApplication, account_info, app_accounts, app_notes, collect_folders, collect_notes_in_folder,
    collect_notes_in_folders, note_info, obj_folders, obj_notes,
};
use super::helpers::{keys, kvc_index_of, kvc_set, kvc_string, kvc_string_vec, sb_at, sb_count};
use super::types::{AccountInfo, FolderInfo, NoteInfo, PartialNoteInfo};

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
        trace!(
            bundle_id = APPLE_NOTES_BUNDLE_ID,
            "connecting via ScriptingBridge"
        );

        let sb_app = unsafe { SBApplication::applicationWithBundleIdentifier(&bundle_id) }
            .ok_or_else(|| anyhow!("Cannot connect to Apple Notes via ScriptingBridge"))?;
        let app = Self { sb_app };

        match app.list_accounts() {
            Ok(accounts) if accounts.is_empty() => warn!(
                "Notes returned 0 accounts — Automation permission is probably missing. \
                 Go to System Settings → Privacy & Security → Automation and allow \
                 this binary to control Notes.app, then restart."
            ),
            Ok(accounts) => info!(accounts = accounts.len(), "Notes.app connected"),
            Err(e) => {
                error!(error = %e, "Notes.app probe failed — check Automation permission");
                return Err(e);
            }
        }

        Ok(app)
    }

    #[instrument(skip(self))]
    pub fn list_accounts(&self) -> Result<Vec<AccountInfo>> {
        unsafe {
            let arr = app_accounts(&self.sb_app);
            let count = sb_count(&arr);
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                out.push(account_info(&sb_at(&arr, i)));
            }
            debug!(count, "listed accounts");
            Ok(out)
        }
    }

    #[instrument(skip(self))]
    pub fn list_folders(&self) -> Result<Vec<FolderInfo>> {
        unsafe {
            let mut out = Vec::new();
            self.for_each_account(|account, account_name| {
                let folders_arr = obj_folders(account);
                collect_folders(&folders_arr, account_name, account_name, &mut out);
            });
            debug!(total = out.len(), "listed folders");
            Ok(out)
        }
    }

    #[instrument(skip(self))]
    pub fn get_subfolders(&self, folder_name: &str) -> Result<Vec<FolderInfo>> {
        unsafe {
            let mut out = Vec::new();
            self.with_folder(folder_name, |folder, account_name| {
                let sub_arr = obj_folders(folder);
                collect_folders(&sub_arr, account_name, folder_name, &mut out);
            });
            debug!(count = out.len(), "listed subfolders");
            Ok(out)
        }
    }

    #[instrument(skip(self))]
    pub fn list_notes(&self) -> Result<Vec<String>> {
        unsafe {
            let arr = app_notes(&self.sb_app);
            let names = kvc_string_vec(&arr, keys::name());
            debug!(count = names.len(), "listed note titles");
            Ok(names)
        }
    }

    #[instrument(skip(self))]
    pub fn get_all_notes(&self) -> Result<Vec<NoteInfo>> {
        unsafe {
            let mut out = Vec::new();
            self.for_each_account(|account, account_name| {
                let folders_arr = obj_folders(account);
                collect_notes_in_folders(&folders_arr, account_name, &mut out);
            });
            debug!(total = out.len(), "collected all notes");
            Ok(out)
        }
    }

    #[instrument(skip(self))]
    pub fn get_note_by_title(&self, title: &str) -> Result<Option<NoteInfo>> {
        unsafe {
            let target = NSString::from_str(title);
            let accounts_arr = app_accounts(&self.sb_app);
            for i in 0..sb_count(&accounts_arr) {
                let account = sb_at(&accounts_arr, i);
                let account_name = kvc_string(&account, keys::name());
                let folders_arr = obj_folders(&account);
                let folder_count = sb_count(&folders_arr);
                for j in 0..folder_count {
                    let folder = sb_at(&folders_arr, j);
                    let notes_arr = obj_notes(&folder);
                    // One batched title fetch per folder, compared as NSStrings.
                    let Some(k) = kvc_index_of(&notes_arr, keys::name(), &target) else {
                        continue;
                    };
                    let folder_name = kvc_string(&folder, keys::name());
                    debug!(folder = %folder_name, account = %account_name, "note found");
                    let note = sb_at(&notes_arr, k);
                    return Ok(Some(note_info(&note, &folder_name, &account_name)));
                }
            }
            debug!("note not found");
            Ok(None)
        }
    }

    #[instrument(skip(self))]
    pub fn get_notes_in_folder(&self, folder_name: &str) -> Result<Vec<NoteInfo>> {
        unsafe {
            let mut out = Vec::new();
            self.with_folder(folder_name, |folder, account_name| {
                collect_notes_in_folder(folder, folder_name, account_name, &mut out);
            });
            debug!(count = out.len(), "collected notes in folder");
            Ok(out)
        }
    }

    #[instrument(skip(self))]
    pub fn get_notes_in_account(&self, account_name: &str) -> Result<Vec<NoteInfo>> {
        unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
            let target = NSString::from_str(account_name);
            let Some(i) = kvc_index_of(&accounts_arr, keys::name(), &target) else {
                debug!("account not found");
                return Ok(Vec::new());
            };
            let account = sb_at(&accounts_arr, i);
            let mut out = Vec::new();
            collect_notes_in_folders(&obj_folders(&account), account_name, &mut out);
            debug!(count = out.len(), "collected notes in account");
            Ok(out)
        }
    }

    #[instrument(skip(self, content))]
    pub fn create_note(&self, title: &str, content: &str) -> Result<PartialNoteInfo> {
        unsafe {
            let note = self.new_note_object(title, content)?;

            let arr = app_notes(&self.sb_app);
            let _: () = msg_send![&*arr, insertObject: &*note, atIndex: 0usize];
            debug!("note created");

            // The freshly inserted proxy is unresolved; read the stored note back
            // out of the collection to get its assigned id and dates.
            let resolved = sb_at(&arr, 0);
            Ok(PartialNoteInfo {
                id: kvc_string(&resolved, keys::id()),
                title: Some(title.to_owned()),
                body: Some(content.to_owned()),
                creation_date: Some(kvc_string(&resolved, keys::creation_date())),
                modification_date: Some(kvc_string(&resolved, keys::modification_date())),
            })
        }
    }

    #[instrument(skip(self, content))]
    pub fn update_note(
        &self,
        title: &str,
        new_title: Option<&str>,
        content: Option<&str>,
    ) -> Result<Option<PartialNoteInfo>> {
        unsafe {
            let arr = app_notes(&self.sb_app);
            let target = NSString::from_str(title);
            let Some(i) = kvc_index_of(&arr, keys::name(), &target) else {
                debug!("note not found");
                return Ok(None);
            };
            let note = sb_at(&arr, i);
            if let Some(body) = content {
                kvc_set(&note, keys::body(), body);
            }
            if let Some(name) = new_title {
                kvc_set(&note, keys::name(), name);
            }
            debug!("note updated");

            Ok(Some(PartialNoteInfo {
                id: kvc_string(&note, keys::id()),
                title: new_title.map(str::to_owned),
                body: content.map(str::to_owned),
                creation_date: None,
                modification_date: Some(kvc_string(&note, keys::modification_date())),
            }))
        }
    }

    #[instrument(skip(self))]
    pub fn delete_note(&self, title: &str) -> Result<bool> {
        unsafe {
            let arr = app_notes(&self.sb_app);
            let target = NSString::from_str(title);
            let Some(i) = kvc_index_of(&arr, keys::name(), &target) else {
                debug!("note not found");
                return Ok(false);
            };
            // SBObject resolves `delete` dynamically; removing the element from
            // its parent collection is the supported ScriptingBridge spelling.
            let _: () = msg_send![&*arr, removeObjectAtIndex: i];
            debug!("note deleted");
            Ok(true)
        }
    }

    /// Allocate a `note` scripting object with its properties already set.
    ///
    /// `initWithProperties:` carries both fields in the creation Apple Event,
    /// which is more reliable than setting them via KVC after insertion.
    unsafe fn new_note_object(&self, title: &str, content: &str) -> Result<Retained<AnyObject>> {
        unsafe {
            let class_name = NSString::from_str("note");
            let note_cls: Option<Retained<AnyObject>> =
                msg_send![&*self.sb_app, classForScriptingClass: &*class_name];
            let note_cls = note_cls
                .context("Notes scripting class 'note' not found — is Notes.app installed?")?;

            let props = NSMutableDictionary::<NSString, NSObject>::new();
            let name_val = NSString::from_str(title);
            let body_val = NSString::from_str(content);
            let _: () = msg_send![&*props, setValue: &*name_val, forKey: keys::name()];
            let _: () = msg_send![&*props, setValue: &*body_val, forKey: keys::body()];

            let raw_alloc: *mut AnyObject = msg_send![&*note_cls, alloc];
            if raw_alloc.is_null() {
                anyhow::bail!("Failed to allocate note object");
            }
            let raw_init: *mut AnyObject = msg_send![raw_alloc, initWithProperties: &*props];
            Retained::from_raw(raw_init).ok_or_else(|| anyhow!("Failed to initialize note object"))
        }
    }

    /// Run `f` for every account, passing its proxy and name.
    unsafe fn for_each_account(&self, mut f: impl FnMut(&AnyObject, &str)) {
        unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
            for i in 0..sb_count(&accounts_arr) {
                let account = sb_at(&accounts_arr, i);
                let account_name = kvc_string(&account, keys::name());
                f(&account, &account_name);
            }
        }
    }

    /// Run `f` on the first top-level folder named `folder_name`, in any account.
    unsafe fn with_folder(&self, folder_name: &str, mut f: impl FnMut(&AnyObject, &str)) {
        unsafe {
            let target = NSString::from_str(folder_name);
            let accounts_arr = app_accounts(&self.sb_app);
            for i in 0..sb_count(&accounts_arr) {
                let account = sb_at(&accounts_arr, i);
                let folders_arr = obj_folders(&account);
                if let Some(j) = kvc_index_of(&folders_arr, keys::name(), &target) {
                    let account_name = kvc_string(&account, keys::name());
                    f(&sb_at(&folders_arr, j), &account_name);
                    return;
                }
            }
            debug!("folder not found");
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
