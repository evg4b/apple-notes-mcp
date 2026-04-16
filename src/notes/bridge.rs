use super::helpers::{kvc_bool, kvc_string, sb_at, sb_collection, sb_count};
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
pub(super) unsafe fn note_attachments(note: &AnyObject) -> Retained<AnyObject> {
    unsafe { sb_collection(note, objc2::sel!(attachments)) }
}

pub(super) unsafe fn account_info(obj: &AnyObject) -> AccountInfo {
    AccountInfo {
        id: unsafe { kvc_string(obj, "id") },
        name: unsafe { kvc_string(obj, "name") },
    }
}

/// Build a FolderInfo from an SBObject folder proxy.
///
/// `account_name` and `parent_name` are passed by the caller from the traversal
/// context: the container specifier returned by ScriptingBridge from the flat
/// element arrays is unresolved, so we must supply context from the outside.
pub(super) unsafe fn folder_info(
    obj: &AnyObject,
    account_name: &str,
    parent_name: &str,
) -> FolderInfo {
    FolderInfo {
        id: unsafe { kvc_string(obj, "id") },
        name: unsafe { kvc_string(obj, "name") },
        account: account_name.to_owned(),
        parent: parent_name.to_owned(),
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
pub(super) unsafe fn collect_folders(
    folders_arr: &AnyObject,
    account_name: &str,
    parent_name: &str,
    out: &mut Vec<FolderInfo>,
) {
    let count = unsafe { sb_count(folders_arr) };
    for i in 0..count {
        let folder = unsafe { sb_at(folders_arr, i) };
        let fi = unsafe { folder_info(&folder, account_name, parent_name) };
        let folder_name = fi.name.clone();
        out.push(fi);
        let sub_arr = unsafe { obj_folders(&folder) };
        unsafe { collect_folders(&sub_arr, account_name, &folder_name, out) };
    }
}

/// Collect all `NoteInfo` entries from a folder's notes array.
pub(super) unsafe fn collect_notes_in_folder(
    folder: &AnyObject,
    folder_name: &str,
    account_name: &str,
    out: &mut Vec<NoteInfo>,
) {
    let notes_arr = unsafe { obj_notes(folder) };
    let count = unsafe { sb_count(&notes_arr) };
    for i in 0..count {
        let note = unsafe { sb_at(&notes_arr, i) };
        out.push(unsafe { note_info(&note, folder_name, account_name) });
    }
}

/// Collect all `NoteInfo` entries from all folders in a folder array (recursive).
pub(super) unsafe fn collect_notes_in_folders(
    folders_arr: &AnyObject,
    account_name: &str,
    out: &mut Vec<NoteInfo>,
) {
    let count = unsafe { sb_count(folders_arr) };
    for i in 0..count {
        let folder = unsafe { sb_at(folders_arr, i) };
        let folder_name = unsafe { kvc_string(&folder, "name") };
        unsafe { collect_notes_in_folder(&folder, &folder_name, account_name, out) };
        let sub_arr = unsafe { obj_folders(&folder) };
        unsafe { collect_notes_in_folders(&sub_arr, account_name, out) };
    }
}
