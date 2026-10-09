use anyhow::{Context, Result, anyhow, bail};
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSMutableDictionary, NSObject, NSString};
use tracing::{debug, error, info, instrument, trace, warn};

use super::bridge::{
    NoteLocation, SBApplication, SearchFields, app_accounts, collect_attachments, collect_folders,
    collect_notes_in_folder, collect_notes_in_folders, collect_titles_in_folders,
    locate_note_in_folders, note_info, obj_folders, obj_notes, search_notes_in_folders,
};
use super::helpers::{
    keys, kvc_bool, kvc_get, kvc_index_of, kvc_set, kvc_string, kvc_string_vec, sb_at, sb_command,
    sb_count, take_at,
};
use super::types::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo, NotePage, PartialNoteInfo};

pub struct NotesApp {
    sb_app: Retained<SBApplication>,
}

// SAFETY: ScriptingBridge sends synchronous Apple Events, which macOS
// serializes, so sharing one proxy across threads is safe in practice.
unsafe impl Send for NotesApp {}
unsafe impl Sync for NotesApp {}

const APPLE_NOTES_BUNDLE_ID: &str = "com.apple.Notes";

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
            let mut ids = kvc_string_vec(&arr, keys::id());
            let names = kvc_string_vec(&arr, keys::name());
            let out: Vec<_> = names
                .into_iter()
                .enumerate()
                .map(|(i, name)| AccountInfo {
                    id: take_at(&mut ids, i),
                    name,
                })
                .collect();
            debug!(count = out.len(), "listed accounts");
            Ok(out)
        }
    }

    #[instrument(skip(self))]
    pub fn list_folders(&self) -> Result<Vec<FolderInfo>> {
        unsafe {
            let mut out = Vec::new();
            for (account, account_name) in self.accounts() {
                collect_folders(
                    &obj_folders(&account),
                    &account_name,
                    &account_name,
                    &mut out,
                );
            }
            debug!(total = out.len(), "listed folders");
            Ok(out)
        }
    }

    #[instrument(skip(self))]
    pub fn get_subfolders(&self, folder_name: &str) -> Result<Vec<FolderInfo>> {
        unsafe {
            let mut out = Vec::new();
            if let Some(found) = self.find_folder(folder_name, None) {
                let sub_arr = obj_folders(&found.folder());
                collect_folders(&sub_arr, &found.account, folder_name, &mut out);
            }
            debug!(count = out.len(), "listed subfolders");
            Ok(out)
        }
    }

    /// Walked folder by folder rather than read off the application's flat
    /// `notes`, which includes trashed notes: a title listed here must be one
    /// `get_note` can find.
    #[instrument(skip(self))]
    pub fn list_notes(&self) -> Result<Vec<String>> {
        unsafe {
            let mut names = Vec::new();
            for (account, _) in self.accounts() {
                collect_titles_in_folders(&obj_folders(&account), &mut names);
            }
            debug!(count = names.len(), "listed note titles");
            Ok(names)
        }
    }

    #[instrument(skip(self))]
    pub fn get_all_notes(&self, limit: usize) -> Result<NotePage> {
        let page = NotePage::collect(limit, |ceiling, out| unsafe {
            for (account, account_name) in self.accounts() {
                if out.len() >= ceiling {
                    break;
                }
                collect_notes_in_folders(&obj_folders(&account), &account_name, ceiling, out);
            }
        });
        debug!(total = page.notes.len(), "collected all notes");
        Ok(page)
    }

    #[instrument(skip(self))]
    pub fn get_note_by_title(&self, title: &str) -> Result<Option<NoteInfo>> {
        unsafe {
            let found = self.find_note(title).map(|found| {
                note_info(
                    &found.location.note(),
                    &found.location.folder_name,
                    &found.account,
                )
            });
            debug!(found = found.is_some(), "note lookup");
            Ok(found)
        }
    }

    #[instrument(skip(self))]
    pub fn get_notes_in_folder(&self, folder_name: &str, limit: usize) -> Result<NotePage> {
        let page = NotePage::collect(limit, |ceiling, out| unsafe {
            if let Some(found) = self.find_folder(folder_name, None) {
                collect_notes_in_folder(&found.folder(), folder_name, &found.account, ceiling, out);
            }
        });
        debug!(count = page.notes.len(), "collected notes in folder");
        Ok(page)
    }

    #[instrument(skip(self))]
    pub fn get_notes_in_account(&self, account_name: &str, limit: usize) -> Result<NotePage> {
        let Some(account) = (unsafe { self.find_account(account_name) }) else {
            debug!("account not found");
            return Ok(NotePage::default());
        };
        let page = NotePage::collect(limit, |ceiling, out| unsafe {
            collect_notes_in_folders(&obj_folders(&account), account_name, ceiling, out);
        });
        debug!(count = page.notes.len(), "collected notes in account");
        Ok(page)
    }

    #[instrument(skip(self))]
    pub fn search_notes(&self, query: &str, in_body: bool, limit: usize) -> Result<NotePage> {
        if limit == 0 {
            return Ok(NotePage::default());
        }
        let needle = query.to_lowercase();
        let fields = SearchFields {
            title: true,
            body: in_body,
        };
        let page = NotePage::collect(limit, |ceiling, out| unsafe {
            for (account, account_name) in self.accounts() {
                if out.len() >= ceiling {
                    break;
                }
                let folders_arr = obj_folders(&account);
                search_notes_in_folders(&folders_arr, &account_name, &needle, fields, ceiling, out);
            }
        });
        debug!(matches = page.notes.len(), "search complete");
        Ok(page)
    }

    #[instrument(skip(self))]
    pub fn get_note_attachments(&self, title: &str) -> Result<Vec<AttachmentInfo>> {
        unsafe {
            let Some(found) = self.find_note(title) else {
                debug!("note not found");
                return Ok(Vec::new());
            };
            let mut out = Vec::new();
            collect_attachments(&found.location.note(), title, &mut out);
            debug!(count = out.len(), "collected attachments");
            Ok(out)
        }
    }

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
                None => obj_notes(&self.sb_app),
                Some(name) => {
                    // Every account has a "Notes" folder; prefer the one in the
                    // account Notes itself would create the note in.
                    let default_account = kvc_string(&*self.default_account()?, keys::name());
                    let found = self
                        .find_folder(name, Some(&default_account))
                        .ok_or_else(|| anyhow!("Folder {name:?} not found"))?;
                    obj_notes(&found.folder())
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
            let Some(found) = self.find_note(title) else {
                debug!("note not found");
                return Ok(None);
            };
            let note = found.location.note();
            ensure_unlocked(&note, title)?;
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

    #[instrument(skip(self, content))]
    pub fn append_to_note(&self, title: &str, content: &str) -> Result<Option<PartialNoteInfo>> {
        unsafe {
            let Some(found) = self.find_note(title) else {
                debug!("note not found");
                return Ok(None);
            };
            let note = found.location.note();
            ensure_unlocked(&note, title)?;
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

    /// Moves across accounts are refused: Notes carries them out by trashing
    /// the original, and the copy it is meant to leave in the destination is
    /// not reliably there.
    #[instrument(skip(self))]
    pub fn move_note(&self, title: &str, folder_name: &str) -> Result<Option<PartialNoteInfo>> {
        unsafe {
            let Some(found) = self.find_note(title) else {
                debug!("note not found");
                return Ok(None);
            };
            let dest = self
                .find_folder(folder_name, Some(&found.account))
                .ok_or_else(|| anyhow!("Folder {folder_name:?} not found"))?;
            if dest.account != found.account {
                bail!(
                    "{title:?} is in account {:?} but folder {folder_name:?} is in {:?}; \
                     notes cannot be moved between accounts",
                    found.account,
                    dest.account
                );
            }
            let folder = dest.folder();
            let note = found.location.note();
            let id = NSString::from_str(&kvc_string(&note, keys::id()));
            sb_command(&note, objc2::sel!(moveTo:), &folder);

            // `move` returns nothing, so confirm it by finding the note in its
            // destination.
            let dest_notes = obj_notes(&folder);
            let index = kvc_index_of(&dest_notes, keys::id(), &id)
                .ok_or_else(|| anyhow!("Notes did not move {title:?} to {folder_name:?}"))?;
            let moved = sb_at(&dest_notes, index);
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

    #[instrument(skip(self))]
    pub fn create_folder(&self, name: &str, account: Option<&str>) -> Result<FolderInfo> {
        unsafe {
            let (account_obj, account_name) = match account {
                None => {
                    let account = self.default_account()?;
                    let name = kvc_string(&account, keys::name());
                    (account, name)
                }
                Some(name) => {
                    let account = self
                        .find_account(name)
                        .ok_or_else(|| anyhow!("Account {name:?} not found"))?;
                    (account, name.to_owned())
                }
            };

            let folder = self.new_object("folder", &[(keys::name(), name)])?;
            let folders_arr = obj_folders(&account_obj);
            let _: () = msg_send![&*folders_arr, insertObject: &*folder, atIndex: 0usize];
            debug!("folder created");

            // Folders are not kept in insertion order, so find the new one by name.
            let target = NSString::from_str(name);
            let index = kvc_index_of(&folders_arr, keys::name(), &target)
                .ok_or_else(|| anyhow!("Notes did not create folder {name:?}"))?;
            let resolved = sb_at(&folders_arr, index);
            Ok(FolderInfo {
                id: kvc_string(&resolved, keys::id()),
                name: name.to_owned(),
                parent: account_name.clone(),
                account: account_name,
            })
        }
    }

    #[instrument(skip(self))]
    pub fn delete_folder(&self, name: &str) -> Result<bool> {
        unsafe {
            let Some(found) = self.find_folder(name, None) else {
                debug!("folder not found");
                return Ok(false);
            };
            let _: () = msg_send![&*found.parent, removeObjectAtIndex: found.index];
            debug!("folder deleted");
            Ok(true)
        }
    }

    /// Delete a note. Notes moves it to Recently Deleted in accounts that
    /// have one; notes already there are never matched, so this cannot erase
    /// one for good.
    #[instrument(skip(self))]
    pub fn delete_note(&self, title: &str) -> Result<bool> {
        unsafe {
            let Some(found) = self.find_note(title) else {
                debug!("note not found");
                return Ok(false);
            };
            // SBObject resolves `delete` dynamically; removing the element from
            // its parent collection is the supported ScriptingBridge spelling.
            let location = found.location;
            let _: () = msg_send![&*location.notes, removeObjectAtIndex: location.index];
            debug!("note deleted");
            Ok(true)
        }
    }

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
                bail!("Failed to allocate {class_name} object");
            }
            let raw_init: *mut AnyObject = msg_send![raw_alloc, initWithProperties: &*props];
            Retained::from_raw(raw_init)
                .ok_or_else(|| anyhow!("Failed to initialize {class_name} object"))
        }
    }

    /// Walks folders instead of the application's flat `notes`, which includes
    /// trashed notes: a write could land on a deleted copy, and deleting that
    /// copy is permanent.
    unsafe fn find_note(&self, title: &str) -> Option<FoundNote> {
        unsafe {
            let target = NSString::from_str(title);
            self.accounts().find_map(|(account, account_name)| {
                locate_note_in_folders(&obj_folders(&account), &target).map(|location| FoundNote {
                    location,
                    account: account_name,
                })
            })
        }
    }

    /// Locate a folder by exact name, top-level or nested, in any account.
    ///
    /// The search goes breadth-first across all accounts, so a top-level folder
    /// wins over a nested one of the same name. `prefer` puts one account's
    /// folders ahead of the rest at every depth. Each level costs one batched
    /// name fetch per folder array, plus one count per folder to descend.
    unsafe fn find_folder(&self, folder_name: &str, prefer: Option<&str>) -> Option<FoundFolder> {
        unsafe {
            let target = NSString::from_str(folder_name);
            let mut level: Vec<(Retained<AnyObject>, String)> = self
                .accounts()
                .map(|(account, account_name)| (obj_folders(&account), account_name))
                .collect();
            if let Some(preferred) = prefer {
                level.sort_by_key(|(_, account)| account != preferred);
            }
            while !level.is_empty() {
                for (arr, account) in &level {
                    if let Some(index) = kvc_index_of(arr, keys::name(), &target) {
                        return Some(FoundFolder {
                            parent: arr.clone(),
                            index,
                            account: account.clone(),
                        });
                    }
                }
                let mut next = Vec::new();
                for (arr, account) in &level {
                    for i in 0..sb_count(arr) {
                        next.push((obj_folders(&sb_at(arr, i)), account.clone()));
                    }
                }
                level = next;
            }
            None
        }
    }

    /// The account Notes creates new notes in, falling back to the first one.
    unsafe fn default_account(&self) -> Result<Retained<AnyObject>> {
        unsafe {
            if let Some(account) = kvc_get(self.sb_app.as_ref(), keys::default_account()) {
                return Ok(account);
            }
            let accounts_arr = app_accounts(&self.sb_app);
            if sb_count(&accounts_arr) == 0 {
                bail!("Notes reported no accounts — check Automation permission");
            }
            Ok(sb_at(&accounts_arr, 0))
        }
    }

    unsafe fn find_account(&self, name: &str) -> Option<Retained<AnyObject>> {
        unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
            let index = kvc_index_of(&accounts_arr, keys::name(), &NSString::from_str(name))?;
            Some(sb_at(&accounts_arr, index))
        }
    }

    /// Every account with its name. The names are batch-fetched in one Apple
    /// Event; the accounts themselves are resolved lazily, so a caller that
    /// stops early skips the rest.
    unsafe fn accounts(&self) -> impl Iterator<Item = (Retained<AnyObject>, String)> {
        let accounts_arr = unsafe { app_accounts(&self.sb_app) };
        let mut names = unsafe { kvc_string_vec(&accounts_arr, keys::name()) };
        (0..names.len()).map(move |i| {
            let account = unsafe { sb_at(&accounts_arr, i) };
            (account, take_at(&mut names, i))
        })
    }
}

struct FoundNote {
    location: NoteLocation,
    account: String,
}

/// Kept as a position in the parent's element array so it can be removed as
/// well as read.
struct FoundFolder {
    parent: Retained<AnyObject>,
    index: usize,
    account: String,
}

impl FoundFolder {
    unsafe fn folder(&self) -> Retained<AnyObject> {
        unsafe { sb_at(&self.parent, self.index) }
    }
}

/// Notes hides a locked note's body from scripts, so an edit would overwrite
/// it with an empty one.
unsafe fn ensure_unlocked(note: &AnyObject, title: &str) -> Result<()> {
    if unsafe { kvc_bool(note, keys::password_protected()) } {
        bail!("{title:?} is password-protected; unlock it in Notes to edit it");
    }
    Ok(())
}
