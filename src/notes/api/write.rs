use super::NotesApp;
use crate::notes::bridge::{obj_folders, obj_notes};
use crate::notes::helpers::{keys, kvc_bool, kvc_index_of, kvc_set, kvc_string, sb_at, sb_command};
use crate::notes::types::{FolderInfo, PartialNoteInfo};
use anyhow::{Context, Result, anyhow, bail};
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSMutableDictionary, NSObject, NSString};
use std::sync::Arc;
use tracing::{debug, instrument};

impl NotesApp {
    #[instrument(skip(self, content))]
    pub fn create_note(
        &self,
        title: &str,
        content: &str,
        folder: Option<&str>,
    ) -> Result<PartialNoteInfo> {
        self.run(|| unsafe {
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
        })
    }

    #[instrument(skip(self, content))]
    pub fn update_note(
        &self,
        title: &str,
        new_title: Option<&str>,
        content: Option<&str>,
    ) -> Result<Option<PartialNoteInfo>> {
        self.run(|| unsafe {
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
        })
    }

    #[instrument(skip(self, content))]
    pub fn append_to_note(&self, title: &str, content: &str) -> Result<Option<PartialNoteInfo>> {
        self.run(|| unsafe {
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
        })
    }

    /// Moves across accounts are refused: Notes carries them out by trashing
    /// the original, and the copy it is meant to leave in the destination is
    /// not reliably there.
    #[instrument(skip(self))]
    pub fn move_note(&self, title: &str, folder_name: &str) -> Result<Option<PartialNoteInfo>> {
        self.run(|| unsafe {
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
        })
    }

    #[instrument(skip(self))]
    pub fn create_folder(&self, name: &str, account: Option<&str>) -> Result<FolderInfo> {
        self.run(|| unsafe {
            let (account_obj, account_name) = match account {
                None => {
                    let account = self.default_account()?;
                    let name = kvc_string(&account, keys::name()).into();
                    (account, name)
                }
                Some(name) => {
                    let account = self
                        .find_account(name)
                        .ok_or_else(|| anyhow!("Account {name:?} not found"))?;
                    (account, name.into())
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
                name: name.into(),
                parent: Arc::clone(&account_name),
                account: account_name,
            })
        })
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
}

/// Notes hides a locked note's body from scripts, so an edit would overwrite
/// it with an empty one.
unsafe fn ensure_unlocked(note: &AnyObject, title: &str) -> Result<()> {
    if unsafe { kvc_bool(note, keys::password_protected()) } {
        bail!("{title:?} is password-protected; unlock it in Notes to edit it");
    }
    Ok(())
}
