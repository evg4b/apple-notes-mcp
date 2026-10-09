use super::helpers::{
    contains_ignore_case, keys, kvc_bool, kvc_bool_vec, kvc_index_of, kvc_string, kvc_string_vec,
    sb_at, sb_collection, sb_count, take_at,
};
use super::types::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::NSString;
use std::ops::ControlFlow;

pub use objc2_scripting_bridge::SBApplication;

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

/// Folder and account names come from the caller: walking back up the
/// containment chain would cost extra Apple Events per note.
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

/// Only a top-level folder can be the trash; a nested user folder with the same
/// name is left alone.
pub(super) fn is_trash(name: &str, top_level: bool) -> bool {
    top_level && name == RECENTLY_DELETED
}

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

/// Depth-first walk over a folder array and everything nested under it,
/// skipping Recently Deleted. Folder names are batch-fetched once per level.
unsafe fn walk_folders<B>(
    folders_arr: &AnyObject,
    top_level: bool,
    visit: &mut impl FnMut(&AnyObject, String) -> ControlFlow<B>,
) -> ControlFlow<B> {
    let count = unsafe { sb_count(folders_arr) };
    if count == 0 {
        return ControlFlow::Continue(());
    }
    let mut names = unsafe { kvc_string_vec(folders_arr, keys::name()) };
    for i in 0..count {
        let folder_name = take_at(&mut names, i);
        if is_trash(&folder_name, top_level) {
            continue;
        }
        let folder = unsafe { sb_at(folders_arr, i) };
        visit(&folder, folder_name)?;
        unsafe { walk_folders(&obj_folders(&folder), false, visit) }?;
    }
    ControlFlow::Continue(())
}

/// Costs one batched title fetch per folder plus one name fetch per level.
pub(super) unsafe fn locate_note_in_folders(
    folders_arr: &AnyObject,
    target: &NSString,
) -> Option<NoteLocation> {
    let found = unsafe {
        walk_folders(folders_arr, true, &mut |folder, folder_name| {
            let notes = obj_notes(folder);
            match kvc_index_of(&notes, keys::name(), target) {
                Some(index) => ControlFlow::Break(NoteLocation {
                    notes,
                    index,
                    folder_name,
                }),
                None => ControlFlow::Continue(()),
            }
        })
    };
    found.break_value()
}

pub(super) unsafe fn collect_titles_in_folders(folders_arr: &AnyObject, out: &mut Vec<String>) {
    let _ = unsafe {
        walk_folders::<()>(folders_arr, true, &mut |folder, _| {
            out.extend(kvc_string_vec(&obj_notes(folder), keys::name()));
            ControlFlow::Continue(())
        })
    };
}

/// Every per-note column but the title, one batched Apple Event each, so a
/// folder of N notes costs a fixed number of Apple Events rather than N times
/// as many.
struct NoteColumns {
    ids: Vec<String>,
    bodies: Vec<String>,
    created: Vec<String>,
    modified: Vec<String>,
    shared: Vec<bool>,
    protected: Vec<bool>,
}

impl NoteColumns {
    unsafe fn fetch(notes_arr: &AnyObject) -> Self {
        unsafe {
            Self {
                ids: kvc_string_vec(notes_arr, keys::id()),
                bodies: kvc_string_vec(notes_arr, keys::body()),
                created: kvc_string_vec(notes_arr, keys::creation_date()),
                modified: kvc_string_vec(notes_arr, keys::modification_date()),
                shared: kvc_bool_vec(notes_arr, keys::shared()),
                protected: kvc_bool_vec(notes_arr, keys::password_protected()),
            }
        }
    }

    fn take(&mut self, i: usize, title: String, folder: &str, account: &str) -> NoteInfo {
        NoteInfo {
            id: take_at(&mut self.ids, i),
            title,
            body: take_at(&mut self.bodies, i),
            creation_date: take_at(&mut self.created, i),
            modification_date: take_at(&mut self.modified, i),
            folder: folder.to_owned(),
            account: account.to_owned(),
            shared: self.shared.get(i).copied().unwrap_or_default(),
            password_protected: self.protected.get(i).copied().unwrap_or_default(),
        }
    }
}

pub(super) unsafe fn collect_notes_in_folder(
    folder: &AnyObject,
    folder_name: &str,
    account_name: &str,
    ceiling: usize,
    out: &mut Vec<NoteInfo>,
) {
    let room = ceiling.saturating_sub(out.len());
    if room == 0 {
        return;
    }
    let notes_arr = unsafe { obj_notes(folder) };
    let count = unsafe { sb_count(&notes_arr) }.min(room);
    if count == 0 {
        return;
    }
    let mut names = unsafe { kvc_string_vec(&notes_arr, keys::name()) };
    let mut columns = unsafe { NoteColumns::fetch(&notes_arr) };

    out.reserve(count);
    for i in 0..count {
        let title = take_at(&mut names, i);
        out.push(columns.take(i, title, folder_name, account_name));
    }
}

pub(super) unsafe fn collect_notes_in_folders(
    folders_arr: &AnyObject,
    account_name: &str,
    ceiling: usize,
    out: &mut Vec<NoteInfo>,
) {
    let _ = unsafe {
        walk_folders(folders_arr, true, &mut |folder, folder_name| {
            collect_notes_in_folder(folder, &folder_name, account_name, ceiling, out);
            stop_when_full(out, ceiling)
        })
    };
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SearchFields {
    pub title: bool,
    pub body: bool,
}

/// Titles (and plain-text bodies, when searched) are batch-fetched per folder.
/// Bodies are matched on Notes' own `plaintext`, not the HTML, so a query such
/// as "div" or "&" does not hit every note through its markup. The remaining
/// columns are only fetched for folders that actually contain a match, so
/// scanning a large library costs one or two Apple Events per folder.
pub(super) unsafe fn search_notes_in_folders(
    folders_arr: &AnyObject,
    account_name: &str,
    query: &str,
    fields: SearchFields,
    limit: usize,
    out: &mut Vec<NoteInfo>,
) {
    let _ = unsafe {
        walk_folders(folders_arr, true, &mut |folder, folder_name| {
            search_notes_in_folder(
                folder,
                &folder_name,
                account_name,
                query,
                fields,
                limit,
                out,
            );
            stop_when_full(out, limit)
        })
    };
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

    let mut columns = unsafe { NoteColumns::fetch(&notes_arr) };
    out.reserve(matched.len());
    for i in matched {
        let title = take_at(&mut names, i);
        out.push(columns.take(i, title, folder_name, account_name));
    }
}

fn stop_when_full<T>(out: &[T], ceiling: usize) -> ControlFlow<()> {
    if out.len() >= ceiling {
        ControlFlow::Break(())
    } else {
        ControlFlow::Continue(())
    }
}
