use super::AppleNotesMCP;
use super::models::{
    AccountRequest, AccountsResponse, AttachmentsResponse, EmptyRequest, FolderRequest,
    FoldersResponse, NoteResponse, NoteTitlesResponse, NotesResponse, SearchRequest, TitleRequest,
};
use anyhow::Result;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool};
use tracing::{info, warn};

/// Degrade a failed scripting call to an empty payload.
///
/// A missing Automation permission or a transient Apple Event failure should
/// read as "nothing found" to the client rather than aborting the tool call.
fn or_empty<T: Default>(tool: &'static str, result: Result<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            warn!(tool, %error, "read failed");
            T::default()
        }
    }
}

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
        let titles = or_empty("list_notes", self.app.list_notes());
        info!(tool = "list_notes", count = titles.len(), "ok");
        Ok(Json(NoteTitlesResponse { titles }))
    }

    #[tool(
        description = "Return full metadata and HTML body for every note across all accounts. \
                       Slow on large libraries — prefer search_notes to find content and \
                       get_note for a single note."
    )]
    pub fn get_all_notes(
        &self,
        _p: Parameters<EmptyRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        let notes = or_empty("get_all_notes", self.app.get_all_notes());
        info!(tool = "get_all_notes", count = notes.len(), "ok");
        Ok(Json(NotesResponse { notes }))
    }

    #[tool(
        description = "Return full metadata and HTML body for one note by exact title. \
                       Returns null when no note matches. Use search_notes or list_notes \
                       first if the exact title is unknown."
    )]
    pub fn get_note(&self, p: Parameters<TitleRequest>) -> Result<Json<NoteResponse>, String> {
        let note = or_empty("get_note", self.app.get_note_by_title(&p.0.title));
        info!(tool = "get_note", found = note.is_some(), "ok");
        Ok(Json(NoteResponse { note }))
    }

    #[tool(
        description = "Find notes whose title — and by default body — contains the query, \
                       compared case-insensitively. Returns at most `limit` notes \
                       (default 50). Prefer this over get_all_notes for content lookups."
    )]
    pub fn search_notes(
        &self,
        p: Parameters<SearchRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        let notes = or_empty(
            "search_notes",
            self.app
                .search_notes(&p.0.query, p.0.in_body(), p.0.limit()),
        );
        info!(tool = "search_notes", count = notes.len(), "ok");
        Ok(Json(NotesResponse { notes }))
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
        let notes = or_empty(
            "get_notes_in_folder",
            self.app.get_notes_in_folder(&p.0.folder),
        );
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
        let notes = or_empty(
            "get_notes_in_account",
            self.app.get_notes_in_account(&p.0.account),
        );
        info!(tool = "get_notes_in_account", count = notes.len(), "ok");
        Ok(Json(NotesResponse { notes }))
    }

    #[tool(
        description = "Return the files attached to a note, matched by exact title. \
                       Returns an empty list when the note has no attachments or does \
                       not exist."
    )]
    pub fn get_attachments(
        &self,
        p: Parameters<TitleRequest>,
    ) -> Result<Json<AttachmentsResponse>, String> {
        let attachments = or_empty("get_attachments", self.app.get_note_attachments(&p.0.title));
        info!(tool = "get_attachments", count = attachments.len(), "ok");
        Ok(Json(AttachmentsResponse { attachments }))
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
        let folders = or_empty("list_folders", self.app.list_folders());
        info!(tool = "list_folders", count = folders.len(), "ok");
        Ok(Json(FoldersResponse { folders }))
    }

    #[tool(
        description = "Return all direct and nested subfolders of a folder, matched by \
                       exact folder name. Returns empty when the folder has no children \
                       or does not exist."
    )]
    pub fn get_subfolders(
        &self,
        p: Parameters<FolderRequest>,
    ) -> Result<Json<FoldersResponse>, String> {
        let folders = or_empty("get_subfolders", self.app.get_subfolders(&p.0.folder));
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
        let accounts = or_empty("list_accounts", self.app.list_accounts());
        info!(tool = "list_accounts", count = accounts.len(), "ok");
        Ok(Json(AccountsResponse { accounts }))
    }
}
