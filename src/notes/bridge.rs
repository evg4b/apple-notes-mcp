use super::helpers::{
    contains_ignore_case, keys, kvc_bool, kvc_bool_vec, kvc_index_of, kvc_string, kvc_string_vec,
    sb_at, sb_collection, sb_count, take_at,
};
use super::types::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::NSString;

pub use objc2_scripting_bridge::SBApplication;

pub(super) unsafe fn app_notes(app: &AnyObject) -> Retained<AnyObject> {
    unsafe { sb_collection(app, objc2::sel!(notes)) }
}

pub(super) unsafe fn app_accounts(app: &SBApplication) -> Retained<AnyObject> {
    unsafe { sb_collection(app.as_ref(), objc2::sel!(accounts)) }
}

pub(super) unsafe fn obj_notes(obj: &AnyObject) -> Retained<AnyObject> {
    unsafe { sb_collection(obj, objc2::sel!(notes)) }
}

pub(super) unsafe fn obj_folders(obj: &AnyObject) -> Retained<AnyObject> {
    unsafe { sb_collection(obj, objc2::sel!(folders)) }
}

pub(super) unsafe fn obj_attachments(obj: &AnyObject) -> Retained<AnyObject> {
    unsafe { sb_collection(obj, objc2::sel!(attachments)) }
}

pub(super) unsafe fn account_info(obj: &AnyObject) -> AccountInfo {
    AccountInfo {
        id: unsafe { kvc_string(obj, keys::id()) },
        name: unsafe { kvc_string(obj, keys::name()) },
    }
}

/// Build a [`NoteInfo`] from a single note proxy. The folder and account names
/// come from the caller because walking back up the containment chain would
/// cost extra Apple Events per note.
pub(super) unsafe fn note_info(obj: &AnyObject, folder_name: &str, account_name: &str) -> NoteInfo {
    NoteInfo {
        id: unsafe { kvc_string(obj, keys::id()) },
        title: unsafe { kvc_string(obj, keys::name()) },
        body: unsafe { kvc_string(obj, keys::body()) },
        creation_date: unsafe { kvc_string(obj, keys::creation_date()) },
        modification_date: unsafe { kvc_string(obj, keys::modification_date()) },
        folder: folder_name.to_owned(),
        account: account_name.to_owned(),
        shared: unsafe { kvc_bool(obj, keys::shared()) },
        password_protected: unsafe { kvc_bool(obj, keys::password_protected()) },
    }
}

/// Collect a folder array and, recursively, everything nested under it.
///
/// `id` and `name` are batch-fetched for the whole level (2 Apple Events per
/// level instead of 2 per folder); recursion still costs one `sb_at` plus one
/// `folders` fetch per folder.
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
    let mut ids = unsafe { kvc_string_vec(folders_arr, keys::id()) };
    let mut names = unsafe { kvc_string_vec(folders_arr, keys::name()) };

    out.reserve(count);
    for i in 0..count {
        let name = take_at(&mut names, i);
        out.push(FolderInfo {
            id: take_at(&mut ids, i),
            name: name.clone(),
            account: account_name.to_owned(),
            parent: parent_name.to_owned(),
        });
        let folder = unsafe { sb_at(folders_arr, i) };
        let sub_arr = unsafe { obj_folders(&folder) };
        unsafe { collect_folders(&sub_arr, account_name, &name, out) };
    }
}

/// Name of the top-level folder that iCloud accounts keep deleted notes in.
///
/// Notes exposes it as an ordinary folder, and its notes as ordinary notes, so
/// without this check a title lookup can land on a trashed copy, and deleting
/// that copy is permanent. The scripting dictionary has no property that marks
/// the folder, so it is recognised by name; Notes reports it in English.
const RECENTLY_DELETED: &str = "Recently Deleted";

/// Whether the folder named `name` at the given depth is Recently Deleted.
/// Only a top-level folder can be; a user folder nested somewhere with the
/// same name is left alone.
pub(super) fn is_trash(name: &str, top_level: bool) -> bool {
    top_level && name == RECENTLY_DELETED
}

/// Where a note sits: its folder's element array and its index in it, which
/// is enough to read it, write it, or remove it.
pub(super) struct NoteLocation {
    pub notes: Retained<AnyObject>,
    pub index: usize,
    pub folder_name: String,
}

impl NoteLocation {
    pub unsafe fn note(&self) -> Retained<AnyObject> {
        unsafe { sb_at(&self.notes, self.index) }
    }
}

/// Find the first note titled `target` in a folder array or anything nested
/// under it, skipping Recently Deleted. Costs one batched title fetch per
/// folder plus one name fetch per level.
pub(super) unsafe fn locate_note_in_folders(
    folders_arr: &AnyObject,
    target: &NSString,
    top_level: bool,
) -> Option<NoteLocation> {
    let count = unsafe { sb_count(folders_arr) };
    if count == 0 {
        return None;
    }
    let mut names = unsafe { kvc_string_vec(folders_arr, keys::name()) };
    for i in 0..count {
        let folder_name = take_at(&mut names, i);
        if is_trash(&folder_name, top_level) {
            continue;
        }
        let folder = unsafe { sb_at(folders_arr, i) };
        let notes_arr = unsafe { obj_notes(&folder) };
        if let Some(index) = unsafe { kvc_index_of(&notes_arr, keys::name(), target) } {
            return Some(NoteLocation {
                notes: notes_arr,
                index,
                folder_name,
            });
        }
        let sub_arr = unsafe { obj_folders(&folder) };
        if let Some(found) = unsafe { locate_note_in_folders(&sub_arr, target, false) } {
            return Some(found);
        }
    }
    None
}

/// Collect the title of every note in a folder array, recursing into
/// subfolders and skipping Recently Deleted. One batched fetch per folder.
pub(super) unsafe fn collect_titles_in_folders(
    folders_arr: &AnyObject,
    top_level: bool,
    out: &mut Vec<String>,
) {
    let count = unsafe { sb_count(folders_arr) };
    if count == 0 {
        return;
    }
    let names = unsafe { kvc_string_vec(folders_arr, keys::name()) };
    for i in 0..count {
        if names.get(i).is_some_and(|name| is_trash(name, top_level)) {
            continue;
        }
        let folder = unsafe { sb_at(folders_arr, i) };
        out.extend(unsafe { kvc_string_vec(&obj_notes(&folder), keys::name()) });
        let sub_arr = unsafe { obj_folders(&folder) };
        unsafe { collect_titles_in_folders(&sub_arr, false, out) };
    }
}

/// Collect every note of a folder, batch-fetching one property at a time.
///
/// Each `kvc_*_vec` call is a single "get every note's <property>" Apple Event,
/// so a folder of N notes costs 8 Apple Events rather than 8N. The per-note
/// values are moved out of the column vectors, never cloned — note bodies are
/// the largest strings in the payload.
pub(super) unsafe fn collect_notes_in_folder(
    folder: &AnyObject,
    folder_name: &str,
    account_name: &str,
    ceiling: usize,
    out: &mut Vec<NoteInfo>,
) {
    if out.len() >= ceiling {
        return;
    }
    let notes_arr = unsafe { obj_notes(folder) };
    let count = unsafe { sb_count(&notes_arr) };
    if count == 0 {
        return;
    }
    let mut ids = unsafe { kvc_string_vec(&notes_arr, keys::id()) };
    let mut names = unsafe { kvc_string_vec(&notes_arr, keys::name()) };
    let mut bodies = unsafe { kvc_string_vec(&notes_arr, keys::body()) };
    let mut created = unsafe { kvc_string_vec(&notes_arr, keys::creation_date()) };
    let mut modified = unsafe { kvc_string_vec(&notes_arr, keys::modification_date()) };
    let shared = unsafe { kvc_bool_vec(&notes_arr, keys::shared()) };
    let protected = unsafe { kvc_bool_vec(&notes_arr, keys::password_protected()) };

    out.reserve(count.min(ceiling - out.len()));
    for i in 0..count {
        if out.len() >= ceiling {
            return;
        }
        out.push(NoteInfo {
            id: take_at(&mut ids, i),
            title: take_at(&mut names, i),
            body: take_at(&mut bodies, i),
            creation_date: take_at(&mut created, i),
            modification_date: take_at(&mut modified, i),
            folder: folder_name.to_owned(),
            account: account_name.to_owned(),
            shared: shared.get(i).copied().unwrap_or_default(),
            password_protected: protected.get(i).copied().unwrap_or_default(),
        });
    }
}

/// Collect the notes of every folder in a folder array, recursing into
/// subfolders and skipping Recently Deleted.
pub(super) unsafe fn collect_notes_in_folders(
    folders_arr: &AnyObject,
    account_name: &str,
    top_level: bool,
    ceiling: usize,
    out: &mut Vec<NoteInfo>,
) {
    let count = unsafe { sb_count(folders_arr) };
    if count == 0 || out.len() >= ceiling {
        return;
    }
    let mut names = unsafe { kvc_string_vec(folders_arr, keys::name()) };
    for i in 0..count {
        if out.len() >= ceiling {
            return;
        }
        let folder_name = take_at(&mut names, i);
        if is_trash(&folder_name, top_level) {
            continue;
        }
        let folder = unsafe { sb_at(folders_arr, i) };
        unsafe { collect_notes_in_folder(&folder, &folder_name, account_name, ceiling, out) };
        let sub_arr = unsafe { obj_folders(&folder) };
        unsafe { collect_notes_in_folders(&sub_arr, account_name, false, ceiling, out) };
    }
}

/// Collect every attachment of a note, one batched fetch per property.
pub(super) unsafe fn collect_attachments(
    note: &AnyObject,
    note_title: &str,
    out: &mut Vec<AttachmentInfo>,
) {
    let arr = unsafe { obj_attachments(note) };
    let count = unsafe { sb_count(&arr) };
    if count == 0 {
        return;
    }
    let mut ids = unsafe { kvc_string_vec(&arr, keys::id()) };
    let mut names = unsafe { kvc_string_vec(&arr, keys::name()) };
    let mut created = unsafe { kvc_string_vec(&arr, keys::creation_date()) };
    let mut modified = unsafe { kvc_string_vec(&arr, keys::modification_date()) };
    let mut urls = unsafe { kvc_string_vec(&arr, keys::url()) };

    out.reserve(count);
    for i in 0..count {
        out.push(AttachmentInfo {
            id: take_at(&mut ids, i),
            name: take_at(&mut names, i),
            creation_date: take_at(&mut created, i),
            modification_date: take_at(&mut modified, i),
            url: take_at(&mut urls, i),
            note_title: note_title.to_owned(),
        });
    }
}

/// Which note fields a search compares against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SearchFields {
    pub title: bool,
    pub body: bool,
}

/// Search a folder array recursively, appending matches until `limit` is hit.
/// Recently Deleted is skipped.
///
/// Titles (and plain-text bodies, when searched) are batch-fetched per folder.
/// Bodies are matched on Notes' own `plaintext`, not the HTML, so a query such
/// as "div" or "&" does not hit every note through its markup. The remaining
/// columns are only fetched for folders that actually contain a match, so
/// scanning a large library costs one or two Apple Events per folder.
pub(super) unsafe fn search_notes_in_folders(
    folders_arr: &AnyObject,
    account_name: &str,
    top_level: bool,
    query: &str,
    fields: SearchFields,
    limit: usize,
    out: &mut Vec<NoteInfo>,
) {
    let count = unsafe { sb_count(folders_arr) };
    if count == 0 {
        return;
    }
    let mut folder_names = unsafe { kvc_string_vec(folders_arr, keys::name()) };
    for i in 0..count {
        if out.len() >= limit {
            return;
        }
        let folder_name = take_at(&mut folder_names, i);
        if is_trash(&folder_name, top_level) {
            continue;
        }
        let folder = unsafe { sb_at(folders_arr, i) };
        unsafe {
            search_notes_in_folder(
                &folder,
                &folder_name,
                account_name,
                query,
                fields,
                limit,
                out,
            )
        };
        let sub_arr = unsafe { obj_folders(&folder) };
        unsafe {
            search_notes_in_folders(&sub_arr, account_name, false, query, fields, limit, out)
        };
    }
}

unsafe fn search_notes_in_folder(
    folder: &AnyObject,
    folder_name: &str,
    account_name: &str,
    query: &str,
    fields: SearchFields,
    limit: usize,
    out: &mut Vec<NoteInfo>,
) {
    let notes_arr = unsafe { obj_notes(folder) };
    let count = unsafe { sb_count(&notes_arr) };
    if count == 0 {
        return;
    }
    let mut names = unsafe { kvc_string_vec(&notes_arr, keys::name()) };
    let texts = if fields.body {
        unsafe { kvc_string_vec(&notes_arr, keys::plaintext()) }
    } else {
        Vec::new()
    };

    let matched: Vec<usize> = (0..count)
        .filter(|&i| {
            let title_hit =
                fields.title && names.get(i).is_some_and(|n| contains_ignore_case(n, query));
            let body_hit =
                fields.body && texts.get(i).is_some_and(|t| contains_ignore_case(t, query));
            title_hit || body_hit
        })
        .take(limit.saturating_sub(out.len()))
        .collect();
    if matched.is_empty() {
        return;
    }

    let mut ids = unsafe { kvc_string_vec(&notes_arr, keys::id()) };
    let mut bodies = unsafe { kvc_string_vec(&notes_arr, keys::body()) };
    let mut created = unsafe { kvc_string_vec(&notes_arr, keys::creation_date()) };
    let mut modified = unsafe { kvc_string_vec(&notes_arr, keys::modification_date()) };
    let shared = unsafe { kvc_bool_vec(&notes_arr, keys::shared()) };
    let protected = unsafe { kvc_bool_vec(&notes_arr, keys::password_protected()) };

    out.reserve(matched.len());
    for i in matched {
        out.push(NoteInfo {
            id: take_at(&mut ids, i),
            title: take_at(&mut names, i),
            body: take_at(&mut bodies, i),
            creation_date: take_at(&mut created, i),
            modification_date: take_at(&mut modified, i),
            folder: folder_name.to_owned(),
            account: account_name.to_owned(),
            shared: shared.get(i).copied().unwrap_or_default(),
            password_protected: protected.get(i).copied().unwrap_or_default(),
        });
    }
}
