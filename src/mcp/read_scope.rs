use super::AppleNotesMCP;
use super::models::{
    AccountNotesRequest, AccountsResponse, AttachmentsResponse, BulkNotesRequest, EmptyRequest,
    FolderNotesRequest, FolderRequest, FoldersResponse, GetNoteRequest, NoteResponse,
    NoteTitlesResponse, NotesResponse, SearchRequest, TitleRequest,
};
use anyhow::Result;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool};
use tracing::{info, warn};

/// Report a failed scripting call as a tool error.
///
/// An empty payload would tell the client "nothing matched", and it would act
/// on that — offer to create a note that already exists, say. An error keeps
/// "Notes could not be read" distinct from "there is nothing there".
fn read<T>(tool: &'static str, result: Result<T>) -> Result<T, String> {
    result.map_err(|error| {
        warn!(tool, %error, "read failed");
        error.to_string()
    })
}

fn notes_ok(tool: &'static str, response: NotesResponse) -> Result<Json<NotesResponse>, String> {
    info!(
        tool,
        count = response.notes.len(),
        truncated = response.truncated,
        "ok"
    );
    Ok(Json(response))
}

impl AppleNotesMCP {
    #[tool(
        description = "Titles of every note, no bodies. The cheapest way to see what \
                       exists; follow up with get_note."
    )]
    pub fn list_notes(
        &self,
        _p: Parameters<EmptyRequest>,
    ) -> Result<Json<NoteTitlesResponse>, String> {
        let titles = read("list_notes", self.app.list_notes())?;
        info!(tool = "list_notes", count = titles.len(), "ok");
        Ok(Json(NoteTitlesResponse { titles }))
    }

    #[tool(
        description = "One note by exact title, with its body. Returns null if nothing \
                       matches — search_notes first when unsure of the title."
    )]
    pub fn get_note(&self, p: Parameters<GetNoteRequest>) -> Result<Json<NoteResponse>, String> {
        let note = read("get_note", self.app.get_note_by_title(&p.0.title))?;
        info!(tool = "get_note", found = note.is_some(), "ok");
        Ok(Json(NoteResponse::new(
            note,
            p.0.format.unwrap_or_default(),
        )))
    }

    #[tool(
        description = "Notes whose title, or body unless in_body is false, contains the \
                       query. Case-insensitive. The right way to find notes by content."
    )]
    pub fn search_notes(
        &self,
        p: Parameters<SearchRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        let page = read(
            "search_notes",
            self.app
                .search_notes(&p.0.query, p.0.in_body(), p.0.body.limit()),
        )?;
        notes_ok("search_notes", NotesResponse::new(page, p.0.body.format()))
    }

    #[tool(
        description = "Every note with its body, account by account. Expensive — use \
                       search_notes to find content and get_note for one known title."
    )]
    pub fn get_all_notes(
        &self,
        p: Parameters<BulkNotesRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        let page = read("get_all_notes", self.app.get_all_notes(p.0.body.limit()))?;
        notes_ok("get_all_notes", NotesResponse::new(page, p.0.body.format()))
    }

    #[tool(
        description = "Notes in one folder, nested or not, by exact name. Subfolders' \
                          notes are not included. See list_folders."
    )]
    pub fn get_notes_in_folder(
        &self,
        p: Parameters<FolderNotesRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        let page = read(
            "get_notes_in_folder",
            self.app.get_notes_in_folder(&p.0.folder, p.0.body.limit()),
        )?;
        notes_ok(
            "get_notes_in_folder",
            NotesResponse::new(page, p.0.body.format()),
        )
    }

    #[tool(description = "Notes in one account, by exact account name. See list_accounts.")]
    pub fn get_notes_in_account(
        &self,
        p: Parameters<AccountNotesRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        let page = read(
            "get_notes_in_account",
            self.app
                .get_notes_in_account(&p.0.account, p.0.body.limit()),
        )?;
        notes_ok(
            "get_notes_in_account",
            NotesResponse::new(page, p.0.body.format()),
        )
    }

    #[tool(description = "Files attached to a note, by exact title. Empty if it has none.")]
    pub fn get_attachments(
        &self,
        p: Parameters<TitleRequest>,
    ) -> Result<Json<AttachmentsResponse>, String> {
        let attachments = read("get_attachments", self.app.get_note_attachments(&p.0.title))?;
        info!(tool = "get_attachments", count = attachments.len(), "ok");
        Ok(Json(AttachmentsResponse { attachments }))
    }

    #[tool(
        description = "Every folder and subfolder, with its account and parent. Call \
                       before any tool that takes a folder name."
    )]
    pub fn list_folders(
        &self,
        _p: Parameters<EmptyRequest>,
    ) -> Result<Json<FoldersResponse>, String> {
        let folders = read("list_folders", self.app.list_folders())?;
        info!(tool = "list_folders", count = folders.len(), "ok");
        Ok(Json(FoldersResponse { folders }))
    }

    #[tool(description = "Subfolders of one folder, nested ones included. Empty if none.")]
    pub fn get_subfolders(
        &self,
        p: Parameters<FolderRequest>,
    ) -> Result<Json<FoldersResponse>, String> {
        let folders = read("get_subfolders", self.app.get_subfolders(&p.0.folder))?;
        info!(tool = "get_subfolders", count = folders.len(), "ok");
        Ok(Json(FoldersResponse { folders }))
    }

    #[tool(description = "Configured accounts: iCloud, On My Mac, Exchange, and so on.")]
    pub fn list_accounts(
        &self,
        _p: Parameters<EmptyRequest>,
    ) -> Result<Json<AccountsResponse>, String> {
        let accounts = read("list_accounts", self.app.list_accounts())?;
        info!(tool = "list_accounts", count = accounts.len(), "ok");
        Ok(Json(AccountsResponse { accounts }))
    }
}
