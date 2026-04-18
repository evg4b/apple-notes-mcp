use crate::models::{
    AccountRequest, AccountsResponse, CreateNoteRequest, EmptyRequest, FolderRequest,
    FoldersResponse, NoteResponse, NoteTitlesResponse, NotesResponse, TitleRequest,
    UpdateNoteRequest, WriteResponse,
};
use crate::notes::NotesApp;
use clap::ValueEnum;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router, Json};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum Scope {
    Read,
    Write,
    Delete,
}

#[derive(Clone)]
pub struct AppleNotesMCP {
    app: Arc<NotesApp>,
    #[allow(unused)]
    scopes: Vec<Scope>,
}

impl AppleNotesMCP {
    pub fn new(app: NotesApp, scopes: Vec<Scope>) -> Self {
        Self {
            app: Arc::new(app),
            scopes,
        }
    }
}

#[tool_router(server_handler)]
impl AppleNotesMCP {
    #[tool(
        description = "Return the titles of every note. Fast: skips body content. \
                          Use this to discover what notes exist or to find a title before \
                          calling get_note."
    )]
    pub fn list_notes(
        &self,
        _p: Parameters<EmptyRequest>,
    ) -> Result<Json<NoteTitlesResponse>, String> {
        debug!(tool = "list_notes", "called");
        let titles = self
            .app
            .list_notes()
            .inspect_err(|e| warn!(error = %e, "list_notes failed"))
            .unwrap_or_default();
        info!(tool = "list_notes", count = titles.len(), "ok");
        Ok(Json(NoteTitlesResponse { titles }))
    }

    #[tool(
        description = "Return full metadata and HTML body for every note across all accounts. \
                          Slow on large libraries — use only when you need to read or search \
                          content of many notes at once. For a single note prefer get_note."
    )]
    pub fn get_all_notes(
        &self,
        _p: Parameters<EmptyRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        debug!(tool = "get_all_notes", "called");
        let notes = self
            .app
            .get_all_notes()
            .inspect_err(|e| warn!(error = %e, "get_all_notes failed"))
            .unwrap_or_default();
        info!(tool = "get_all_notes", count = notes.len(), "ok");
        Ok(Json(NotesResponse { notes }))
    }

    #[tool(
        description = "Return full metadata and HTML body for one note by exact title. \
                          Returns null when no note matches. Use list_notes first if the \
                          exact title is unknown."
    )]
    pub fn get_note(&self, p: Parameters<TitleRequest>) -> Result<Json<NoteResponse>, String> {
        debug!(tool = "get_note", "called");
        let note = self
            .app
            .get_note_by_title(&p.0.title)
            .inspect_err(|e| warn!(error = %e, "get_note failed"))
            .ok()
            .flatten();
        info!(tool = "get_note", found = note.is_some(), "ok");
        Ok(Json(NoteResponse { note }))
    }

    #[tool(
        description = "Return full metadata and HTML body for all notes in a folder, \
                          matched by exact folder name. Use list_folders first if the \
                          folder name is unknown."
    )]
    pub fn get_notes_in_folder(
        &self,
        p: Parameters<FolderRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        debug!(tool = "get_notes_in_folder", "called");
        let notes = self
            .app
            .get_notes_in_folder(&p.0.folder)
            .inspect_err(|e| warn!(error = %e, "get_notes_in_folder failed"))
            .unwrap_or_default();
        info!(tool = "get_notes_in_folder", count = notes.len(), "ok");
        Ok(Json(NotesResponse { notes }))
    }

    #[tool(
        description = "Return full metadata and HTML body for all notes in an account, \
                          matched by exact account name. Use list_accounts first if the \
                          account name is unknown."
    )]
    pub fn get_notes_in_account(
        &self,
        p: Parameters<AccountRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        debug!(tool = "get_notes_in_account", "called");
        let notes = self
            .app
            .get_notes_in_account(&p.0.account)
            .inspect_err(|e| warn!(error = %e, "get_notes_in_account failed"))
            .unwrap_or_default();
        info!(tool = "get_notes_in_account", count = notes.len(), "ok");
        Ok(Json(NotesResponse { notes }))
    }

    #[tool(
        description = "Return all folders and subfolders across every account, each with \
                          its account and parent name. Call this to discover folder names \
                          before using get_notes_in_folder or get_subfolders."
    )]
    pub fn list_folders(
        &self,
        _p: Parameters<EmptyRequest>,
    ) -> Result<Json<FoldersResponse>, String> {
        debug!(tool = "list_folders", "called");
        let folders = self
            .app
            .list_folders()
            .inspect_err(|e| warn!(error = %e, "list_folders failed"))
            .unwrap_or_default();
        info!(tool = "list_folders", count = folders.len(), "ok");
        Ok(Json(FoldersResponse { folders }))
    }

    #[tool(description = "Return all direct and nested subfolders of a folder, \
                          matched by exact folder name. Returns empty when the folder \
                          has no children or does not exist.")]
    pub fn get_subfolders(
        &self,
        p: Parameters<FolderRequest>,
    ) -> Result<Json<FoldersResponse>, String> {
        debug!(tool = "get_subfolders", "called");
        let folders = self
            .app
            .get_subfolders(&p.0.folder)
            .inspect_err(|e| warn!(error = %e, "get_subfolders failed"))
            .unwrap_or_default();
        info!(tool = "get_subfolders", count = folders.len(), "ok");
        Ok(Json(FoldersResponse { folders }))
    }

    #[tool(
        description = "Return all accounts configured in Apple Notes (iCloud, On My Mac, \
                          Exchange, …). Call this to discover account names before using \
                          get_notes_in_account."
    )]
    pub fn list_accounts(
        &self,
        _p: Parameters<EmptyRequest>,
    ) -> Result<Json<AccountsResponse>, String> {
        debug!(tool = "list_accounts", "called");
        let accounts = self
            .app
            .list_accounts()
            .inspect_err(|e| warn!(error = %e, "list_accounts failed"))
            .unwrap_or_default();
        info!(tool = "list_accounts", count = accounts.len(), "ok");
        Ok(Json(AccountsResponse { accounts }))
    }

    #[tool(description = "Create a new note in the default folder. \
                          content must be an HTML string, e.g. \"<b>Hello</b> world\". \
                          Use plain text wrapped in <div> tags if no formatting is needed.")]
    pub fn create_note(
        &self,
        p: Parameters<CreateNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        debug!(tool = "create_note", "called");
        let note = self
            .app
            .create_note(&p.0.title, &p.0.content)
            .inspect_err(|e| warn!(error = %e, "create_note failed"))
            .ok();
        let success = note.is_some();
        info!(tool = "create_note", success, "ok");
        Ok(Json(WriteResponse { success, note }))
    }

    #[tool(
        description = "Update the title and/or HTML body of an existing note by exact title. \
                          Omit new_title or new_content to leave that field unchanged. \
                          Returns success=false when no note with that title is found."
    )]
    pub fn update_note(
        &self,
        p: Parameters<UpdateNoteRequest>,
    ) -> Result<Json<WriteResponse>, String> {
        debug!(
            tool = "update_note",
            new_title = p.0.new_title.as_deref().unwrap_or("<unchanged>"),
            "called"
        );
        let note = self
            .app
            .update_note(
                &p.0.title,
                p.0.new_title.as_deref(),
                p.0.new_content.as_deref(),
            )
            .inspect_err(|e| warn!(error = %e, "update_note failed"))
            .ok()
            .flatten();
        let success = note.is_some();
        info!(tool = "update_note", success, "ok");
        Ok(Json(WriteResponse { success, note }))
    }

    #[tool(
        description = "Permanently delete a note by exact title. Cannot be undone. \
                          Returns success=false when no note with that title is found."
    )]
    pub fn delete_note(&self, p: Parameters<TitleRequest>) -> Result<Json<WriteResponse>, String> {
        debug!(tool = "delete_note", "called");
        let success = self
            .app
            .delete_note(&p.0.title)
            .inspect_err(|e| warn!(error = %e, "delete_note failed"))
            .unwrap_or(false);
        info!(tool = "delete_note", success, "ok");
        Ok(Json(WriteResponse {
            success,
            note: None,
        }))
    }
}
