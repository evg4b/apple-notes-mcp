use super::NotesApp;
use crate::notes::bridge::{
    SearchFields, app_accounts, collect_attachments, collect_folders, collect_notes_in_folder,
    collect_notes_in_folders, collect_titles_in_folders, note_info, obj_folders,
    search_notes_in_folders,
};
use crate::notes::helpers::{keys, kvc_string_vec, take_at};
use crate::notes::types::{AccountInfo, AttachmentInfo, FolderInfo, NoteInfo, NotePage};
use anyhow::Result;
use tracing::{debug, instrument};

impl NotesApp {
    #[instrument(skip(self))]
    pub fn list_accounts(&self) -> Result<Vec<AccountInfo>> {
        self.run(|| unsafe {
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
        })
    }

    #[instrument(skip(self))]
    pub fn list_folders(&self) -> Result<Vec<FolderInfo>> {
        self.run(|| unsafe {
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
        })
    }

    #[instrument(skip(self))]
    pub fn get_subfolders(&self, folder_name: &str) -> Result<Vec<FolderInfo>> {
        self.run(|| unsafe {
            let mut out = Vec::new();
            if let Some(found) = self.find_folder(folder_name, None) {
                let sub_arr = obj_folders(&found.folder());
                collect_folders(&sub_arr, &found.account, &folder_name.into(), &mut out);
            }
            debug!(count = out.len(), "listed subfolders");
            Ok(out)
        })
    }

    /// Walked folder by folder rather than read off the application's flat
    /// `notes`, which includes trashed notes: a title listed here must be one
    /// `get_note` can find.
    #[instrument(skip(self))]
    pub fn list_notes(&self) -> Result<Vec<String>> {
        self.run(|| unsafe {
            let mut names = Vec::new();
            for (account, _) in self.accounts() {
                collect_titles_in_folders(&obj_folders(&account), &mut names);
            }
            debug!(count = names.len(), "listed note titles");
            Ok(names)
        })
    }

    #[instrument(skip(self))]
    pub fn get_all_notes(&self, limit: usize) -> Result<NotePage> {
        self.run(|| {
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
        })
    }

    #[instrument(skip(self))]
    pub fn get_note_by_title(&self, title: &str) -> Result<Option<NoteInfo>> {
        self.run(|| unsafe {
            let found = self.find_note(title).map(|found| {
                note_info(
                    &found.location.note(),
                    &found.location.folder_name,
                    &found.account,
                )
            });
            debug!(found = found.is_some(), "note lookup");
            Ok(found)
        })
    }

    #[instrument(skip(self))]
    pub fn get_notes_in_folder(&self, folder_name: &str, limit: usize) -> Result<NotePage> {
        self.run(|| {
            let page = NotePage::collect(limit, |ceiling, out| unsafe {
                if let Some(found) = self.find_folder(folder_name, None) {
                    let folder = found.folder();
                    collect_notes_in_folder(
                        &folder,
                        &folder_name.into(),
                        &found.account,
                        ceiling,
                        out,
                    );
                }
            });
            debug!(count = page.notes.len(), "collected notes in folder");
            Ok(page)
        })
    }

    #[instrument(skip(self))]
    pub fn get_notes_in_account(&self, account_name: &str, limit: usize) -> Result<NotePage> {
        self.run(|| {
            let Some(account) = (unsafe { self.find_account(account_name) }) else {
                debug!("account not found");
                return Ok(NotePage::default());
            };
            let page = NotePage::collect(limit, |ceiling, out| unsafe {
                collect_notes_in_folders(
                    &obj_folders(&account),
                    &account_name.into(),
                    ceiling,
                    out,
                );
            });
            debug!(count = page.notes.len(), "collected notes in account");
            Ok(page)
        })
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
        self.run(|| {
            let page = NotePage::collect(limit, |ceiling, out| unsafe {
                for (account, account_name) in self.accounts() {
                    if out.len() >= ceiling {
                        break;
                    }
                    let folders = obj_folders(&account);
                    search_notes_in_folders(&folders, &account_name, &needle, fields, ceiling, out);
                }
            });
            debug!(matches = page.notes.len(), "search complete");
            Ok(page)
        })
    }

    #[instrument(skip(self))]
    pub fn get_note_attachments(&self, title: &str) -> Result<Vec<AttachmentInfo>> {
        self.run(|| unsafe {
            let Some(found) = self.find_note(title) else {
                debug!("note not found");
                return Ok(Vec::new());
            };
            let mut out = Vec::new();
            collect_attachments(&found.location.note(), &title.into(), &mut out);
            debug!(count = out.len(), "collected attachments");
            Ok(out)
        })
    }
}
