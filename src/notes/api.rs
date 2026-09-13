use anyhow::{Context, Result, anyhow};
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSMutableDictionary, NSObject, NSString};
use tracing::{debug, error, info, instrument, trace, warn};

use super::bridge::{
    SBApplication, SearchFields, account_info, app_accounts, app_notes, collect_attachments,
    collect_folders, collect_notes_in_folder, collect_notes_in_folders, note_info, obj_folders,
    obj_notes, search_notes_in_folders,
};
use super::helpers::{
    keys, kvc_index_of, kvc_set, kvc_string, kvc_string_vec, sb_at, sb_count, sb_perform,
};
use super::types::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo, NotePage, PartialNoteInfo};

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

    /// Every note, capped at `limit`. One note past the cap is collected so the
    /// caller can report whether anything was left behind.
    #[instrument(skip(self))]
    pub fn get_all_notes(&self, limit: usize) -> Result<NotePage> {
        unsafe {
            let ceiling = limit.saturating_add(1);
            let mut out = Vec::new();
            self.for_each_account(|account, account_name| {
                let folders_arr = obj_folders(account);
                collect_notes_in_folders(&folders_arr, account_name, ceiling, &mut out);
            });
            debug!(total = out.len(), "collected all notes");
            Ok(NotePage::from_overshoot(out, limit))
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
    pub fn get_notes_in_folder(&self, folder_name: &str, limit: usize) -> Result<NotePage> {
        unsafe {
            let ceiling = limit.saturating_add(1);
            let mut out = Vec::new();
            self.with_folder(folder_name, |folder, account_name| {
                collect_notes_in_folder(folder, folder_name, account_name, ceiling, &mut out);
            });
            debug!(count = out.len(), "collected notes in folder");
            Ok(NotePage::from_overshoot(out, limit))
        }
    }

    #[instrument(skip(self))]
    pub fn get_notes_in_account(&self, account_name: &str, limit: usize) -> Result<NotePage> {
        unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
            let target = NSString::from_str(account_name);
            let Some(i) = kvc_index_of(&accounts_arr, keys::name(), &target) else {
                debug!("account not found");
                return Ok(NotePage::from_overshoot(Vec::new(), limit));
            };
            let account = sb_at(&accounts_arr, i);
            let ceiling = limit.saturating_add(1);
            let mut out = Vec::new();
            collect_notes_in_folders(&obj_folders(&account), account_name, ceiling, &mut out);
            debug!(count = out.len(), "collected notes in account");
            Ok(NotePage::from_overshoot(out, limit))
        }
    }

    /// Case-insensitive substring search over note titles and, optionally, bodies.
    ///
    /// `limit` caps the number of results so a broad query cannot pull an entire
    /// library into memory.
    #[instrument(skip(self))]
    pub fn search_notes(&self, query: &str, in_body: bool, limit: usize) -> Result<NotePage> {
        if limit == 0 {
            return Ok(NotePage::from_overshoot(Vec::new(), 0));
        }
        let ceiling = limit.saturating_add(1);
        let needle = query.to_lowercase();
        let fields = SearchFields {
            title: true,
            body: in_body,
        };
        unsafe {
            let mut out = Vec::new();
            self.for_each_account(|account, account_name| {
                if out.len() >= ceiling {
                    return;
                }
                let folders_arr = obj_folders(account);
                search_notes_in_folders(
                    &folders_arr,
                    account_name,
                    &needle,
                    fields,
                    ceiling,
                    &mut out,
                );
            });
            debug!(matches = out.len(), "search complete");
            Ok(NotePage::from_overshoot(out, limit))
        }
    }

    #[instrument(skip(self))]
    pub fn get_note_attachments(&self, title: &str) -> Result<Vec<AttachmentInfo>> {
        unsafe {
            let arr = app_notes(&self.sb_app);
            let target = NSString::from_str(title);
            let Some(i) = kvc_index_of(&arr, keys::name(), &target) else {
                debug!("note not found");
                return Ok(Vec::new());
            };
            let mut out = Vec::new();
            collect_attachments(&sb_at(&arr, i), title, &mut out);
            debug!(count = out.len(), "collected attachments");
            Ok(out)
        }
    }

    /// Create a note in `folder`, or in the default folder when `folder` is `None`.
    #[instrument(skip(self, content))]
    pub fn create_note(
        &self,
        title: &str,
        content: &str,
        folder: Option<&str>,
    ) -> Result<PartialNoteInfo> {
        unsafe {
            let note =
                self.new_object("note", &[(keys::name(), title), (keys::body(), content)])?;

            let collection = match folder {
                None => app_notes(&self.sb_app),
                Some(name) => {
                    let folder = self
                        .find_folder(name)
                        .ok_or_else(|| anyhow!("Folder {name:?} not found"))?;
                    obj_notes(&folder)
                }
            };
            let _: () = msg_send![&*collection, insertObject: &*note, atIndex: 0usize];
            debug!("note created");

            // The freshly inserted proxy is unresolved; read the stored note back
            // out of the collection to get its assigned id and dates.
            let resolved = sb_at(&collection, 0);
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
            let Some((arr, i)) = self.find_note(title) else {
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

    /// Append HTML to the end of a note's body, leaving the existing body intact.
    #[instrument(skip(self, content))]
    pub fn append_to_note(&self, title: &str, content: &str) -> Result<Option<PartialNoteInfo>> {
        unsafe {
            let Some((arr, i)) = self.find_note(title) else {
                debug!("note not found");
                return Ok(None);
            };
            let note = sb_at(&arr, i);
            let mut body = kvc_string(&note, keys::body());
            body.push_str(content);
            kvc_set(&note, keys::body(), &body);
            debug!("note appended to");

            Ok(Some(PartialNoteInfo {
                id: kvc_string(&note, keys::id()),
                title: None,
                body: Some(body),
                creation_date: None,
                modification_date: Some(kvc_string(&note, keys::modification_date())),
            }))
        }
    }

    /// Move a note into `folder_name`, keeping its id, dates and attachments.
    #[instrument(skip(self))]
    pub fn move_note(&self, title: &str, folder_name: &str) -> Result<Option<PartialNoteInfo>> {
        unsafe {
            let folder = self
                .find_folder(folder_name)
                .ok_or_else(|| anyhow!("Folder {folder_name:?} not found"))?;
            let Some((arr, i)) = self.find_note(title) else {
                debug!("note not found");
                return Ok(None);
            };
            let note = sb_at(&arr, i);
            let moved = sb_perform(&note, objc2::sel!(moveTo:), &folder)
                .ok_or_else(|| anyhow!("Notes refused to move {title:?} to {folder_name:?}"))?;
            debug!("note moved");

            Ok(Some(PartialNoteInfo {
                id: kvc_string(&moved, keys::id()),
                title: Some(title.to_owned()),
                body: None,
                creation_date: None,
                modification_date: Some(kvc_string(&moved, keys::modification_date())),
            }))
        }
    }

    /// Create a top-level folder in `account`, or in the first account when
    /// `account` is `None`.
    #[instrument(skip(self))]
    pub fn create_folder(&self, name: &str, account: Option<&str>) -> Result<FolderInfo> {
        unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
            let index = match account {
                None => 0,
                Some(account_name) => {
                    let target = NSString::from_str(account_name);
                    kvc_index_of(&accounts_arr, keys::name(), &target)
                        .ok_or_else(|| anyhow!("Account {account_name:?} not found"))?
                }
            };
            if sb_count(&accounts_arr) == 0 {
                anyhow::bail!("Notes reported no accounts — check Automation permission");
            }
            let account_obj = sb_at(&accounts_arr, index);
            let account_name = kvc_string(&account_obj, keys::name());

            let folder = self.new_object("folder", &[(keys::name(), name)])?;
            let folders_arr = obj_folders(&account_obj);
            let _: () = msg_send![&*folders_arr, insertObject: &*folder, atIndex: 0usize];
            debug!("folder created");

            let resolved = sb_at(&folders_arr, 0);
            Ok(FolderInfo {
                id: kvc_string(&resolved, keys::id()),
                name: name.to_owned(),
                parent: account_name.clone(),
                account: account_name,
            })
        }
    }

    /// Delete a top-level folder and everything inside it.
    #[instrument(skip(self))]
    pub fn delete_folder(&self, name: &str) -> Result<bool> {
        unsafe {
            let target = NSString::from_str(name);
            let accounts_arr = app_accounts(&self.sb_app);
            for i in 0..sb_count(&accounts_arr) {
                let account = sb_at(&accounts_arr, i);
                let folders_arr = obj_folders(&account);
                if let Some(j) = kvc_index_of(&folders_arr, keys::name(), &target) {
                    let _: () = msg_send![&*folders_arr, removeObjectAtIndex: j];
                    debug!("folder deleted");
                    return Ok(true);
                }
            }
            debug!("folder not found");
            Ok(false)
        }
    }

    #[instrument(skip(self))]
    pub fn delete_note(&self, title: &str) -> Result<bool> {
        unsafe {
            let Some((arr, i)) = self.find_note(title) else {
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

    /// Allocate a scripting object of `class_name` with its properties already set.
    ///
    /// `initWithProperties:` carries the fields in the creation Apple Event,
    /// which is more reliable than setting them via KVC after insertion.
    unsafe fn new_object(
        &self,
        class_name: &str,
        properties: &[(&NSString, &str)],
    ) -> Result<Retained<AnyObject>> {
        unsafe {
            let class_ns = NSString::from_str(class_name);
            let cls: Option<Retained<AnyObject>> =
                msg_send![&*self.sb_app, classForScriptingClass: &*class_ns];
            let cls = cls.with_context(|| {
                format!("Notes scripting class {class_name:?} not found — is Notes.app installed?")
            })?;

            let props = NSMutableDictionary::<NSString, NSObject>::new();
            for (key, value) in properties {
                let value_ns = NSString::from_str(value);
                let _: () = msg_send![&*props, setValue: &*value_ns, forKey: *key];
            }

            let raw_alloc: *mut AnyObject = msg_send![&*cls, alloc];
            if raw_alloc.is_null() {
                anyhow::bail!("Failed to allocate {class_name} object");
            }
            let raw_init: *mut AnyObject = msg_send![raw_alloc, initWithProperties: &*props];
            Retained::from_raw(raw_init)
                .ok_or_else(|| anyhow!("Failed to initialize {class_name} object"))
        }
    }

    /// Locate a note by exact title across the whole library, returning the
    /// collection it lives in together with its index.
    unsafe fn find_note(&self, title: &str) -> Option<(Retained<AnyObject>, usize)> {
        unsafe {
            let arr = app_notes(&self.sb_app);
            let target = NSString::from_str(title);
            let index = kvc_index_of(&arr, keys::name(), &target)?;
            Some((arr, index))
        }
    }

    /// Locate a top-level folder by exact name in any account.
    unsafe fn find_folder(&self, folder_name: &str) -> Option<Retained<AnyObject>> {
        unsafe {
            let target = NSString::from_str(folder_name);
            let accounts_arr = app_accounts(&self.sb_app);
            for i in 0..sb_count(&accounts_arr) {
                let account = sb_at(&accounts_arr, i);
                let folders_arr = obj_folders(&account);
                if let Some(j) = kvc_index_of(&folders_arr, keys::name(), &target) {
                    return Some(sb_at(&folders_arr, j));
                }
            }
            None
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
