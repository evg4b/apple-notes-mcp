use super::helpers::{kvc_bool, kvc_bool_vec, kvc_string, kvc_string_vec, sb_at, sb_collection, sb_count};
use super::types::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo};
use objc2::extern_class;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::NSObject;

#[link(name = "ScriptingBridge", kind = "framework")]
unsafe extern "C" {}

extern_class!(
    #[unsafe(super(NSObject))]
    pub struct SBObject;
);

extern_class!(
    #[unsafe(super(SBObject, NSObject))]
    pub struct SBApplication;
);

pub(super) unsafe fn app_notes(app: &AnyObject) -> Retained<AnyObject> {
    unsafe { sb_collection(app, objc2::sel!(notes)) }
}
pub(super) unsafe fn app_accounts(app: &SBApplication) -> Retained<AnyObject> {
    unsafe { sb_collection(app, objc2::sel!(accounts)) }
}
pub(super) unsafe fn obj_notes(obj: &AnyObject) -> Retained<AnyObject> {
    unsafe { sb_collection(obj, objc2::sel!(notes)) }
}
pub(super) unsafe fn obj_folders(obj: &AnyObject) -> Retained<AnyObject> {
    unsafe { sb_collection(obj, objc2::sel!(folders)) }
}

#[allow(dead_code)]
pub(super) unsafe fn note_attachments(note: &AnyObject) -> Retained<AnyObject> {
    unsafe { sb_collection(note, objc2::sel!(attachments)) }
}

pub(super) unsafe fn account_info(obj: &AnyObject) -> AccountInfo {
    AccountInfo {
        id: unsafe { kvc_string(obj, "id") },
        name: unsafe { kvc_string(obj, "name") },
    }
}

/// Build a NoteInfo from an SBObject note proxy.
///
/// `folder_name` and `account_name` are passed by the caller (see note above
/// in `folder_info`).
pub(super) unsafe fn note_info(obj: &AnyObject, folder_name: &str, account_name: &str) -> NoteInfo {
    NoteInfo {
        id: unsafe { kvc_string(obj, "id") },
        title: unsafe { kvc_string(obj, "name") },
        body: unsafe { kvc_string(obj, "body") },
        creation_date: unsafe { kvc_string(obj, "creationDate") },
        modification_date: unsafe { kvc_string(obj, "modificationDate") },
        folder: folder_name.to_owned(),
        account: account_name.to_owned(),
        shared: unsafe { kvc_bool(obj, "shared") },
        password_protected: unsafe { kvc_bool(obj, "passwordProtected") },
    }
}

#[allow(dead_code)]
pub(super) unsafe fn attachment_info(obj: &AnyObject, note_title: &str) -> AttachmentInfo {
    AttachmentInfo {
        id: unsafe { kvc_string(obj, "id") },
        name: unsafe { kvc_string(obj, "name") },
        creation_date: unsafe { kvc_string(obj, "creationDate") },
        modification_date: unsafe { kvc_string(obj, "modificationDate") },
        url: unsafe { kvc_string(obj, "URL") },
        note_title: note_title.to_owned(),
    }
}

/// Collect all `FolderInfo` entries from a folder array with the given account context.
/// Recurses into sub-folders.
///
/// Batch-fetches `id` and `name` for the current level in two Apple Events
/// instead of two per folder. Sub-folder recursion still requires one
/// `sb_at` + one `obj_folders` per folder.
pub(super) unsafe fn collect_folders(
    folders_arr: &AnyObject,
    account_name: &str,
    parent_name: &str,
    out: &mut Vec<FolderInfo>,
) {
    let count = unsafe { sb_count(folders_arr) };
    if count == 0 {
        return;
    }
    let ids = unsafe { kvc_string_vec(folders_arr, "id") };
    let names = unsafe { kvc_string_vec(folders_arr, "name") };
    for i in 0..count {
        let folder_name = names.get(i).cloned().unwrap_or_default();
        out.push(FolderInfo {
            id: ids.get(i).cloned().unwrap_or_default(),
            name: folder_name.clone(),
            account: account_name.to_owned(),
            parent: parent_name.to_owned(),
        });
        let folder = unsafe { sb_at(folders_arr, i) };
        let sub_arr = unsafe { obj_folders(&folder) };
        unsafe { collect_folders(&sub_arr, account_name, &folder_name, out) };
    }
}

/// Collect all `NoteInfo` entries from a folder's notes array.
///
/// Uses `kvc_string_vec` / `kvc_bool_vec` to fetch each property for *all*
/// notes in one Apple Event per property (7 AEs total) instead of 8 AEs per
/// note. For N notes this reduces Apple Events from O(8N + 2) to O(9).
pub(super) unsafe fn collect_notes_in_folder(
    folder: &AnyObject,
    folder_name: &str,
    account_name: &str,
    out: &mut Vec<NoteInfo>,
) {
    let notes_arr = unsafe { obj_notes(folder) };
    let count = unsafe { sb_count(&notes_arr) };
    if count == 0 {
        return;
    }
    let ids = unsafe { kvc_string_vec(&notes_arr, "id") };
    let names = unsafe { kvc_string_vec(&notes_arr, "name") };
    let bodies = unsafe { kvc_string_vec(&notes_arr, "body") };
    let creation_dates = unsafe { kvc_string_vec(&notes_arr, "creationDate") };
    let mod_dates = unsafe { kvc_string_vec(&notes_arr, "modificationDate") };
    let shared = unsafe { kvc_bool_vec(&notes_arr, "shared") };
    let pw_protected = unsafe { kvc_bool_vec(&notes_arr, "passwordProtected") };
    for (i, id) in ids.iter().enumerate() {
        out.push(NoteInfo {
            id: id.clone(),
            title: names.get(i).cloned().unwrap_or_default(),
            body: bodies.get(i).cloned().unwrap_or_default(),
            creation_date: creation_dates.get(i).cloned().unwrap_or_default(),
            modification_date: mod_dates.get(i).cloned().unwrap_or_default(),
            folder: folder_name.to_owned(),
            account: account_name.to_owned(),
            shared: shared.get(i).copied().unwrap_or_default(),
            password_protected: pw_protected.get(i).copied().unwrap_or_default(),
        });
    }
}

/// Collect all `NoteInfo` entries from all folders in a folder array (recursive).
pub(super) unsafe fn collect_notes_in_folders(
    folders_arr: &AnyObject,
    account_name: &str,
    out: &mut Vec<NoteInfo>,
) {
    let count = unsafe { sb_count(folders_arr) };
    if count == 0 {
        return;
    }
    let names = unsafe { kvc_string_vec(folders_arr, "name") };
    for i in 0..count {
        let folder = unsafe { sb_at(folders_arr, i) };
        let folder_name = names.get(i).cloned().unwrap_or_default();
        unsafe { collect_notes_in_folder(&folder, &folder_name, account_name, out) };
        let sub_arr = unsafe { obj_folders(&folder) };
        unsafe { collect_notes_in_folders(&sub_arr, account_name, out) };
    }
}
